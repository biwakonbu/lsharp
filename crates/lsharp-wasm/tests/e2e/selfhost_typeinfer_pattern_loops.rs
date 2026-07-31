use super::support::*;

#[test]
fn test_e2e_selfhost_typeinfer_pattern_children_use_bounded_chunks() {
    let source = selfhost_module("TypeInferPattern.ls");

    assert!(
        source.contains("infer-pattern-children-step-64-loop-bounded")
            && source.contains("infer-pattern-children-rooted-v3")
            && source.contains("infer-constructor-pattern-children-step-64-loop-bounded")
            && source.contains("infer-constructor-pattern-children-rooted-v3")
            && source.contains("infer-record-pattern-schema-children-step-64-loop-bounded")
            && source.contains("infer-record-pattern-schema-children-rooted-v3"),
        "TypeInferPattern child scans should use bounded rooted helpers"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_pattern_children_cross_chunk_boundary() {
    let mut generic_node = "(vector-new 0)".to_string();
    let mut constructor_node =
        "(vector-push (vector-push (vector-push (vector-new 3) 11) 3000) 65)".to_string();
    let mut constructor_type = "(mk-con 4000)".to_string();
    let mut record_node = "(vector-push (vector-push (vector-new 2) 44) 65)".to_string();
    let mut record_type = "(make-type-record 5000)".to_string();

    for idx in 0..65 {
        generic_node = format!(
            "(vector-push {} (vector-push (vector-push (vector-new 2) 4) {}))",
            generic_node,
            2000 + idx
        );
        constructor_node = format!(
            "(vector-push {} (vector-push (vector-new 1) 1))",
            constructor_node
        );
        constructor_type = format!("(mk-fun (mk-int) {})", constructor_type);
        record_node = format!(
            "(vector-push (vector-push {} {}) (vector-push (vector-new 1) 1))",
            record_node,
            18000 + idx
        );
        record_type = format!(
            "(type-record-add-field {} {} (mk-int))",
            record_type,
            18000 + idx
        );
    }

    let harness = format!(
        r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        generic-result
          (infer-pattern-children {generic_node} 0 65 0 1 env (subst-new) counter)
        generic-subst (pattern-children-subst generic-result)
        generic-env (pattern-children-env generic-result)
        constructor-result
          (infer-constructor-pattern-children
            {constructor_node} 0 65 env (subst-new) counter {constructor_type})
        record-result
          (infer-record-pattern-schema-children
            {record_node} 0 65 env (subst-new) counter {record_type})]
    (do
      (print (map-get-safe generic-subst -1))
      (print (if (= (type-env-lookup generic-env 2064) 0) 0 1))
      (print (result-failed constructor-result))
      (print (ty-tag (result-type constructor-result)))
      (print (ty-name (result-type constructor-result)))
      (print (result-failed record-result))
      (print (ty-tag (result-type record-result)))
      (print (ty-name (result-type record-result)))
      0)))
"#,
        generic_node = generic_node,
        constructor_node = constructor_node,
        constructor_type = constructor_type,
        record_node = record_node,
        record_type = record_type,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "0", "1", "4000", "0", "4", "5000"],
        "65 child patterns は chunk 境界を越えて binding/unify/schema 結果を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_match_arms_use_bounded_rooted_chunks() {
    let source = selfhost_module("TypeInferPattern.ls");

    assert!(
        source.contains("infer-match-arms-step-64-loop-bounded")
            && source.contains("infer-match-arms-rooted-v3")
            && source.contains("strip-match-scrutinee-vars-step-64-loop-bounded")
            && source.contains("strip-match-scrutinee-vars-rooted-v3"),
        "TypeInferPattern match arm scans should use bounded rooted helpers"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_match_arms_cross_chunk_boundary() {
    let mut node =
        "(vector-push (vector-push (vector-push (vector-new 3) 10) (make-lit-int 1)) 65)"
            .to_string();

    for idx in 0..65 {
        node = format!(
            "(vector-push (vector-push {} (make-lit-int 1)) (make-lit-int {}))",
            node,
            9000 + idx
        );
    }

    let harness = format!(
        r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        result (infer-expr {node} env (subst-new) counter)]
    (do
      (print (result-failed result))
      (print (ty-tag (result-type result)))
      (print (ty-name (result-type result)))
      0)))
"#,
        node = node,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "100"],
        "65 match arms は chunk 境界を越えて結果型を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_gadt_scrutinee_strip_crosses_chunk_boundary() {
    let mut vars = "(vector-new 0)".to_string();
    let mut subst = "(subst-new)".to_string();
    for idx in 0..65 {
        let var_id = 1000 + idx;
        vars = format!("(vector-push {} {})", vars, var_id);
        subst = format!(
            "(map-insert-object-safe {} {} (make-type-var {}))",
            subst, var_id, var_id
        );
    }

    let harness = format!(
        r#"
(defn main []
  (let [vars {vars}
        subst {subst}
        result (strip-match-scrutinee-vars subst vars 0 65)]
    (do
      (print (map-get-safe result 1000))
      (print (map-get-safe result 1064))
      0)))
"#,
        vars = vars,
        subst = subst,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0"],
        "65個の GADT scrutinee 型変数は chunk 境界を越えて arm-local に除去されるべき"
    );
}
