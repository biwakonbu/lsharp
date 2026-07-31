use super::support::*;

#[test]
fn test_e2e_selfhost_typeinfer_record_filtering_uses_bounded_chunks() {
    let record_decl = selfhost_module("TypeInferRecordDecl.ls");
    let adt = selfhost_module("TypeInferAdt.ls");

    assert!(
        record_decl.contains("typeinfer-record-only-contains-step-64-loop-bounded")
            && record_decl.contains("typeinfer-record-only-contains-rooted-v3")
            && record_decl
                .contains("typeinfer-record-remove-unallowed-accessors-step-64-loop-bounded")
            && record_decl.contains("typeinfer-record-remove-unallowed-accessors-rooted-v3")
            && adt.contains("typeinfer-register-adt-defs-step-64-loop-bounded")
            && adt.contains("typeinfer-register-adt-defs-rooted-v3"),
        "record :only filtering と ADT definition scan は bounded helper へ分離するべき"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_large_record_filtering_and_adt_registration_preserve_results() {
    let mut only_hashes_expr = "(vector-new 0)".to_string();
    let mut raw_fields_expr = "(vector-new 0)".to_string();
    let mut record_env_expr = "(type-env-new)".to_string();
    let mut program_expr = "(vector-new 0)".to_string();

    for idx in 0..65 {
        let accessor_hash = 40000 + idx;
        let only_hash = if idx == 0 || idx == 64 {
            accessor_hash
        } else {
            50000 + idx
        };
        only_hashes_expr = format!("(vector-push {} {})", only_hashes_expr, only_hash);
        raw_fields_expr = format!(
            "(vector-push (vector-push (vector-push {} {}) {}) (vector-new 0))",
            raw_fields_expr,
            30000 + idx,
            accessor_hash
        );
        record_env_expr = format!(
            "(type-env-insert {} {} (mono (mk-int)))",
            record_env_expr, accessor_hash
        );

        let variant = format!("(make-type-variant {} (vector-new 0))", 22000 + idx);
        let variants_expr = format!("(vector-push (vector-new 0) {})", variant);
        program_expr = format!(
            "(vector-push {} (make-type-decl-with-variants {} {}))",
            program_expr,
            21000 + idx,
            variants_expr
        );
    }

    let harness = format!(
        r#"
(defn main []
  (let [only-hashes {only_hashes_expr}
        raw-fields {raw_fields_expr}
        record-env {record_env_expr}
        filtered-env
          (typeinfer-record-remove-unallowed-accessors-loop
            raw-fields 0 195 only-hashes record-env)
        program {program_expr}
        counter (make-var-counter)
        adt-env (typeinfer-register-adt-defs program (type-env-new) counter)
        first-allowed
          (typeinfer-record-export-allowed? only-hashes 40000)
        last-allowed
          (typeinfer-record-export-allowed? only-hashes 40064)
        missing-allowed
          (typeinfer-record-export-allowed? only-hashes 49999)
        first-accessor (type-env-lookup filtered-env 40000)
        middle-accessor (type-env-lookup filtered-env 40032)
        last-accessor (type-env-lookup filtered-env 40064)
        first-constructor (type-env-lookup adt-env 22000)
        last-constructor (type-env-lookup adt-env 22064)]
    (do
      (print first-allowed)
      (print last-allowed)
      (print missing-allowed)
      (print (if (= first-accessor 0) 0 1))
      (print (if (= middle-accessor 0) 1 0))
      (print (if (= last-accessor 0) 0 1))
      (print (if (= first-constructor 0) 0 1))
      (print (if (= last-constructor 0) 0 1))
      0)))
"#,
        only_hashes_expr = only_hashes_expr,
        raw_fields_expr = raw_fields_expr,
        record_env_expr = record_env_expr,
        program_expr = program_expr,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "1", "0", "1", "1", "1", "1", "1"],
        "65 要素の record :only filtering / accessor cleanup / ADT definition scan は chunk 境界を越えて結果を保持するべき"
    );
}
