use super::support::{NATIVE_HARNESS_STACK_BYTES, compile_and_run, run_with_expanded_stack};

#[test]
fn selfhost_contract_suite_preserves_case_expectation_source_spans() {
    let source = "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))";
    let first_start = source
        .find("(expect (succ 1) 2)")
        .expect("first case expectation span fixture が見つかる");
    let first_end = first_start + "(expect (succ 1) 2)".len();
    let first_actual_start = first_start + "(expect ".len();
    let first_actual_end = first_actual_start + "(succ 1)".len();
    let first_expected_start = first_actual_end + 1;
    let first_expected_end = first_expected_start + "2".len();
    let second_start = source
        .find("(expect (succ 2) 4)")
        .expect("second case expectation span fixture が見つかる");
    let second_end = second_start + "(expect (succ 2) 4)".len();
    let second_actual_start = second_start + "(expect ".len();
    let second_actual_end = second_actual_start + "(succ 2)".len();
    let second_expected_start = second_actual_end + 1;
    let second_expected_end = second_expected_start + "4".len();
    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source_literal}"
        suites (extract-parser-contract-suites src)
        cases (vector-get (vector-get (vector-get suites 0) 2) 0)
        expectations (vector-get cases 1)
        first (vector-get expectations 0)
        second (vector-get expectations 1)]
    (do
      (print (vector-length first))
      (print (if (> (vector-length first) 2) (vector-get first 2) -1))
      (print (if (> (vector-length first) 3) (vector-get first 3) -1))
      (print (if (> (vector-length first) 4) (vector-get first 4) -1))
      (print (if (> (vector-length first) 5) (vector-get first 5) -1))
      (print (if (> (vector-length first) 6) (vector-get first 6) -1))
      (print (if (> (vector-length first) 7) (vector-get first 7) -1))
      (print (vector-length second))
      (print (if (> (vector-length second) 2) (vector-get second 2) -1))
      (print (if (> (vector-length second) 3) (vector-get second 3) -1))
      (print (if (> (vector-length second) 4) (vector-get second 4) -1))
      (print (if (> (vector-length second) 5) (vector-get second 5) -1))
      (print (if (> (vector-length second) 6) (vector-get second 6) -1))
      (print (if (> (vector-length second) 7) (vector-get second 7) -1))
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
            "8",
            &first_start.to_string(),
            &first_end.to_string(),
            &first_actual_start.to_string(),
            &first_actual_end.to_string(),
            &first_expected_start.to_string(),
            &first_expected_end.to_string(),
            "8",
            &second_start.to_string(),
            &second_end.to_string(),
            &second_actual_start.to_string(),
            &second_actual_end.to_string(),
            &second_expected_start.to_string(),
            &second_expected_end.to_string(),
        ],
        "selfhost canonical case は expectation 全体の個別 source span を保持するべき"
    );
}

#[test]
fn selfhost_test_runner_preserves_case_expression_spans() {
    let source = "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))";
    let first_start = source
        .find("(expect (succ 1) 2)")
        .expect("first case expectation span fixture が見つかる");
    let first_actual_start = first_start + "(expect ".len();
    let first_actual_end = first_actual_start + "(succ 1)".len();
    let first_expected_start = first_actual_end + 1;
    let first_expected_end = first_expected_start + "2".len();
    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source_literal}"
        cases (extract-cases-from-program (parse-program src))
        first (vector-get cases 0)
        second (vector-get cases 1)]
    (do
      (print (vector-length first))
      (print (if (> (vector-length first) 4) (vector-get first 4) -1))
      (print (if (> (vector-length first) 5) (vector-get first 5) -1))
      (print (if (> (vector-length first) 6) (vector-get first 6) -1))
      (print (if (> (vector-length first) 7) (vector-get first 7) -1))
      (print (vector-length second))
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
            "8",
            &first_actual_start.to_string(),
            &first_actual_end.to_string(),
            &first_expected_start.to_string(),
            &first_expected_end.to_string(),
            "8",
        ],
        "selfhost test runner は case actual/expected の個別 source span を保持するべき"
    );
}

#[test]
fn selfhost_test_runner_reports_case_diagnostic_span() {
    let source = "(defn identity [x] :case [(expect missing 1)] x)";
    let entry_start = source
        .find("(expect missing 1)")
        .expect("case diagnostic span fixture が見つかる");
    let actual_start = entry_start + "(expect ".len();
    let actual_end = actual_start + "missing".len();
    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source_literal}"
        suite (generate-tests-from-source src)
        results (vector-get suite 3)
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
            "1",
            &actual_start.to_string(),
            &actual_end.to_string(),
        ],
        "selfhost test runner は case の unknown-variable 診断 span を結果へ保持するべき"
    );
}

#[test]
fn selfhost_case_typecheck_selects_expected_span_for_type_mismatch() {
    let source = "(defn noop [] :case [(expect 1 true)] true)";
    let expected_start = source
        .find("(expect 1 true)")
        .expect("case mismatch span fixture が見つかる")
        + "(expect 1 ".len();
    let expected_end = expected_start + "true".len();
    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [program (parse-program "{source_literal}")
        result (check-canonical-cases program)]
    (do
      (print (vector-length result))
      (print (vector-get result 0))
      (print (vector-get result 1))
      (print (vector-get result 2))
      (print (vector-get result 3))
      0)))
"#
    );
    let combined = format!(
        "{}\n{}",
        super::support::selfhost_typeinfer_runtime_bundle(),
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
            "1",
            "1002",
            &expected_start.to_string(),
            &expected_end.to_string(),
        ],
        "selfhost case typecheck は actual/expected 型不一致を expected span へ紐付けるべき"
    );
}

#[test]
fn selfhost_test_runner_reports_case_expected_unknown_variable_span() {
    let source = "(defn noop [] :case [(expect 1 missing)] true)";
    let expected_start = source
        .find("(expect 1 missing)")
        .expect("case expected unknown variable span fixture が見つかる")
        + "(expect 1 ".len();
    let expected_end = expected_start + "missing".len();
    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source_literal}"
        suite (generate-tests-from-source src)
        results (vector-get suite 3)
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
            "1",
            &expected_start.to_string(),
            &expected_end.to_string(),
        ],
        "selfhost case expected-side unknown-variable 診断は expected span を結果へ保持するべき"
    );
}
