use super::support::*;

#[test]
fn test_e2e_selfhost_type_record_checks_use_bounded_chunks() {
    let source = selfhost_module("Type.ls");

    assert!(
        source.contains("apply-subst-record-fields-step-64-loop-bounded")
            && source.contains("apply-subst-record-fields-rooted-v3")
            && source.contains("occurs-check-record-fields-step-64-loop-bounded")
            && source.contains("occurs-check-record-fields-rooted-v3")
            && source.contains("unify-record-fields-step-64-loop-bounded")
            && source.contains("unify-record-fields-rooted-v3"),
        "Type.ls record substitution/check/unification should use bounded rooted helpers"
    );
}

#[test]
fn test_e2e_selfhost_large_record_checks_preserve_results() {
    let (type_ls, type_scheme_ls) = typescheme_runtime_modules();
    let mut record_expr = "(make-type-record 700)".to_string();
    let mut same_expr = "(make-type-record 700)".to_string();
    let mut unify_left_expr = "(make-type-record 700)".to_string();
    let mut unify_right_expr = "(make-type-record 700)".to_string();

    for idx in 0..65 {
        let field_hash = 18000 + idx;
        record_expr = format!(
            "(type-record-add-field {} {} (make-type-var {}))",
            record_expr,
            field_hash,
            1000 + idx
        );
        same_expr = format!(
            "(type-record-add-field {} {} (make-type-var {}))",
            same_expr,
            field_hash,
            1000 + idx
        );
        unify_left_expr = format!(
            "(type-record-add-field {} {} (make-type-int))",
            unify_left_expr, field_hash
        );
        let right_ty = if idx == 64 {
            "(make-type-string)"
        } else {
            "(make-type-int)"
        };
        unify_right_expr = format!(
            "(type-record-add-field {} {} {})",
            unify_right_expr, field_hash, right_ty
        );
    }

    let harness = format!(
        r#"
(defn main []
  (let [record-ty {record_expr}
        same-ty {same_expr}
        unify-left {unify_left_expr}
        unify-right {unify_right_expr}
        subst1 (subst-bind (subst-new) 1000 (make-type-string))
        substituted (apply-subst subst1 record-ty)
        unified (unify record-ty same-ty (subst-new))
        unify-mismatch (unify unify-left unify-right (subst-new))]
    (do
      (print (type-tag (type-record-field-type substituted 18000)))
      (print (type-name (type-record-field-type substituted 18000)))
      (print (type-tag (type-record-field-type substituted 18064)))
      (print (type-name (type-record-field-type substituted 18064)))
      (print (occurs-check 1000 record-ty))
      (print (occurs-check 9999 record-ty))
      (print (unify-failed unified))
      (print (unify-failed unify-mismatch))
      0)))
"#,
        record_expr = record_expr,
        same_expr = same_expr,
        unify_left_expr = unify_left_expr,
        unify_right_expr = unify_right_expr,
    );
    let combined = format!("{}\n{}\n{}", type_ls, type_scheme_ls, harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "300", "2", "1064", "1", "0", "0", "1"],
        "65 field の substitution/occurs/unify は chunk 境界を越えて結果を保持するべき"
    );
}
