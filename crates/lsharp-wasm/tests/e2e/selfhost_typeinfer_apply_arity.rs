use super::support::*;

fn curried_int_type(arity: usize) -> String {
    let mut ty = "(mk-int)".to_string();
    for _ in 0..arity {
        ty = format!("(mk-fun (mk-int) {ty})");
    }
    ty
}

fn apply_node(arity: usize, function_hash: i64) -> String {
    let mut node = format!("(vector-new {})", arity + 3);
    for value in [
        "5".to_string(),
        format!("(make-var {function_hash})"),
        arity.to_string(),
    ] {
        node = format!("(vector-push {node} {value})");
    }
    for index in 0..arity {
        node = format!("(vector-push {node} (make-lit-int {index}))");
    }
    node
}

#[test]
fn test_e2e_selfhost_typeinfer_apply_arity_uses_bounded_limit() {
    let source = selfhost_module("TypeInferApply.ls");

    assert!(
        source.contains("(if (<= argc 64)")
            && !source.contains("(if (<= argc 7)")
            && source.contains("infer-apply-args-step-64-loop-bounded"),
        "8-64 引数 apply は既存の64要素bounded scanへ到達させるべき"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_apply_accepts_eight_and_sixty_four_args() {
    let f8_hash = 136;
    let f64_hash = 137;
    let f8_type = curried_int_type(8);
    let f64_type = curried_int_type(64);
    let node8 = apply_node(8, f8_hash);
    let node64 = apply_node(64, f64_hash);
    let node65 = apply_node(65, 138);
    let harness = format!(
        r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        int-ty (mk-int)
        env8 (type-env-insert env0 {f8_hash} (mono {f8_type}))
        env64 (type-env-insert env8 {f64_hash} (mono {f64_type}))
        result8 (infer-expr {node8} env64 (subst-new) counter)
        result64 (infer-expr {node64} env64 (subst-new) counter)
        result65 (infer-expr {node65} env64 (subst-new) counter)]
    (do
      (print (result-failed result8))
      (print (ty-tag (result-type result8)))
      (print (ty-name (result-type result8)))
      (print (result-failed result64))
      (print (ty-tag (result-type result64)))
      (print (ty-name (result-type result64)))
      (print (result-failed result65))
      (print (result-error-code result65))
      0)))
"#,
        f8_hash = f8_hash,
        f64_hash = f64_hash,
        f8_type = f8_type,
        f64_type = f64_type,
        node8 = node8,
        node64 = node64,
        node65 = node65,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "100", "0", "1", "100", "1", "6"],
        "8/64 引数は成功し、65 引数は明示的に拒否するべき"
    );
}
