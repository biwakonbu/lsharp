use super::support::*;

/// EC-M1 runtime: selfhost AST は String literal の decoded value を source span と併記する。
#[test]
fn test_e2e_selfhost_parser_preserves_decoded_string_value() {
    let harness = r#"
(defn main []
  (let [plain (vector-get (parse-program "\"same\"") 0)
        escaped (vector-get (parse-program "\"line\\nnext\"") 0)]
    (do
      (print (vector-length plain))
      (print (if (> (vector-length plain) 4)
        (if (string-eq (vector-get plain 4) "same") 1 0)
        0))
      (print (vector-length escaped))
      (print (if (> (vector-length escaped) 4)
        (if (string-eq (vector-get escaped 4) "line\nnext") 1 0)
        0))
      0)))
"#;

    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);

    assert_eq!(
        output.trim().lines().collect::<Vec<_>>(),
        vec!["5", "1", "5", "1"],
        "String AST は span/hash だけでなく decoded value を保持するべき"
    );
}

/// EC-M1 runtime: selfhost TestRunner は String literal と pattern を内容で比較する。
#[test]
fn test_e2e_selfhost_test_runner_compares_string_values_by_content() {
    let harness = r#"
(defn main []
  (let [src "(defn classify [] :case [(expect (match \"same\" [\"same\" 1] [_ 0]) 1) (expect (match \"other\" [\"same\" 1] [_ 0]) 0) (expect \"same\" \"same\") (expect \"same\" \"other\")] 0)"
        suite (generate-tests-from-source src)
        cases (vector-get suite 3)]
    (do
      (print (vector-length cases))
      (print (vector-get (vector-get cases 0) 1))
      (print (vector-get (vector-get cases 1) 1))
      (print (vector-get (vector-get cases 2) 1))
      (print (vector-get (vector-get cases 3) 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });

    assert_eq!(
        output.trim().lines().collect::<Vec<_>>(),
        vec!["4", "1", "1", "1", "0"],
        "selfhost runner は String の一致・不一致と literal pattern を内容で評価するべき"
    );
}

/// EC-M1-05: string-eq は decoded literal と property sample の内部表現差を越えて比較する。
#[test]
fn test_e2e_selfhost_test_runner_string_eq_compares_literals_and_property_samples() {
    let harness = r#"
(defn main []
  (let [src "(defn identity [x] :case [(expect (string-eq \"same\" \"same\") true) (expect (string-eq \"same\" \"other\") false)] :property [(for-all [sample String] :cases 1 :postcondition (string-eq result \"\"))] x)"
        suite (generate-tests-from-source src)
        cases (vector-get suite 3)
        properties (vector-get suite 4)
        property-result (vector-get properties 0)]
    (do
      (print (vector-length cases))
      (print (vector-get (vector-get cases 0) 1))
      (print (vector-get (vector-get cases 1) 1))
      (print (vector-length properties))
      (print (vector-get property-result 1))
      (print (vector-get property-result 2))
      (print (vector-get property-result 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_test_runner_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });

    assert_eq!(
        output.trim().lines().collect::<Vec<_>>(),
        vec!["2", "1", "1", "1", "1", "1", "0"],
        "string-eq は literal/literal と property sample/literal を decoded text で比較するべき"
    );
}
