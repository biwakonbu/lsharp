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
    let diags = check(r#"(defn abs [x] :invariant (unknown-fn result) (if (< x 0) (- 0 x) x))"#);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("'unknown-fn'"));
    assert!(errors[0].message.contains(":invariant"));
}

#[test]
fn test_invariant_unknown_reference_uses_identifier_span() {
    let source = "(defn succ [x] :invariant (= result (+ missing 1)) (+ x 1))";
    let diags = check(source);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .collect();
    let missing_start = source
        .find("missing")
        .expect("fixture に missing があるべき");

    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].span.start, missing_start);
    assert_eq!(errors[0].span.end, missing_start + "missing".len());
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
