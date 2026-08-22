use super::support::*;

#[test]
fn test_e2e_selfhost_typeinfer_record_inference_uses_bounded_chunks() {
    let source = selfhost_module("TypeInferRecord.ls");

    assert!(
        source.contains("infer-record-fields-step-64-loop-bounded")
            && source.contains("infer-record-fields-rooted-v3")
            && source.contains("recordlit-field-node-step-64-loop-bounded")
            && source.contains("recordlit-field-node-rooted-v3")
            && source.contains("infer-declared-recordlit-fields-step-64-loop-bounded")
            && source.contains("infer-declared-recordlit-fields-rooted-v3")
            && source.contains("infer-declared-recordupdate-fields-step-64-loop-bounded")
            && source.contains("infer-declared-recordupdate-fields-rooted-v3"),
        "record field/value inference scans should use bounded rooted helpers"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_large_record_inference_preserves_results() {
    let mut record_node_expr =
        "(vector-push (vector-push (vector-push (vector-new 2) 0) 0) 65)".to_string();
    let mut typed_node_expr =
        "(vector-push (vector-push (vector-push (vector-new 2) 0) 0) 65)".to_string();
    let mut record_type_expr = "(make-type-record 700)".to_string();

    for idx in 0..65 {
        let field_hash = 18000 + idx;
        record_node_expr = format!(
            "(vector-push (vector-push {} {}) {})",
            record_node_expr,
            field_hash,
            100 + idx
        );
        typed_node_expr = format!(
            "(vector-push (vector-push {} {}) (make-lit-int {}))",
            typed_node_expr, field_hash, idx
        );
        record_type_expr = format!(
            "(type-record-add-field {} {} (mk-int))",
            record_type_expr, field_hash
        );
    }

    let harness = format!(
        r#"
(defn main []
  (let [record-node {record_node_expr}
        typed-node {typed_node_expr}
        record-ty {record_type_expr}
        counter (make-var-counter)
        env (init-builtin-env counter)
        fields-result
          (infer-record-fields typed-node 0 65 env (subst-new) counter)
        literal-result
          (infer-declared-recordlit-fields
            typed-node 0 65 env (subst-new) counter record-ty)
        update-result
          (infer-declared-recordupdate-fields
            typed-node 0 65 env (subst-new) counter record-ty)]
    (do
      (print (result-failed fields-result))
      (print (ty-tag (result-type fields-result)))
      (print (recordlit-field-node record-node 18000))
      (print (recordlit-field-node record-node 18032))
      (print (recordlit-field-node record-node 18064))
      (print (recordlit-field-node record-node 49999))
      (print (result-failed literal-result))
      (print (ty-tag (result-type literal-result)))
      (print (ty-name (result-type literal-result)))
      (print (result-failed update-result))
      (print (ty-tag (result-type update-result)))
      (print (ty-name (result-type update-result)))
      0)))
"#,
        record_node_expr = record_node_expr,
        typed_node_expr = typed_node_expr,
        record_type_expr = record_type_expr,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "0", "1", "100", "132", "164", "0", "0", "4", "700", "0", "4", "700",
        ],
        "65 要素の record field/value inference は chunk 境界を越えて結果を保持するべき"
    );
}
