use super::support::{NATIVE_HARNESS_STACK_BYTES, compile_and_run, run_with_expanded_stack};

#[test]
fn selfhost_contract_suite_preserves_property_binder_source_span() {
    let source = "(defn identity [x] :property [(for-all [value Int] :cases 1 :postcondition (= result value))] x)";
    let binder_start = source
        .find("value Int")
        .expect("binder span fixture が見つかる");
    let binder_end = binder_start + "value Int".len();
    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source_literal}"
        suites (extract-parser-contract-suites src)
        property (vector-get (vector-get (vector-get suites 0) 2) 0)
        payload (vector-get property 1)
        binder (vector-get (vector-get payload 0) 0)]
    (do
      (print (vector-length binder))
      (print (if (> (vector-length binder) 4) (vector-get binder 3) -1))
      (print (if (> (vector-length binder) 4) (vector-get binder 4) -1))
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
        vec!["5", &binder_start.to_string(), &binder_end.to_string()],
        "selfhost canonical property は binder の個別 source span を保持するべき"
    );
}

#[test]
fn selfhost_contract_suite_preserves_property_postcondition_source_span() {
    let source = "(defn identity [x] :property [(for-all [value Int] :cases 1 :postcondition (= result value))] x)";
    let postcondition_start = source
        .find("(= result value)")
        .expect("postcondition span fixture が見つかる");
    let postcondition_end = postcondition_start + "(= result value)".len();
    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source_literal}"
        suites (extract-parser-contract-suites src)
        property (vector-get (vector-get (vector-get suites 0) 2) 0)
        payload (vector-get property 1)
        post-span (if (> (vector-length payload) 5) (vector-get payload 5) 0)
        precondition-spans (if (> (vector-length payload) 6) (vector-get payload 6) 0)]
    (do
      (print (vector-length payload))
      (print (if (> (vector-length payload) 5) (vector-get post-span 0) -1))
      (print (if (> (vector-length payload) 5) (vector-get post-span 1) -1))
      (print (if (> (vector-length payload) 6) (vector-length precondition-spans) -1))
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
            "7",
            &postcondition_start.to_string(),
            &postcondition_end.to_string(),
            "0",
        ],
        "selfhost canonical property は postcondition の個別 source span を保持するべき"
    );
}

#[test]
fn selfhost_contract_suite_preserves_property_precondition_source_spans() {
    let source = "(defn identity [x] :property [(for-all [value Int] :cases 1 :precondition [(>= value 0) (= value 0)] :postcondition (= result value))] x)";
    let first_start = source
        .find("(>= value 0)")
        .expect("first precondition span fixture が見つかる");
    let first_end = first_start + "(>= value 0)".len();
    let second_start = source
        .find("(= value 0)")
        .expect("second precondition span fixture が見つかる");
    let second_end = second_start + "(= value 0)".len();
    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source_literal}"
        suites (extract-parser-contract-suites src)
        property (vector-get (vector-get (vector-get suites 0) 2) 0)
        payload (vector-get property 1)
        spans (if (> (vector-length payload) 6) (vector-get payload 6) 0)]
    (do
      (print (vector-length payload))
      (print (if (> (vector-length payload) 6) (vector-length spans) -1))
      (print (if (> (vector-length payload) 6) (vector-get spans 0) -1))
      (print (if (> (vector-length payload) 6) (vector-get spans 1) -1))
      (print (if (> (vector-length payload) 6) (vector-get spans 2) -1))
      (print (if (> (vector-length payload) 6) (vector-get spans 3) -1))
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
            "7",
            "4",
            &first_start.to_string(),
            &first_end.to_string(),
            &second_start.to_string(),
            &second_end.to_string(),
        ],
        "selfhost canonical property は複数 precondition の個別 source span を保持するべき"
    );
}

#[test]
fn selfhost_test_runner_reports_property_unknown_variable_diagnostic_span() {
    let source = "(defn identity [x] :property [(for-all [value Int] :cases 1 :postcondition missing)] x)";
    let unknown_start = source
        .find("missing")
        .expect("property unknown variable span fixture が見つかる");
    let unknown_end = unknown_start + "missing".len();
    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source_literal}"
        suite (generate-tests-from-source src)
        results (vector-get suite 4)
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
            &unknown_start.to_string(),
            &unknown_end.to_string(),
        ],
        "selfhost property unknown-variable 診断は postcondition の source span を結果へ保持するべき"
    );
}

#[test]
fn selfhost_test_runner_reports_property_precondition_unknown_variable_diagnostic_span() {
    let source = "(defn identity [x] :property [(for-all [value Int] :cases 1 :precondition [(> missing 0)] :postcondition (= result value))] x)";
    let unknown_start = source
        .find("missing")
        .expect("property precondition unknown variable span fixture が見つかる");
    let unknown_end = unknown_start + "missing".len();
    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source_literal}"
        suite (generate-tests-from-source src)
        results (vector-get suite 4)
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
            &unknown_start.to_string(),
            &unknown_end.to_string(),
        ],
        "selfhost property unknown-variable 診断は precondition の source span を結果へ保持するべき"
    );
}
