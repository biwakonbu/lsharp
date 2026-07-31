use super::support::*;

#[test]
fn test_e2e_selfhost_type_record_ops_use_bounded_chunks() {
    let source = selfhost_module("Type.ls");

    assert!(
        source.contains("type-record-field-type-step-64-loop-bounded")
            && source.contains("type-record-field-type-rooted-v3")
            && source.contains("type-record-fields-eq-step-64-loop-bounded")
            && source.contains("type-record-fields-eq-rooted-v3"),
        "Type.ls record field operations should use bounded rooted helpers"
    );
}

#[test]
fn test_e2e_selfhost_large_record_type_operations_preserve_results() {
    let (type_ls, type_scheme_ls) = typescheme_runtime_modules();
    let mut record_expr = "(make-type-record 700)".to_string();
    let mut same_expr = "(make-type-record 700)".to_string();
    let mut mismatch_expr = "(make-type-record 700)".to_string();

    for idx in 0..65 {
        let field_hash = 18000 + idx;
        let field_ty = format!("(make-type-var {})", 1000 + idx);
        let mismatch_ty = if idx == 64 {
            "(make-type-int)".to_string()
        } else {
            field_ty.clone()
        };
        record_expr = format!(
            "(type-record-add-field {} {} {})",
            record_expr, field_hash, field_ty
        );
        same_expr = format!(
            "(type-record-add-field {} {} (make-type-var {}))",
            same_expr,
            field_hash,
            1000 + idx
        );
        mismatch_expr = format!(
            "(type-record-add-field {} {} {})",
            mismatch_expr, field_hash, mismatch_ty
        );
    }

    let harness = format!(
        r#"
(defn main []
  (let [record-ty {record_expr}
        same-ty {same_expr}
        mismatch-ty {mismatch_expr}
        equal-result (types-eq record-ty same-ty)
        mismatch-result (types-eq record-ty mismatch-ty)]
    (do
      (print (type-name (type-record-field-type record-ty 18000)))
      (print (type-name (type-record-field-type record-ty 18032)))
      (print (type-name (type-record-field-type record-ty 18064)))
      (print (type-record-field-type record-ty 49999))
      (print equal-result)
      (print mismatch-result)
      0)))
"#,
        record_expr = record_expr,
        same_expr = same_expr,
        mismatch_expr = mismatch_expr,
    );
    let combined = format!("{}\n{}\n{}", type_ls, type_scheme_ls, harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1000", "1032", "1064", "0", "1", "0"],
        "65 field の record lookup/equality は chunk 境界を越えて結果を保持するべき"
    );
}
