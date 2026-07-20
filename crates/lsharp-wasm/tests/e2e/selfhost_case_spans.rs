use super::support::{NATIVE_HARNESS_STACK_BYTES, compile_and_run, run_with_expanded_stack};

#[test]
fn selfhost_contract_suite_preserves_case_expectation_source_spans() {
    let source = "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))";
    let first_start = source
        .find("(expect (succ 1) 2)")
        .expect("first case expectation span fixture が見つかる");
    let first_end = first_start + "(expect (succ 1) 2)".len();
    let second_start = source
        .find("(expect (succ 2) 4)")
        .expect("second case expectation span fixture が見つかる");
    let second_end = second_start + "(expect (succ 2) 4)".len();
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
      (print (vector-length second))
      (print (if (> (vector-length second) 2) (vector-get second 2) -1))
      (print (if (> (vector-length second) 3) (vector-get second 3) -1))
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
            "4",
            &first_start.to_string(),
            &first_end.to_string(),
            "4",
            &second_start.to_string(),
            &second_end.to_string(),
        ],
        "selfhost canonical case は expectation 全体の個別 source span を保持するべき"
    );
}
