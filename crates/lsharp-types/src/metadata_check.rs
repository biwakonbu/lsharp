//! メタデータ機械的検証エンジン
//!
//! 構造化メタデータ（:doc, :params, :returns 等）の整合性を検証する。
//! エラー（不整合）と警告（推奨）を区別して報告する。

use crate::canonical_contract_check::{
    check_assertion_non_vacuity, check_assertion_types, check_case_non_vacuity, check_case_types,
    check_property_non_vacuity, check_property_types,
};
use crate::metadata_contract::inventory_contract_suites;
use lsharp_syntax::ast::{ComputationStep, Decl, Expr, Metadata, Pattern, Program, TypeExpr};
use lsharp_syntax::metadata::{MetadataFormKind, PropertyForm};
use lsharp_syntax::span::Span;

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

/// :doc 文字列からバッククォート内の識別子を抽出
fn extract_doc_identifiers(doc: &str) -> Vec<String> {
    let mut identifiers = Vec::new();
    let mut chars = doc.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '`' {
            let mut ident = String::new();
            for c in chars.by_ref() {
                if c == '`' {
                    break;
                }
                ident.push(c);
            }
            if !ident.is_empty() {
                identifiers.push(ident);
            }
        }
    }

    identifiers
}

/// 式から参照されている変数名を再帰的に収集
fn collect_var_references(expr: &Expr) -> Vec<(String, Span)> {
    let mut refs = Vec::new();
    collect_var_references_inner(expr, &mut refs);
    refs
}

/// 式から変数参照を再帰的に収集（内部実装）
fn collect_var_references_inner(expr: &Expr, refs: &mut Vec<(String, Span)>) {
    match expr {
        Expr::Var(span, name) => {
            refs.push((name.clone(), *span));
        }
        Expr::Lit(_, _) => {}
        Expr::If(_, cond, then_branch, else_branch) => {
            collect_var_references_inner(cond, refs);
            collect_var_references_inner(then_branch, refs);
            collect_var_references_inner(else_branch, refs);
        }
        Expr::Let(_, bindings, body) => {
            for (_, expr) in bindings {
                collect_var_references_inner(expr, refs);
            }
            collect_var_references_inner(body, refs);
        }
        Expr::Lambda(_, _, body) => {
            collect_var_references_inner(body, refs);
        }
        Expr::App(_, func, args) => {
            collect_var_references_inner(func, refs);
            for arg in args {
                collect_var_references_inner(arg, refs);
            }
        }
        Expr::Match(_, scrutinee, arms) => {
            collect_var_references_inner(scrutinee, refs);
            for arm in arms {
                collect_var_references_inner(&arm.body, refs);
            }
        }
        Expr::Do(_, exprs) => {
            for e in exprs {
                collect_var_references_inner(e, refs);
            }
        }
        Expr::Ann(_, inner, _) => {
            collect_var_references_inner(inner, refs);
        }
        Expr::RecordLit(_, _, fields) => {
            for (_, e) in fields {
                collect_var_references_inner(e, refs);
            }
        }
        Expr::FieldAccess(_, inner, _) => {
            collect_var_references_inner(inner, refs);
        }
        Expr::RecordUpdate(_, base, fields) => {
            collect_var_references_inner(base, refs);
            for (_, e) in fields {
                collect_var_references_inner(e, refs);
            }
        }
        Expr::Computation(_, _, steps) => {
            for step in steps {
                match step {
                    ComputationStep::LetBang(_, _, expr) => {
                        collect_var_references_inner(expr, refs)
                    }
                    ComputationStep::DoBang(_, expr) => collect_var_references_inner(expr, refs),
                    ComputationStep::Return(_, expr) => collect_var_references_inner(expr, refs),
                    ComputationStep::Expr(expr) => collect_var_references_inner(expr, refs),
                }
            }
        }
        // P10-1: Quote/Unquote/UnquoteSplice -- 内部式の変数参照を再帰的に収集
        Expr::Quote(_, inner) | Expr::Unquote(_, inner) | Expr::UnquoteSplice(_, inner) => {
            collect_var_references_inner(inner, refs);
        }
    }
}

/// lexical scope を考慮して、式の自由変数参照だけを収集する。
fn collect_scoped_var_references(expr: &Expr) -> Vec<(String, Span)> {
    let mut refs = Vec::new();
    let mut scope = Vec::new();
    collect_scoped_var_references_inner(expr, &mut scope, &mut refs);
    refs
}

fn collect_scoped_var_references_inner(
    expr: &Expr,
    scope: &mut Vec<String>,
    refs: &mut Vec<(String, Span)>,
) {
    match expr {
        Expr::Var(span, name) => {
            if !scope.iter().any(|bound| bound == name) {
                refs.push((name.clone(), *span));
            }
        }
        Expr::Lit(_, _) => {}
        Expr::If(_, cond, then_branch, else_branch) => {
            collect_scoped_var_references_inner(cond, scope, refs);
            collect_scoped_var_references_inner(then_branch, scope, refs);
            collect_scoped_var_references_inner(else_branch, scope, refs);
        }
        Expr::Let(_, bindings, body) => {
            let scope_start = scope.len();
            for (pattern, value) in bindings {
                collect_scoped_var_references_inner(value, scope, refs);
                collect_pattern_bindings(pattern, scope);
            }
            collect_scoped_var_references_inner(body, scope, refs);
            scope.truncate(scope_start);
        }
        Expr::Lambda(_, params, body) => {
            let scope_start = scope.len();
            scope.extend(params.iter().map(|param| param.name.clone()));
            collect_scoped_var_references_inner(body, scope, refs);
            scope.truncate(scope_start);
        }
        Expr::App(_, func, args) => {
            collect_scoped_var_references_inner(func, scope, refs);
            for arg in args {
                collect_scoped_var_references_inner(arg, scope, refs);
            }
        }
        Expr::Match(_, scrutinee, arms) => {
            collect_scoped_var_references_inner(scrutinee, scope, refs);
            for arm in arms {
                let scope_start = scope.len();
                collect_pattern_bindings(&arm.pattern, scope);
                if let Some(guard) = &arm.guard {
                    collect_scoped_var_references_inner(guard, scope, refs);
                }
                collect_scoped_var_references_inner(&arm.body, scope, refs);
                scope.truncate(scope_start);
            }
        }
        Expr::Do(_, exprs) => {
            for expr in exprs {
                collect_scoped_var_references_inner(expr, scope, refs);
            }
        }
        Expr::Ann(_, inner, _) => collect_scoped_var_references_inner(inner, scope, refs),
        Expr::RecordLit(_, _, fields) => {
            for (_, value) in fields {
                collect_scoped_var_references_inner(value, scope, refs);
            }
        }
        Expr::FieldAccess(_, inner, _) => {
            collect_scoped_var_references_inner(inner, scope, refs);
        }
        Expr::RecordUpdate(_, base, fields) => {
            collect_scoped_var_references_inner(base, scope, refs);
            for (_, value) in fields {
                collect_scoped_var_references_inner(value, scope, refs);
            }
        }
        Expr::Computation(_, _, steps) => {
            let scope_start = scope.len();
            for step in steps {
                match step {
                    ComputationStep::LetBang(_, pattern, value) => {
                        collect_scoped_var_references_inner(value, scope, refs);
                        collect_pattern_bindings(pattern, scope);
                    }
                    ComputationStep::DoBang(_, expr)
                    | ComputationStep::Return(_, expr)
                    | ComputationStep::Expr(expr) => {
                        collect_scoped_var_references_inner(expr, scope, refs)
                    }
                }
            }
            scope.truncate(scope_start);
        }
        Expr::Quote(_, inner) | Expr::Unquote(_, inner) | Expr::UnquoteSplice(_, inner) => {
            collect_scoped_var_references_inner(inner, scope, refs);
        }
    }
}

fn collect_pattern_bindings(pattern: &Pattern, scope: &mut Vec<String>) {
    match pattern {
        Pattern::Wildcard(_) | Pattern::Lit(_, _) => {}
        Pattern::Var(_, name) => scope.push(name.clone()),
        Pattern::Constructor(_, _, fields) => {
            for field in fields {
                collect_pattern_bindings(field, scope);
            }
        }
        Pattern::RecordPat(_, _, fields) => {
            for (_, field) in fields {
                collect_pattern_bindings(field, scope);
            }
        }
    }
}

/// 組み込み関数・演算子名（検証で除外する）
fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "+" | "-"
            | "*"
            | "/"
            | "%"
            | "="
            | "!="
            | "<"
            | ">"
            | "<="
            | ">="
            | "and"
            | "or"
            | "not"
            | "print"
            | "println"
            | "true"
            | "false"
            | "nil"
    )
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
        check_invariant(
            diagnostics,
            name,
            &param_names,
            invariant_expr,
            span,
            all_names,
        );
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
    span: Span,
    all_names: &[String],
) {
    let var_refs = collect_scoped_var_references(invariant);

    for (ref_name, _ref_span) in &var_refs {
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
                span,
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
}

/// `:property` のうち、移行期 runner が実行できる narrow profile を返す。
///
/// type-directed sampling、seed、shrink は別 slice のため、ここで暗黙に既定値へ
/// 丸めず、明示的に profile 外として扱う。precondition は source order の
/// conjunction として deterministic sample の filter に使う。
pub fn property_smoke_test_spec(property: &PropertyForm) -> Option<PropertySmokeTestSpec> {
    if !(1..=2).contains(&property.binders().len())
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
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    match binder_types.as_slice() {
        [PropertyBinderType::Int] | [PropertyBinderType::Int, PropertyBinderType::Int] => {}
        [PropertyBinderType::Bool]
        | [PropertyBinderType::Int, PropertyBinderType::Bool]
        | [PropertyBinderType::Bool, PropertyBinderType::Int]
            if cases <= 2 => {}
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

            // 移行期 deterministic property smoke profile からテスト生成
            for form in &meta.forms {
                let MetadataFormKind::Property { properties } = &form.kind else {
                    continue;
                };
                for (property_index, property) in properties.iter().enumerate() {
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
mod tests {
    use super::*;

    fn check(source: &str) -> Vec<MetadataDiagnostic> {
        let program = lsharp_syntax::parse(source).unwrap();
        check_metadata(&program)
    }

    #[test]
    fn test_no_metadata_no_diagnostics() {
        let diags = check("(defn add [x y] (+ x y))");
        assert!(diags.is_empty());
    }

    #[test]
    fn test_correct_params_metadata() {
        let diags =
            check(r#"(defn add [x y] :doc "addition" :params [(x "left") (y "right")] (+ x y))"#);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_unknown_param_in_metadata() {
        let diags = check(r#"(defn add [x y] :params [(x "left") (z "unknown")] (+ x y))"#);
        // 'z' は引数にないのでエラー、'y' は :params にないので警告
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("'z'"));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("'y'"));
    }

    #[test]
    fn test_missing_param_documentation() {
        let diags = check(r#"(defn add [x y] :params [(x "left")] (+ x y))"#);
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("'y'"));
    }

    #[test]
    fn test_see_also_valid_reference() {
        let diags = check(
            r#"(defn add [x y] :doc "add" :see-also [sub] (+ x y))
               (defn sub [x y] (- x y))"#,
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn test_see_also_invalid_reference() {
        let diags = check(r#"(defn add [x y] :doc "add" :see-also [nonexistent] (+ x y))"#);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("'nonexistent'"));
    }

    // P3-3-4: :doc 内識別子チェック

    #[test]
    fn test_doc_valid_identifier_reference() {
        // :doc 内で参照した識別子が存在する場合は警告なし
        let diags = check(r#"(defn add [x y] :doc "Adds `x` and `y` together" (+ x y))"#);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_doc_valid_function_reference() {
        // :doc 内で他の関数を参照
        let diags = check(
            r#"(defn add [x y] :doc "See `sub` for subtraction" (+ x y))
               (defn sub [x y] (- x y))"#,
        );
        assert!(diags.is_empty());
    }

    #[test]
    fn test_doc_invalid_identifier_reference() {
        // :doc 内で存在しない識別子を参照した場合は警告
        let diags = check(r#"(defn add [x y] :doc "Uses `nonexistent_fn` internally" (+ x y))"#);
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("`nonexistent_fn`"));
    }

    #[test]
    fn test_doc_multiple_identifiers() {
        // 複数の識別子を参照: 1つは有効、1つは無効
        let diags = check(r#"(defn add [x y] :doc "Takes `x` and calls `missing`" (+ x y))"#);
        let warnings: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("`missing`"));
    }

    #[test]
    fn test_doc_no_backtick_identifiers() {
        // バッククォートのない :doc は識別子チェック対象外
        let diags = check(r#"(defn add [x y] :doc "Simple addition function" (+ x y))"#);
        assert!(diags.is_empty());
    }

    // extract_doc_identifiers 単体テスト

    #[test]
    fn test_extract_doc_identifiers_basic() {
        let idents = extract_doc_identifiers("Use `foo` and `bar`");
        assert_eq!(idents, vec!["foo", "bar"]);
    }

    #[test]
    fn test_extract_doc_identifiers_empty() {
        let idents = extract_doc_identifiers("No backticks here");
        assert!(idents.is_empty());
    }

    #[test]
    fn test_extract_doc_identifiers_nested() {
        // 空のバッククォートは無視
        let idents = extract_doc_identifiers("Empty `` ignored, `valid` kept");
        assert_eq!(idents, vec!["valid"]);
    }

    // P3-3-5: :invariant テスト

    #[test]
    fn test_invariant_valid_references() {
        // :invariant 内で引数と組み込み関数のみ参照 -> エラーなし
        let diags = check(r#"(defn abs [x] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"#);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_invariant_unknown_reference() {
        // :invariant 内で未定義の識別子を参照 -> エラー
        let diags =
            check(r#"(defn abs [x] :invariant (unknown-fn result) (if (< x 0) (- 0 x) x))"#);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("'unknown-fn'"));
        assert!(errors[0].message.contains(":invariant"));
    }

    #[test]
    fn test_invariant_references_other_function() {
        // :invariant 内で他の関数を参照 -> OK
        let diags = check(
            r#"(defn positive? [x] (> x 0))
               (defn abs [x] :invariant (positive? result) (if (< x 0) (- 0 x) x))"#,
        );
        // positive? は >=, > 等のように定義済みなのでOK
        assert!(diags.is_empty());
    }

    // P3-3-6: :example テスト

    #[test]
    fn test_example_valid_references() {
        // :example 内で関数自身と引数値のみ参照 -> エラーなし
        let diags = check(r#"(defn add [x y] :example [(add 1 2)] (+ x y))"#);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_example_unknown_reference() {
        // :example 内で未定義の識別子を参照 -> エラー
        let diags = check(r#"(defn add [x y] :example [(unknown-fn 1 2)] (+ x y))"#);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("'unknown-fn'"));
        assert!(errors[0].message.contains(":example"));
    }

    #[test]
    fn test_example_references_other_function() {
        // :example 内で他の定義済み関数を参照 -> OK
        let diags = check(
            r#"(defn double [x] (* x 2))
               (defn add [x y] :example [(= (add 1 2) 3)] (+ x y))"#,
        );
        assert!(diags.is_empty());
    }

    // collect_var_references 単体テスト

    #[test]
    fn test_collect_vars_from_app() {
        let expr = Expr::App(
            Span::new(0, 0),
            Box::new(Expr::Var(Span::new(0, 0), "add".to_string())),
            vec![
                Expr::Var(Span::new(0, 0), "x".to_string()),
                Expr::Lit(Span::new(0, 0), lsharp_syntax::ast::Literal::Int(1)),
            ],
        );
        let refs = collect_var_references(&expr);
        let names: Vec<&str> = refs.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["add", "x"]);
    }
}

#[cfg(test)]
mod test_generation_tests {
    use super::*;

    fn gen_tests(source: &str) -> Vec<GeneratedTest> {
        let program = lsharp_syntax::parse(source).unwrap();
        generate_tests(&program)
    }

    #[test]
    fn test_generate_invariant_test() {
        let tests = gen_tests(r#"(defn abs [x] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"#);
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "abs_invariant");
        assert_eq!(tests[0].function_name, "abs");
        assert_eq!(tests[0].kind, TestKind::Invariant);
    }

    #[test]
    fn test_generate_example_test() {
        let tests = gen_tests(r#"(defn add [x y] :example [(add 1 2)] (+ x y))"#);
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].name, "add_example_0");
        assert_eq!(tests[0].function_name, "add");
        assert_eq!(tests[0].kind, TestKind::Example);
    }

    #[test]
    fn test_generate_multiple_examples() {
        let tests = gen_tests(r#"(defn add [x y] :example [(add 1 2) (add 0 0)] (+ x y))"#);
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].name, "add_example_0");
        assert_eq!(tests[1].name, "add_example_1");
    }

    #[test]
    fn test_generate_both_invariant_and_example() {
        let tests = gen_tests(
            r#"(defn abs [x] :invariant (>= result 0) :example [(abs 5)] (if (< x 0) (- 0 x) x))"#,
        );
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].kind, TestKind::Invariant);
        assert_eq!(tests[1].kind, TestKind::Example);
    }

    #[test]
    fn test_generate_ordered_canonical_cases() {
        let tests =
            gen_tests(r#"(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))"#);
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].name, "succ_case_0");
        assert_eq!(tests[0].function_name, "succ");
        assert_eq!(tests[0].kind, TestKind::Case);
        assert_eq!(
            tests[0].expected.as_ref().map(ToString::to_string),
            Some("2".to_string())
        );
        assert_eq!(tests[1].name, "succ_case_1");
        assert_eq!(tests[1].kind, TestKind::Case);
        assert_eq!(
            tests[1].expected.as_ref().map(ToString::to_string),
            Some("4".to_string())
        );
    }

    #[test]
    fn test_no_tests_without_metadata() {
        let tests = gen_tests("(defn add [x y] (+ x y))");
        assert!(tests.is_empty());
    }

    #[test]
    fn test_no_tests_with_doc_only() {
        let tests = gen_tests(r#"(defn add [x y] :doc "adds" (+ x y))"#);
        assert!(tests.is_empty());
    }

    #[test]
    fn test_private_function_test_generation() {
        let tests = gen_tests(r#"(private (defn helper [x] :invariant (>= result 0) (+ x 1)))"#);
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].function_name, "helper");
    }
}
