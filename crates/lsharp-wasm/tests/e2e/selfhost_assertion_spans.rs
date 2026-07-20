use super::support::{NATIVE_HARNESS_STACK_BYTES, compile_and_run, run_with_expanded_stack};

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
