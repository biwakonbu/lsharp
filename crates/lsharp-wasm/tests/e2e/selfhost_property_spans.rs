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
        post-span (if (> (vector-length payload) 5) (vector-get payload 5) 0)]
    (do
      (print (vector-length payload))
      (print (if (> (vector-length payload) 5) (vector-get post-span 0) -1))
      (print (if (> (vector-length payload) 5) (vector-get post-span 1) -1))
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
            &postcondition_start.to_string(),
            &postcondition_end.to_string(),
        ],
        "selfhost canonical property は postcondition の個別 source span を保持するべき"
    );
}
