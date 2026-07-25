//! metadata から executable test profile と test case を生成する production logic。

use lsharp_syntax::ast::{Decl, Expr, Program, TypeExpr};
use lsharp_syntax::metadata::{MetadataFormKind, PropertyForm};

/// メタデータから生成されたテストケース
#[derive(Debug, Clone)]
pub struct GeneratedTest {
    /// テスト名
    pub name: String,
    /// テスト対象の関数名
    pub function_name: String,
    /// テスト種別
    pub kind: TestKind,
    /// テスト式（AST）
    pub expr: Expr,
    /// canonical `:case` の期待値。それ以外は `None`。
    pub expected: Option<Expr>,
    /// 移行期の deterministic property smoke profile。通常の test では `None`。
    pub property: Option<PropertySmokeTestSpec>,
}

/// Rust/selfhost が段階的に共有する deterministic property smoke profile。
#[derive(Debug, Clone, PartialEq)]
pub struct PropertySmokeTestSpec {
    pub binder_names: Vec<String>,
    pub binder_types: Vec<PropertyBinderType>,
    pub cases: usize,
    /// 移行期に評価する precondition。source order の conjunction として評価する。
    pub preconditions: Vec<Expr>,
}

/// 移行期 property runner が実値を生成できる binder type。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyBinderType {
    Int,
    Bool,
    String,
}

const DETERMINISTIC_TYPED_BINDER_LIMIT: usize = 8;

/// `:property` のうち、移行期 runner が実行できる narrow profile を返す。
///
/// type-directed sampling、seed、shrink は別 slice のため、ここで暗黙に既定値へ
/// 丸めず、明示的に profile 外として扱う。1〜2 個の Int は legacy prefix、3〜8 個の
/// Int/Bool は cases 1〜2 の typed prefix、単一の String は cases 1〜5 の typed prefix
/// とする。precondition は source order の conjunction として deterministic sample の
/// filter に使う。
pub fn property_smoke_test_spec(property: &PropertyForm) -> Option<PropertySmokeTestSpec> {
    if !(1..=DETERMINISTIC_TYPED_BINDER_LIMIT).contains(&property.binders().len())
        || property.seed().is_some()
        || property.shrink().is_some()
    {
        return None;
    }
    let cases = property.cases()?;
    if !(1..=5).contains(&cases) {
        return None;
    }
    let binder_types = property
        .binders()
        .iter()
        .map(|binder| match binder.ty() {
            TypeExpr::Named(_, ty_name) if ty_name == "Int" => Some(PropertyBinderType::Int),
            TypeExpr::Named(_, ty_name) if ty_name == "Bool" => Some(PropertyBinderType::Bool),
            TypeExpr::Named(_, ty_name) if ty_name == "String" => Some(PropertyBinderType::String),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    match binder_types.as_slice() {
        [PropertyBinderType::Int] | [PropertyBinderType::Int, PropertyBinderType::Int] => {}
        [PropertyBinderType::String] => {}
        [PropertyBinderType::Bool]
        | [PropertyBinderType::Bool, PropertyBinderType::Bool]
        | [PropertyBinderType::Int, PropertyBinderType::Bool]
        | [PropertyBinderType::Bool, PropertyBinderType::Int]
            if cases <= 2 => {}
        _ if binder_types.len() >= 3 && cases <= 2 => {}
        _ => return None,
    }
    Some(PropertySmokeTestSpec {
        binder_names: property
            .binders()
            .iter()
            .map(|binder| binder.name().to_string())
            .collect(),
        binder_types,
        cases,
        preconditions: property.preconditions().to_vec(),
    })
}

/// テストの種別
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestKind {
    /// canonical `:case` から生成されたテスト
    Case,
    /// canonical `:assert` から生成されたテスト
    Assertion,
    /// :invariant から生成されたテスト
    Invariant,
    /// 移行期 deterministic property smoke profile。
    Property,
    /// :example から生成されたテスト
    Example,
}

/// プログラムのメタデータからテストケースを自動生成
///
/// :invariant と :example から検証用テストケースを生成する。
/// 生成されたテストは、コンパイル・実行パイプラインで検証可能。
pub fn generate_tests(program: &Program) -> Vec<GeneratedTest> {
    let mut tests = Vec::new();

    for decl in &program.decls {
        let actual_decl = match decl {
            Decl::Private { inner, .. } => inner.as_ref(),
            other => other,
        };

        if let Decl::Defn {
            name,
            metadata: Some(meta),
            ..
        } = actual_decl
        {
            // :invariant からテスト生成
            if let Some(ref invariant_expr) = meta.invariant {
                tests.push(GeneratedTest {
                    name: format!("{name}_invariant"),
                    function_name: name.clone(),
                    kind: TestKind::Invariant,
                    expr: invariant_expr.clone(),
                    expected: None,
                    property: None,
                });
            }

            // canonical :case からテスト生成
            let mut case_index = 0;
            for form in &meta.forms {
                let MetadataFormKind::Case { expectations } = &form.kind else {
                    continue;
                };
                for expectation in expectations {
                    tests.push(GeneratedTest {
                        name: format!("{name}_case_{case_index}"),
                        function_name: name.clone(),
                        kind: TestKind::Case,
                        expr: expectation.actual().clone(),
                        expected: Some(expectation.expected().clone()),
                        property: None,
                    });
                    case_index += 1;
                }
            }

            // canonical :assert からテスト生成
            let mut assertion_index = 0;
            for form in &meta.forms {
                let MetadataFormKind::Assertion { predicates } = &form.kind else {
                    continue;
                };
                for predicate in predicates {
                    tests.push(GeneratedTest {
                        name: format!("{name}_assertion_{assertion_index}"),
                        function_name: name.clone(),
                        kind: TestKind::Assertion,
                        expr: predicate.clone(),
                        expected: None,
                        property: None,
                    });
                    assertion_index += 1;
                }
            }

            // 移行期 deterministic property smoke profile からテスト生成
            let mut property_index = 0;
            for form in &meta.forms {
                let MetadataFormKind::Property { properties } = &form.kind else {
                    continue;
                };
                for property in properties {
                    let Some(spec) = property_smoke_test_spec(property) else {
                        continue;
                    };
                    tests.push(GeneratedTest {
                        name: format!("{name}_property_{property_index}"),
                        function_name: name.clone(),
                        kind: TestKind::Property,
                        expr: property.postcondition().clone(),
                        expected: None,
                        property: Some(spec),
                    });
                    property_index += 1;
                }
            }

            // :example からテスト生成
            for (i, example_expr) in meta.example.iter().enumerate() {
                tests.push(GeneratedTest {
                    name: format!("{name}_example_{i}"),
                    function_name: name.clone(),
                    kind: TestKind::Example,
                    expr: example_expr.clone(),
                    expected: None,
                    property: None,
                });
            }
        }
    }

    tests
}
