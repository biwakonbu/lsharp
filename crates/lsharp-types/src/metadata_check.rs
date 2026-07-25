//! メタデータ機械的検証エンジン
//!
//! 構造化メタデータ（:doc, :params, :returns 等）の整合性を検証する。
//! エラー（不整合）と警告（推奨）を区別して報告する。

use crate::canonical_contract_check::{
    check_assertion_non_vacuity, check_assertion_types, check_case_non_vacuity, check_case_types,
    check_property_non_vacuity, check_property_types,
};
use crate::infer::Infer;
use crate::metadata_contract::inventory_contract_suites;
use crate::types::Type;
use lsharp_syntax::ast::{Decl, Expr, Metadata, Pattern, Program, TypeExpr};
use lsharp_syntax::metadata::{MetadataFormKind, PropertyForm};
use lsharp_syntax::span::Span;

mod references;
use references::{
    collect_scoped_var_references, collect_var_references, extract_doc_identifiers, is_builtin,
    span_contains,
};

/// メタデータ検証結果の重大度
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    /// 不整合（修正必須）
    Error,
    /// 推奨事項（修正任意）
    Warning,
}

/// メタデータ検証の診断メッセージ
#[derive(Debug, Clone)]
pub struct MetadataDiagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    pub function_name: String,
}

impl std::fmt::Display for MetadataDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(
            f,
            "[{level}] {}: {} ({})",
            self.function_name, self.message, self.span
        )
    }
}

/// プログラム全体のメタデータを検証
pub fn check_metadata(program: &Program) -> Vec<MetadataDiagnostic> {
    let mut diagnostics = Vec::new();

    // 全関数名を収集（:see-also の参照先チェック用）
    let all_names: Vec<String> = program
        .decls
        .iter()
        .filter_map(|d| match d {
            Decl::Defn { name, .. } => Some(name.clone()),
            Decl::TypeDef { name, .. } => Some(name.clone()),
            Decl::RecordDef { name, .. } => Some(name.clone()),
            Decl::TypeAlias { name, .. } => Some(name.clone()),
            Decl::TraitDef { name, .. } => Some(name.clone()),
            Decl::Private { inner, .. } => match inner.as_ref() {
                Decl::Defn { name, .. }
                | Decl::TypeDef { name, .. }
                | Decl::RecordDef { name, .. }
                | Decl::TypeAlias { name, .. }
                | Decl::TraitDef { name, .. } => Some(name.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();

    for decl in &program.decls {
        // Private 内の defn も展開して検証
        let actual_decl = match decl {
            Decl::Private { inner, .. } => inner.as_ref(),
            other => other,
        };

        if let Decl::Defn {
            name,
            params,
            metadata,
            span,
            ..
        } = actual_decl
            && let Some(meta) = metadata
        {
            check_defn_metadata(&mut diagnostics, name, params, meta, *span, &all_names);
        }
    }

    diagnostics.extend(check_legacy_invariant_types(program, &all_names));
    diagnostics.extend(check_assertion_non_vacuity(program));
    diagnostics.extend(check_case_non_vacuity(program));
    diagnostics.extend(check_property_non_vacuity(program));
    if let Ok(suites) = inventory_contract_suites(program) {
        diagnostics.extend(check_assertion_types(program, &suites));
        diagnostics.extend(check_case_types(program, &suites));
        diagnostics.extend(check_property_types(program, &suites));
    }

    diagnostics
}

struct LegacyInvariantProbe {
    name: String,
    owner: String,
    span: Span,
}

/// legacy `:invariant` を実際の関数戻り値の scope で Bool として検査する。
///
/// `result` は元関数を同じ引数で呼び出した値に束縛する synthetic probe を使う。
/// これにより、元関数の推論済み戻り値型を保ったまま metadata 式だけを検査できる。
fn check_legacy_invariant_types(
    program: &Program,
    all_names: &[String],
) -> Vec<MetadataDiagnostic> {
    let mut check_program = program.clone();
    let mut probes = Vec::new();

    for decl in &program.decls {
        let actual_decl = match decl {
            Decl::Private { inner, .. } => inner.as_ref(),
            other => other,
        };
        let Decl::Defn {
            name,
            params,
            metadata: Some(metadata),
            ..
        } = actual_decl
        else {
            continue;
        };
        let Some(invariant) = metadata.invariant.as_ref() else {
            continue;
        };

        let param_names: Vec<&str> = params.iter().map(|param| param.name.as_str()).collect();
        let has_unknown_reference =
            collect_scoped_var_references(invariant)
                .iter()
                .any(|(ref_name, _)| {
                    !is_builtin(ref_name)
                        && ref_name != "result"
                        && !param_names.contains(&ref_name.as_str())
                        && !all_names.contains(ref_name)
                });
        if has_unknown_reference {
            continue;
        }

        let span = invariant.span();
        let call_args = params
            .iter()
            .map(|param| Expr::Var(param.span, param.name.clone()))
            .collect();
        let result_call = Expr::App(span, Box::new(Expr::Var(span, name.clone())), call_args);
        let probe_body = Expr::Let(
            span,
            vec![(Pattern::Var(span, "result".to_string()), result_call)],
            Box::new(invariant.clone()),
        );
        let probe_name = format!("__lsharp_legacy_invariant_{}", probes.len());
        check_program.decls.push(Decl::Defn {
            span,
            name: probe_name.clone(),
            params: params.clone(),
            return_ty: None,
            body: probe_body,
            where_clauses: Vec::new(),
            metadata: None,
        });
        probes.push(LegacyInvariantProbe {
            name: probe_name,
            owner: name.clone(),
            span,
        });
    }

    if probes.is_empty() {
        return Vec::new();
    }

    let mut infer = Infer::new();
    match infer.infer_program(&check_program) {
        Ok(results) => {
            let bool_type = Type::bool();
            probes
                .iter()
                .filter_map(|probe| {
                    let (_, scheme) = results.iter().find(|(name, _)| name == &probe.name)?;
                    let Type::Fun(_, return_type) = &scheme.ty else {
                        return None;
                    };
                    (return_type.as_ref() != &bool_type).then(|| MetadataDiagnostic {
                        severity: Severity::Error,
                        message: format!(
                            ":invariant は Bool 必須ですが、{} が推論されました",
                            return_type
                        ),
                        span: probe.span,
                        function_name: probe.owner.clone(),
                    })
                })
                .collect()
        }
        Err(error) => {
            let Some(error_span) = error.span() else {
                return Vec::new();
            };
            let Some(probe) = probes
                .iter()
                .find(|probe| span_contains(probe.span, error_span))
            else {
                return Vec::new();
            };
            vec![MetadataDiagnostic {
                severity: Severity::Error,
                message: format!(":invariant の型推論に失敗しました: {error}"),
                span: error_span,
                function_name: probe.owner.clone(),
            }]
        }
    }
}

/// 関数定義のメタデータを検証
fn check_defn_metadata(
    diagnostics: &mut Vec<MetadataDiagnostic>,
    name: &str,
    params: &[lsharp_syntax::ast::Param],
    metadata: &Metadata,
    span: Span,
    all_names: &[String],
) {
    let param_names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();

    // :params の検証
    if !metadata.params.is_empty() {
        // P3-3-1: :params キーと引数リストの一致チェック（エラー）
        for (meta_param, _desc) in &metadata.params {
            if !param_names.contains(&meta_param.as_str()) {
                diagnostics.push(MetadataDiagnostic {
                    severity: Severity::Error,
                    message: format!(":params に存在しない引数 '{meta_param}' が記載されています"),
                    span,
                    function_name: name.to_string(),
                });
            }
        }

        // P3-3-2: :params の全引数網羅チェック（警告）
        let meta_param_names: Vec<&str> = metadata.params.iter().map(|(n, _)| n.as_str()).collect();
        for param_name in &param_names {
            if !meta_param_names.contains(param_name) {
                diagnostics.push(MetadataDiagnostic {
                    severity: Severity::Warning,
                    message: format!("引数 '{param_name}' が :params に記載されていません"),
                    span,
                    function_name: name.to_string(),
                });
            }
        }
    }

    // P3-3-3: :see-also 参照先の存在チェック（エラー）
    for ref_name in &metadata.see_also {
        if !all_names.contains(ref_name) {
            diagnostics.push(MetadataDiagnostic {
                severity: Severity::Error,
                message: format!(":see-also に存在しない識別子 '{ref_name}' が参照されています"),
                span,
                function_name: name.to_string(),
            });
        }
    }

    // P3-3-4: :doc 内のバッククォート識別子存在チェック（警告）
    if let Some(ref doc) = metadata.doc {
        let doc_idents = extract_doc_identifiers(doc);
        for ident in &doc_idents {
            // 引数名、関数名、型名のいずれにも存在しない場合は警告
            if !param_names.contains(&ident.as_str()) && !all_names.contains(ident) {
                diagnostics.push(MetadataDiagnostic {
                    severity: Severity::Warning,
                    message: format!(":doc 内の識別子 `{ident}` がプログラム中に見つかりません"),
                    span,
                    function_name: name.to_string(),
                });
            }
        }
    }

    // P3-3-5: :invariant の検証
    if let Some(ref invariant_expr) = metadata.invariant {
        check_invariant(diagnostics, name, &param_names, invariant_expr, all_names);
    }

    // P3-3-6: :example の検証
    for example_expr in &metadata.example {
        check_example(
            diagnostics,
            name,
            &param_names,
            example_expr,
            span,
            all_names,
        );
    }
}

/// :invariant 式の構造検証
///
/// - 不変条件式内で参照されている変数が、関数の引数または既知の関数名であることを確認
/// - 不変条件式が空でないことを確認
fn check_invariant(
    diagnostics: &mut Vec<MetadataDiagnostic>,
    fn_name: &str,
    param_names: &[&str],
    invariant: &Expr,
    all_names: &[String],
) {
    let var_refs = collect_scoped_var_references(invariant);

    for (ref_name, ref_span) in &var_refs {
        // 組み込み演算子・関数はスキップ
        if is_builtin(ref_name) {
            continue;
        }
        // 「result」は暗黙の戻り値参照として許可
        if ref_name == "result" {
            continue;
        }
        // 引数名または既知の関数/型名にない場合はエラー
        if !param_names.contains(&ref_name.as_str()) && !all_names.contains(ref_name) {
            diagnostics.push(MetadataDiagnostic {
                severity: Severity::Error,
                message: format!(":invariant 内で未定義の識別子 '{ref_name}' が参照されています"),
                span: *ref_span,
                function_name: fn_name.to_string(),
            });
        }
    }
}

/// :example 式の構造検証
///
/// - 例示式内で参照されている変数が、関数の引数または既知の関数名であることを確認
fn check_example(
    diagnostics: &mut Vec<MetadataDiagnostic>,
    fn_name: &str,
    param_names: &[&str],
    example: &Expr,
    span: Span,
    all_names: &[String],
) {
    let var_refs = collect_var_references(example);

    for (ref_name, _ref_span) in &var_refs {
        // 組み込み演算子・関数はスキップ
        if is_builtin(ref_name) {
            continue;
        }
        // 引数名または既知の関数/型名にない場合はエラー
        if !param_names.contains(&ref_name.as_str()) && !all_names.contains(ref_name) {
            diagnostics.push(MetadataDiagnostic {
                severity: Severity::Error,
                message: format!(":example 内で未定義の識別子 '{ref_name}' が参照されています"),
                span,
                function_name: fn_name.to_string(),
            });
        }
    }
}

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

#[cfg(test)]
mod test_generation_tests;
#[cfg(test)]
mod tests;
