use super::support::{
    NATIVE_HARNESS_STACK_BYTES, compile_and_run, run_metadata_tests, run_with_expanded_stack,
};

#[test]
fn selfhost_contract_suite_preserves_assert_predicate_source_spans() {
    let source = "(defn positive [] :assert [(> 1 0) (= 1 1)] true)";
    let first_start = source
        .find("(> 1 0)")
        .expect("first assert predicate が見つかる");
    let first_end = first_start + "(> 1 0)".len();
    let second_start = source
        .find("(= 1 1)")
        .expect("second assert predicate が見つかる");
    let second_end = second_start + "(= 1 1)".len();
    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source_literal}"
        suites (extract-parser-contract-suites src)
        assertion (vector-get (vector-get (vector-get suites 0) 2) 0)
        spans (if (> (vector-length assertion) 4) (vector-get assertion 4) 0)]
    (do
      (print (vector-length assertion))
      (print (if (> (vector-length assertion) 4) (vector-length spans) -1))
      (print (if (> (vector-length assertion) 4) (vector-get spans 0) -1))
      (print (if (> (vector-length assertion) 4) (vector-get spans 1) -1))
      (print (if (> (vector-length assertion) 4) (vector-get spans 2) -1))
      (print (if (> (vector-length assertion) 4) (vector-get spans 3) -1))
      0)))
"#
    );
    let combined = format!(
        "{}\n{}",
        super::support::selfhost_test_runner_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "5",
            "4",
            &first_start.to_string(),
            &first_end.to_string(),
            &second_start.to_string(),
            &second_end.to_string(),
        ],
        "selfhost canonical :assert は predicate ごとの source span を保持するべき"
    );
}

#[test]
fn selfhost_assertion_checker_reports_first_predicate_diagnostic_span() {
    let source = "(defn noop [] :assert [1] true)";
    let predicate_start = source
        .find("1")
        .expect("assert checker span fixture が見つかる");
    let predicate_end = predicate_start + 1;
    let empty_source = "(defn noop [] :assert [] true)";
    let empty_start = empty_source
        .find(":assert []")
        .expect("empty assert checker span fixture が見つかる");
    let empty_end = empty_start + ":assert []".len();
    let source_literal = source.replace('"', "\\\"");
    let empty_source_literal = empty_source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [program (parse-program "{source_literal}")
        result (check-canonical-assertions program)
        empty-program (parse-program "{empty_source_literal}")
        empty-result (check-canonical-assertions empty-program)]
    (do
      (print (vector-length result))
      (print (if (> (vector-length result) 2) (vector-get result 2) -1))
      (print (if (> (vector-length result) 3) (vector-get result 3) -1))
      (print (vector-length empty-result))
      (print (if (> (vector-length empty-result) 2) (vector-get empty-result 2) -1))
      (print (if (> (vector-length empty-result) 3) (vector-get empty-result 3) -1))
      0)))
"#
    );
    let combined = format!(
        "{}\n{}",
        super::support::selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "4",
            &predicate_start.to_string(),
            &predicate_end.to_string(),
            "4",
            &empty_start.to_string(),
            &empty_end.to_string(),
        ],
        "selfhost assertion checker は predicate と空 directive の source span を返すべき"
    );
}

#[test]
fn selfhost_test_runner_reports_assertion_diagnostic_span() {
    let source = "(defn positive [] :assert [(+ 1 2)] true)";
    let predicate_start = source
        .find("(+ 1 2)")
        .expect("assert diagnostic span fixture が見つかる");
    let predicate_end = predicate_start + "(+ 1 2)".len();
    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source_literal}"
        suite (generate-tests-from-source src)
        results (vector-get suite 2)
        result (vector-get results 0)]
    (do
      (print (vector-length result))
      (print (vector-get result 3))
      (print (vector-get result 4))
      (print (vector-get result 5))
      0)))
"#
    );
    let combined = format!(
        "{}\n{}",
        super::support::selfhost_test_runner_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "6",
            "2",
            &predicate_start.to_string(),
            &predicate_end.to_string(),
        ],
        "selfhost assert non-Bool 診断は predicate の source span を結果へ保持するべき"
    );
}

#[test]
fn selfhost_assertion_results_match_rust_oracle() {
    let source = "(defn truth [] (= 1 1)) (defn falsehood [] (= 1 2)) (defn positive [] :assert [(truth) (falsehood)] true)";
    let oracle = run_metadata_tests(source);
    assert_eq!(oracle.len(), 2, "Rust oracle は assertion 2 件を生成するべき");
    assert!(oracle[0].passed, "Rust oracle の1件目は pass するべき");
    assert!(!oracle[1].passed, "Rust oracle の2件目は fail するべき");
    assert!(oracle[0].error.is_none());
    assert!(oracle[1].error.is_some());

    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source_literal}"
        suite (generate-tests-from-source src)
        results (vector-get suite 2)
        result0 (vector-get results 0)
        result1 (vector-get results 1)]
    (do
      (print (vector-length results))
      (print (vector-get result0 1))
      (print (vector-get result0 2))
      (print (vector-get result1 1))
      (print (vector-get result1 2))
      (print (test-result-diagnostic result0))
      (print (test-result-diagnostic result1))
      0)))
"#
    );
    let combined = format!(
        "{}\n{}",
        super::support::selfhost_test_runner_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["2", "1", "1", "0", "1", "0", "0"],
        "selfhost assertion の実行数、pass/fail、diagnostic code は Rust oracle と一致するべき"
    );
}
