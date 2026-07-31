use super::support::*;

#[test]
fn test_e2e_selfhost_typeinfer_record_registration_uses_bounded_chunks() {
    let record_decl = selfhost_module("TypeInferRecordDecl.ls");

    assert!(
        record_decl.contains("typeinfer-predeclare-record-env-step-64-loop-bounded")
            && record_decl.contains("typeinfer-predeclare-record-env-rooted-v3")
            && record_decl.contains("typeinfer-register-record-accessors-step-64-loop-bounded")
            && record_decl.contains("typeinfer-register-record-accessors-rooted-v3")
            && record_decl.contains("typeinfer-register-record-defs-step-64-loop-bounded")
            && record_decl.contains("typeinfer-register-record-defs-rooted-v3"),
        "record schema/accessor registration scan は bounded helper と rooted continuation へ分離するべき"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_large_record_registration_preserves_results() {
    let mut records_expr = "(vector-new 0)".to_string();
    let mut wide_fields_expr = "(vector-new 0)".to_string();
    let mut wide_record_type_expr = "(make-type-record 99000)".to_string();

    for idx in 0..65 {
        let record_name = 50000 + idx;
        let field_name = 60000 + idx;
        let accessor_name = 70000 + idx;
        let fields_expr = format!(
            "(vector-push (vector-push (vector-push (vector-new 0) {}) {}) (raw-type-named 100))",
            field_name, accessor_name
        );
        records_expr = format!(
            "(vector-push {} (make-record-def-with-fields {} {}))",
            records_expr, record_name, fields_expr
        );

        let wide_field = 80000 + idx;
        let wide_accessor = 90000 + idx;
        wide_fields_expr = format!(
            "(vector-push (vector-push (vector-push {} {}) {}) (raw-type-named 100))",
            wide_fields_expr, wide_field, wide_accessor
        );
        wide_record_type_expr = format!(
            "(type-record-add-field {} {} (mk-int))",
            wide_record_type_expr, wide_field
        );
    }

    let harness = format!(
        r#"
(defn raw-type-named [name-hash]
  (vector-push (vector-push (vector-new 2) 60) name-hash))

(defn main []
  (let [program {records_expr}
        counter0 (make-var-counter)
        alias-env (map-new)
        record-env (typeinfer-predeclare-record-env program alias-env counter0)
        counter (var-counter-with-alias-env-and-record-env counter0 alias-env record-env)
        registered (typeinfer-register-record-defs program (type-env-new) counter)
        wide-fields {wide_fields_expr}
        wide-record-type {wide_record_type_expr}
        wide-env
          (typeinfer-register-record-accessors
            wide-fields
            (type-env-new)
            wide-record-type
            (vector-new 0))]
    (do
      (print (if (= (map-get-safe record-env 50000) 0) 0 1))
      (print (if (= (map-get-safe record-env 50064) 0) 0 1))
      (print (if (= (type-env-lookup registered 50000) 0) 0 1))
      (print (if (= (type-env-lookup registered 50064) 0) 0 1))
      (print (if (= (type-env-lookup registered 70000) 0) 0 1))
      (print (if (= (type-env-lookup registered 70064) 0) 0 1))
      (print (if (= (type-env-lookup wide-env 90000) 0) 0 1))
      (print (if (= (type-env-lookup wide-env 90064) 0) 0 1))
      0)))
"#,
        records_expr = records_expr,
        wide_fields_expr = wide_fields_expr,
        wide_record_type_expr = wide_record_type_expr,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "1", "1", "1", "1", "1", "1", "1"],
        "65 record declaration/accessor registration は chunk 境界を越えて schema と env の結果を保持するべき"
    );
}
