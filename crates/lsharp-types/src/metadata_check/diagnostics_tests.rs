use lsharp_syntax::ast::Decl;

use super::{MetadataDiagnostic, Severity};

#[test]
fn diagnostics_module_exposes_defn_metadata_check() {
    let program =
        lsharp_syntax::parse(r#"(defn add [x] :params [(missing "unknown")] x)"#).unwrap();
    let Decl::Defn {
        name,
        params,
        metadata: Some(metadata),
        span,
        ..
    } = &program.decls[0]
    else {
        panic!("expected a definition with metadata");
    };

    let mut diagnostics: Vec<MetadataDiagnostic> = Vec::new();
    super::diagnostics::check_defn_metadata(
        &mut diagnostics,
        name,
        params,
        metadata,
        *span,
        &["add".to_string()],
    );

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == Severity::Error && diagnostic.message.contains("missing")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == Severity::Warning && diagnostic.message.contains("x")
    }));
}

// --- CONTRACT-SCOPE-01 (`I-43`) ---
//
// `all_names` は top-level 宣言名しか持たず、ADT variant 名 / trait method 名 /
// quote されたシンボル / builtin の doc 参照を知らないため、正当なプログラムが
// Error で弾かれる。probe は `check_metadata` を直接叩く。

/// `I-43`: 指定 severity の診断メッセージだけを取り出す。
fn diagnostics_of(source: &str, severity: Severity) -> Vec<String> {
    let program = lsharp_syntax::parse(source)
        .unwrap_or_else(|error| panic!("fixture が parse できない: {error:?}\n--- \n{source}"));
    super::check_metadata(&program)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == severity)
        .map(|diagnostic| diagnostic.message)
        .collect()
}

fn errors_of(source: &str) -> Vec<String> {
    diagnostics_of(source, Severity::Error)
}

fn warnings_of(source: &str) -> Vec<String> {
    diagnostics_of(source, Severity::Warning)
}

#[test]
fn contract_scope_plain_function_reference_in_example_is_accepted_control() {
    let source = r#"
(defn helper [x] x)
(defn caller [x] :example [(helper 1)] x)
"#;
    assert_eq!(errors_of(source), Vec::<String>::new());
}

#[test]
fn contract_scope_adt_variant_in_example_is_accepted() {
    let source = r#"
(type Color Red Green)
(defn caller [c] :example [(caller Red)] c)
"#;
    assert_eq!(errors_of(source), Vec::<String>::new());
}

#[test]
fn contract_scope_adt_variant_in_invariant_is_accepted() {
    // `=` は Int 比較なので、variant を直接両辺に置くと型推論で落ちる
    // (識別子スコープではなく `I-43` の対象外)。ここは variant を
    // 関数へ渡す形で「識別子として解決できるか」だけを見る。
    let source = r#"
(type Color Red Green)
(defn code [(: c Color)] 0)
(defn caller [(: c Color)] :invariant (= (code Red) 0) c)
"#;
    assert_eq!(errors_of(source), Vec::<String>::new());
}

#[test]
fn contract_scope_trait_method_in_example_is_accepted() {
    let source = r#"
(trait (Show a) (defn show [self] 0))
(defn caller [x] :example [(show x)] x)
"#;
    assert_eq!(errors_of(source), Vec::<String>::new());
}

/// `I-43` + `I-59`: 識別子スコープ検査からは quote されたシンボルが消え、
/// 代わりに metadata 固有の Error がちょうど 1 件出る。
///
/// `:invariant` は `test_runner.rs:85` で生成ソースへ差し込まれて**実行される**ので、
/// quote を通しても `lsharp test` の lowering で落ちる。早く・正確に落とす。
/// 判断は `docs/adr/decisions-invariant-quote-handling.md`。
#[test]
fn contract_scope_quoted_symbol_in_invariant_reports_metadata_error() {
    let source = r#"
(defn caller [x] :invariant (= 'sym 'sym) x)
"#;
    let errors = errors_of(source);
    assert_eq!(errors.len(), 1, "診断はちょうど 1 件のはず: {errors:?}");
    assert!(
        !errors[0].contains("未定義の識別子") && !errors[0].contains("未定義の変数"),
        "識別子スコープ / 未定義変数 由来の見出しは残らないはず: {errors:?}"
    );
    assert!(
        errors[0].contains(":invariant") && errors[0].contains("quote"),
        ":invariant と quote に言及する metadata 固有のメッセージのはず: {errors:?}"
    );
}

#[test]
fn contract_scope_quoted_symbol_in_example_is_accepted() {
    let source = r#"
(defn caller [x] :example [(caller 'sym)] x)
"#;
    assert_eq!(errors_of(source), Vec::<String>::new());
}

#[test]
fn contract_scope_builtin_in_doc_backticks_is_accepted() {
    let source = r#"
(defn caller [x] :doc "uses `println` and `+`" x)
"#;
    assert_eq!(warnings_of(source), Vec::<String>::new());
}

// --- negative control: 本当に未定義なものは従来どおり Error のまま ---

#[test]
fn contract_scope_undefined_identifier_in_example_still_errors() {
    let source = r#"
(defn caller [x] :example [(nonexistent 1)] x)
"#;
    let errors = errors_of(source);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("nonexistent"), "{errors:?}");
}

#[test]
fn contract_scope_undefined_identifier_in_invariant_still_errors() {
    let source = r#"
(defn caller [x] :invariant (= x nonexistent) x)
"#;
    let errors = errors_of(source);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("nonexistent"), "{errors:?}");
}

#[test]
fn contract_scope_undefined_identifier_in_doc_backticks_still_warns() {
    let source = r#"
(defn caller [x] :doc "uses `nonexistent`" x)
"#;
    let warnings = warnings_of(source);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("nonexistent"), "{warnings:?}");
}

/// `I-43`: quote の内側でも `~` で戻した式は本物の参照なので検査を続ける。
#[test]
fn contract_scope_unquoted_reference_inside_quote_still_errors() {
    let source = r#"
(defn caller [x] :example [(caller '(a ~nonexistent))] x)
"#;
    let errors = errors_of(source);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("nonexistent"), "{errors:?}");
}
