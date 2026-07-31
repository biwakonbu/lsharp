use super::support::*;

#[test]
fn test_e2e_selfhost_typeinfer_block_scans_use_bounded_rooted_chunks() {
    let source = selfhost_module("TypeInferBlock.ls");

    assert!(
        source.contains("infer-do-step-64-loop-bounded")
            && source.contains("infer-do-rooted-v3")
            && source.contains("infer-computation-step-64-loop-bounded")
            && source.contains("infer-computation-rooted-v3"),
        "TypeInferBlock の do/computation scan は bounded helper と rooted continuation へ分離するべき"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_large_do_and_computation_preserve_results() {
    let mut do_node = "(vector-push (vector-push (vector-new 67) 9) 65)".to_string();
    for idx in 0..64 {
        do_node = format!("(vector-push {} (make-lit-int {}))", do_node, idx);
    }
    do_node = format!("(vector-push {} (make-lit-bool 1))", do_node);

    let mut computation_node =
        "(vector-push (vector-push (vector-push (vector-new 198) 15) 904) 65)".to_string();
    for idx in 0..64 {
        computation_node = format!(
            "(vector-push {} (computation-step-do-bang))",
            computation_node
        );
        computation_node = format!("(vector-push {} 0)", computation_node);
        computation_node = format!("(vector-push {} (make-lit-int {}))", computation_node, idx);
    }
    computation_node = format!(
        "(vector-push {} (computation-step-return))",
        computation_node
    );
    computation_node = format!("(vector-push {} 0)", computation_node);
    computation_node = format!("(vector-push {} (make-lit-bool 1))", computation_node);

    let harness = format!(
        r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        do-node {do_node}
        computation-node {computation_node}
        do-result (infer-expr do-node env (subst-new) counter)
        computation-result (infer-expr computation-node env (subst-new) counter)]
    (do
      (print (result-failed do-result))
      (print (ty-tag (result-type do-result)))
      (print (ty-name (result-type do-result)))
      (print (result-failed computation-result))
      (print (ty-tag (result-type computation-result)))
      (print (ty-name (result-type computation-result)))
      0)))
"#,
        do_node = do_node,
        computation_node = computation_node,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "200", "0", "1", "200"],
        "65 do expressions and computation steps should preserve the final Bool type"
    );
}
