use super::support::*;

/// selfhost TypeInfer.ls テスト: infer-defn は自分自身の参照を body 推論前に束縛する
#[test]
fn test_e2e_selfhost_typeinfer_defn_self_recursive_call_uses_placeholder() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 140
        param-hash 141
        callee-node (make-var name-hash)
        param-node (make-var param-hash)
        body-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 5)
                callee-node)
              1)
            param-node)
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 20)
                  name-hash)
                1)
              param-hash)
            body-node)
        result (infer-defn defn-node env counter)
        ty (result-type result)]
    (do
      (print (result-failed result))
      (print (ty-tag ty))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "3"],
        "自己再帰 defn は未定義変数ではなく関数型として推論されるべき"
    );
}

/// selfhost TypeInfer.ls テスト: <= は Rust host と同じ Int 比較の型を持つ
#[test]
fn test_e2e_selfhost_typeinfer_builtin_less_equal_has_int_comparison_type() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        less-equal-node (make-var 1921)
        left-node (vector-push (vector-push (vector-new 2) 1) 1)
        right-node (vector-push (vector-push (vector-new 2) 1) 2)
        comparison-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  less-equal-node)
                2)
              left-node)
            right-node)
        result (infer-expr comparison-node env (subst-new) counter)
        ty (result-type result)]
    (do
      (print (result-failed result))
      (print (ty-tag ty))
      (print (ty-name ty))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "200"],
        "<= は Int -> Int -> Bool として推論されるべき"
    );
}

/// selfhost TypeInfer.ls テスト: program 推論は相互再帰の全 defn を先行登録する
#[test]
fn test_e2e_selfhost_typeinfer_program_analysis_predeclares_mutual_recursion() {
    let harness = r#"
(defn make-one-param-call-defn [name-hash param-hash callee-hash]
  (let [callee-node (make-var callee-hash)
        param-node (make-var param-hash)
        body-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 5)
                callee-node)
              1)
            param-node)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push (vector-new 5) 20)
            name-hash)
          1)
        param-hash)
      body-node)))

(defn main []
  (let [even-hash 140
        odd-hash 142
        even-node (make-one-param-call-defn even-hash 141 odd-hash)
        odd-node (make-one-param-call-defn odd-hash 143 even-hash)
        program (vector-push (vector-push (vector-new 2) even-node) odd-node)
        analysis (infer-program-analysis program)
        first-ty (infer-program-analysis-type analysis)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-code analysis))
      (print (ty-tag first-ty))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "3"],
        "program 推論は相互再帰を未定義変数にせず、関数型を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_program_analysis_uses_bounded_chunks() {
    let type_infer = selfhost_module("TypeInfer.ls");

    assert!(
        type_infer.contains("typeinfer-program-analysis-step-64-loop-bounded")
            && type_infer.contains("typeinfer-program-analysis-rooted-v3")
            && type_infer.contains("typeinfer-program-analysis-step-state"),
        "program analysis は bounded helper と rooted continuation へ分離するべき"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_large_program_analysis_preserves_results() {
    let mut program_expr = "(vector-new 0)".to_string();
    for idx in 0..65 {
        let defn_name = 5000 + idx;
        let defn_expr = format!(
            "(vector-push (vector-push (vector-push (vector-push (vector-new 4) 20) {}) 0) (make-lit-int {}))",
            defn_name, idx
        );
        program_expr = format!("(vector-push {} {})", program_expr, defn_expr);
    }

    let harness = format!(
        r#"
(defn main []
  (let [analysis (infer-program-analysis {program_expr})
        first-ty (infer-program-analysis-type analysis)
        failure-kinds (infer-program-analysis-failure-kinds analysis)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-code analysis))
      (print (infer-program-analysis-first-error-index analysis))
      (print (ty-tag first-ty))
      (print (ty-name first-ty))
      (print (vector-length failure-kinds))
      (print (vector-get failure-kinds 64))
      0)))
"#,
        program_expr = program_expr,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "-1", "1", "200", "65", "0"],
        "65件の program analysis は chunk 境界を越えて全 defn の結果を保持するべき"
    );
}
