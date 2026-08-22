use super::support::*;

#[test]
fn test_e2e_selfhost_typescheme_traversals_use_bounded_rooted_chunks() {
    let source = selfhost_module("TypeScheme.ls");

    assert!(
        source.contains("instantiate-build-subst-step-64-loop-bounded")
            && source.contains("instantiate-build-subst-rooted-v3")
            && source.contains("instantiate-apply-record-fields-step-64-loop-bounded")
            && source.contains("instantiate-apply-record-fields-rooted-v3")
            && source.contains("instantiate-apply-app-args-step-64-loop-bounded")
            && source.contains("instantiate-apply-app-args-rooted-v3")
            && source.contains("free-vars-contains-step-64-loop-bounded")
            && source.contains("free-vars-append-unique-step-64-loop-bounded")
            && source.contains("free-vars-record-fields-step-64-loop-bounded")
            && source.contains("free-vars-app-args-step-64-loop-bounded")
            && source.contains("generalize-collect-bound-step-64-loop-bounded"),
        "TypeScheme traversals should use bounded rooted helpers"
    );
}

#[test]
fn test_e2e_selfhost_typescheme_large_traversals_preserve_results() {
    let (type_ls, type_scheme_ls) = typescheme_runtime_modules();
    let mut bound_expr = "(vector-new 0)".to_string();
    let mut args_expr = "(vector-new 0)".to_string();
    let mut record_expr = "(make-type-record 700)".to_string();

    for idx in 0..65 {
        let var_id = 2000 + idx;
        bound_expr = format!("(vector-push {} {})", bound_expr, var_id);
        args_expr = format!("(vector-push {} (make-type-var {}))", args_expr, var_id);
        record_expr = format!(
            "(type-record-add-field {} {} (make-type-var {}))",
            record_expr,
            18000 + idx,
            var_id
        );
    }

    let harness = format!(
        r#"
(defn main []
  (let [bound {bound_expr}
        args {args_expr}
        record-ty {record_expr}
        app-ty (make-type-app 9000 args)
        fun-ty (make-type-fun record-ty app-ty)
        scheme (poly fun-ty bound)
        counter (make-var-counter)
        instantiated (instantiate scheme counter)
        instantiated-record (type-fun-param instantiated)
        instantiated-app (type-fun-ret instantiated)
        free (free-vars fun-ty)
        env-vars (map-insert (map-new) 2000 1)
        generalized (generalize fun-ty env-vars)
        generalized-vars (scheme-vars generalized)]
    (do
      (print (type-tag instantiated-record))
      (print (type-tag (type-record-field-type instantiated-record 18000)))
      (print (type-name (type-record-field-type instantiated-record 18000)))
      (print (type-tag (type-record-field-type instantiated-record 18064)))
      (print (type-name (type-record-field-type instantiated-record 18064)))
      (print (vector-length (scheme-vars scheme)))
      (print (type-app-arg-count instantiated-app))
      (print (type-name (type-app-arg instantiated-app 0)))
      (print (type-name (type-app-arg instantiated-app 64)))
      (print (vector-length free))
      (print (vector-length generalized-vars))
      (print (vector-get generalized-vars 0))
      (print (vector-get generalized-vars 63))
      0)))
"#,
        bound_expr = bound_expr,
        args_expr = args_expr,
        record_expr = record_expr,
    );
    let combined = format!("{}\n{}\n{}", type_ls, type_scheme_ls, harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "4", "2", "1000", "2", "1064", "65", "65", "1000", "1064", "65", "64", "2001", "2064"
        ],
        "65 bound vars/record fields/app args は chunk 境界を越えて TypeScheme の結果を保持するべき"
    );
}
