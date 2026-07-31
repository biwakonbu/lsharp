use super::support::*;

#[test]
fn test_e2e_selfhost_typeinfer_apply_high_arities_use_bounded_rooted_scan() {
    let source = selfhost_module("TypeInferApply.ls");

    assert!(
        source.contains("infer-apply-args-step-64-loop-bounded")
            && source.contains("infer-apply-args-rooted-v3")
            && source.contains("infer-apply-many-final")
            && source.contains("infer-apply-legacy-raw")
            && source.contains("(infer-apply-many-rooted node env subst counter argc)")
            && !source.contains("(if (= argc 7)")
            && !source.contains("(if (= argc 6)")
            && !source.contains("(if (= argc 5)")
            && !source.contains("(if (= argc 4)"),
        "3-7 引数 apply は共通の bounded rooted scan へ集約するべき"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_apply_high_arity_argument_failure_propagates() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        int-ty (mk-int)
        f-hash 135
        f-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty int-ty)))
        env (type-env-insert env0 f-hash (mono f-ty))
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push (vector-new 6) 5)
                    (make-var f-hash))
                  3)
                (make-lit-int 1))
              (make-var 99999))
            (make-lit-int 3))
        result (infer-expr node env (subst-new) counter)]
    (do
      (print (result-failed result))
      (print (result-error-code result))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, ["1", "1"], "3 引数 apply の引数 failure を伝播するべき");
}

/// selfhost TypeInfer.ls テスト: 2 引数 lambda はカリー化された関数型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_two_params_curried() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        x-hash 120
        y-hash 121
        plus-node (make-var 43)
        x-node (make-var x-hash)
        y-node (make-var y-hash)
        apply-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  plus-node)
                2)
              x-node)
            y-node)
        lambda-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 5) 8)
                2)
              x-hash)
            y-hash)
        node (vector-push lambda-node apply-node)
        result (infer-expr node env (subst-new) counter)
        outer (result-type result)
        inner (ty-fr outer)]
    (do
      (print (result-failed result))
      (print (ty-tag outer))
      (print (ty-tag (ty-fp outer)))
      (print (ty-name (ty-fp outer)))
      (print (ty-tag inner))
      (print (ty-tag (ty-fp inner)))
      (print (ty-name (ty-fp inner)))
      (print (ty-tag (ty-fr inner)))
      (print (ty-name (ty-fr inner)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 9,
        "multi-param lambda typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "multi-param lambda infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
    assert_eq!(lines[2], "1", "outer param type は Con であるべき");
    assert_eq!(
        lines[3], "100",
        "outer param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[4], "3", "outer return type も Fun であるべき");
    assert_eq!(lines[5], "1", "inner param type は Con であるべき");
    assert_eq!(
        lines[6], "100",
        "inner param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[7], "1", "inner return type は Con であるべき");
    assert_eq!(
        lines[8], "100",
        "inner return type は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 2 引数 defn はカリー化された関数型になる
#[test]
fn test_e2e_selfhost_typeinfer_defn_two_params_curried() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 122
        x-hash 120
        y-hash 121
        plus-node (make-var 43)
        x-node (make-var x-hash)
        y-node (make-var y-hash)
        apply-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  plus-node)
                2)
              x-node)
            y-node)
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 6) 20)
                  name-hash)
                2)
              x-hash)
            y-hash)
        node (vector-push defn-node apply-node)
        result (infer-defn node env counter)
        outer (result-type result)
        inner (ty-fr outer)]
    (do
      (print (result-failed result))
      (print (ty-tag outer))
      (print (ty-tag (ty-fp outer)))
      (print (ty-name (ty-fp outer)))
      (print (ty-tag inner))
      (print (ty-tag (ty-fp inner)))
      (print (ty-name (ty-fp inner)))
      (print (ty-tag (ty-fr inner)))
      (print (ty-name (ty-fr inner)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 9,
        "multi-param defn typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "multi-param defn infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
    assert_eq!(lines[2], "1", "outer param type は Con であるべき");
    assert_eq!(
        lines[3], "100",
        "outer param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[4], "3", "outer return type も Fun であるべき");
    assert_eq!(lines[5], "1", "inner param type は Con であるべき");
    assert_eq!(
        lines[6], "100",
        "inner param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[7], "1", "inner return type は Con であるべき");
    assert_eq!(
        lines[8], "100",
        "inner return type は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 3 引数 apply はカリー化された関数型をたどれる
#[test]
fn test_e2e_selfhost_typeinfer_apply_three_args_curried() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        int-ty (mk-int)
        f-hash 130
        f-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty int-ty)))
        env (type-env-insert env0 f-hash (mono f-ty))
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push (vector-new 6) 5)
                    (make-var f-hash))
                  3)
                (make-lit-int 1))
              (make-lit-int 2))
            (make-lit-int 3))
        result (infer-expr node env (subst-new) counter)]
    (do
      (print (result-failed result))
      (print (ty-tag (result-type result)))
      (print (ty-name (result-type result)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "apply three args typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "3 引数 apply infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "3 引数 apply の型タグは Con であるべき");
    assert_eq!(
        lines[2], "100",
        "3 引数 apply の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 4 引数 apply はカリー化された関数型をたどれる
#[test]
fn test_e2e_selfhost_typeinfer_apply_four_args_curried() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        int-ty (mk-int)
        f-hash 131
        f-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty int-ty))))
        env (type-env-insert env0 f-hash (mono f-ty))
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push (vector-new 7) 5)
                      (make-var f-hash))
                    4)
                  (make-lit-int 1))
                (make-lit-int 2))
              (make-lit-int 3))
            (make-lit-int 4))
        result (infer-expr node env (subst-new) counter)]
    (do
      (print (result-failed result))
      (print (ty-tag (result-type result)))
      (print (ty-name (result-type result)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "apply four args typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "4 引数 apply infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "4 引数 apply の型タグは Con であるべき");
    assert_eq!(
        lines[2], "100",
        "4 引数 apply の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 5 引数 apply はカリー化された関数型をたどれる
#[test]
fn test_e2e_selfhost_typeinfer_apply_five_args_curried() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        int-ty (mk-int)
        f-hash 132
        f-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty int-ty)))))
        env (type-env-insert env0 f-hash (mono f-ty))
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push (vector-new 8) 5)
                        (make-var f-hash))
                      5)
                    (make-lit-int 1))
                  (make-lit-int 2))
                (make-lit-int 3))
              (make-lit-int 4))
            (make-lit-int 5))
        result (infer-expr node env (subst-new) counter)]
    (do
      (print (result-failed result))
      (print (ty-tag (result-type result)))
      (print (ty-name (result-type result)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "apply five args typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "5 引数 apply infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "5 引数 apply の型タグは Con であるべき");
    assert_eq!(
        lines[2], "100",
        "5 引数 apply の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 6 引数 apply はカリー化された関数型をたどれる
#[test]
fn test_e2e_selfhost_typeinfer_apply_six_args_curried() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        int-ty (mk-int)
        f-hash 133
        f-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty int-ty))))))
        env (type-env-insert env0 f-hash (mono f-ty))
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push (vector-new 9) 5)
                          (make-var f-hash))
                        6)
                      (make-lit-int 1))
                    (make-lit-int 2))
                  (make-lit-int 3))
                (make-lit-int 4))
              (make-lit-int 5))
            (make-lit-int 6))
        result (infer-expr node env (subst-new) counter)]
    (do
      (print (result-failed result))
      (print (ty-tag (result-type result)))
      (print (ty-name (result-type result)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "apply six args typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "6 引数 apply infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "6 引数 apply の型タグは Con であるべき");
    assert_eq!(
        lines[2], "100",
        "6 引数 apply の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 7 引数 apply はカリー化された関数型をたどれる
#[test]
fn test_e2e_selfhost_typeinfer_apply_seven_args_curried() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        int-ty (mk-int)
        f-hash 134
        f-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty (mk-fun int-ty int-ty)))))))
        env (type-env-insert env0 f-hash (mono f-ty))
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push (vector-new 10) 5)
                            (make-var f-hash))
                          7)
                        (make-lit-int 1))
                      (make-lit-int 2))
                    (make-lit-int 3))
                  (make-lit-int 4))
                (make-lit-int 5))
              (make-lit-int 6))
            (make-lit-int 7))
        result (infer-expr node env (subst-new) counter)]
    (do
      (print (result-failed result))
      (print (ty-tag (result-type result)))
      (print (ty-name (result-type result)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "apply seven args typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "7 引数 apply infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "7 引数 apply の型タグは Con であるべき");
    assert_eq!(
        lines[2], "100",
        "7 引数 apply の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 3 引数 lambda は 3 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_three_params_curried() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        x-hash 120
        y-hash 121
        z-hash 122
        inner-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var y-hash))
            (make-var z-hash))
        body-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var x-hash))
            inner-plus)
        lambda-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 6) 8)
                  3)
                x-hash)
              y-hash)
            z-hash)
        node (vector-push lambda-node body-node)
        result (infer-expr node env (subst-new) counter)
        outer (result-type result)
        mid (ty-fr outer)
        inner (ty-fr mid)]
    (do
      (print (result-failed result))
      (print (ty-tag outer))
      (print (ty-tag (ty-fp outer)))
      (print (ty-name (ty-fp outer)))
      (print (ty-tag mid))
      (print (ty-tag (ty-fp mid)))
      (print (ty-name (ty-fp mid)))
      (print (ty-tag inner))
      (print (ty-tag (ty-fp inner)))
      (print (ty-name (ty-fp inner)))
      (print (ty-tag (ty-fr inner)))
      (print (ty-name (ty-fr inner)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 12,
        "three-param lambda typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "three-param lambda infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
    assert_eq!(lines[2], "1", "outer param type は Con であるべき");
    assert_eq!(
        lines[3], "100",
        "outer param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[4], "3", "mid type は Fun であるべき");
    assert_eq!(lines[5], "1", "mid param type は Con であるべき");
    assert_eq!(lines[6], "100", "mid param type は Int hash=100 であるべき");
    assert_eq!(lines[7], "3", "inner type は Fun であるべき");
    assert_eq!(lines[8], "1", "inner param type は Con であるべき");
    assert_eq!(
        lines[9], "100",
        "inner param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[10], "1", "inner return type は Con であるべき");
    assert_eq!(
        lines[11], "100",
        "inner return type は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 4 引数 lambda は 4 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_four_params_curried() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        w-hash 123
        x-hash 120
        y-hash 121
        z-hash 122
        inner-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var z-hash))
            (make-var w-hash))
        mid-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var y-hash))
            inner-plus)
        body-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var x-hash))
            mid-plus)
        lambda-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push (vector-new 7) 8)
                    4)
                  x-hash)
                y-hash)
              z-hash)
            w-hash)
        node (vector-push lambda-node body-node)
        result (infer-expr node env (subst-new) counter)
        outer (result-type result)
        level2 (ty-fr outer)
        level3 (ty-fr level2)
        level4 (ty-fr level3)]
    (do
      (print (result-failed result))
      (print (ty-tag outer))
      (print (ty-tag (ty-fp outer)))
      (print (ty-name (ty-fp outer)))
      (print (ty-tag level2))
      (print (ty-tag (ty-fp level2)))
      (print (ty-name (ty-fp level2)))
      (print (ty-tag level3))
      (print (ty-tag (ty-fp level3)))
      (print (ty-name (ty-fp level3)))
      (print (ty-tag level4))
      (print (ty-tag (ty-fp level4)))
      (print (ty-name (ty-fp level4)))
      (print (ty-tag (ty-fr level4)))
      (print (ty-name (ty-fr level4)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 15,
        "four-param lambda typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "four-param lambda infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
    assert_eq!(lines[2], "1", "outer param type は Con であるべき");
    assert_eq!(
        lines[3], "100",
        "outer param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[4], "3", "level2 type は Fun であるべき");
    assert_eq!(lines[5], "1", "level2 param type は Con であるべき");
    assert_eq!(
        lines[6], "100",
        "level2 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[7], "3", "level3 type は Fun であるべき");
    assert_eq!(lines[8], "1", "level3 param type は Con であるべき");
    assert_eq!(
        lines[9], "100",
        "level3 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[10], "3", "level4 type は Fun であるべき");
    assert_eq!(lines[11], "1", "level4 param type は Con であるべき");
    assert_eq!(
        lines[12], "100",
        "level4 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[13], "1", "level4 return type は Con であるべき");
    assert_eq!(
        lines[14], "100",
        "level4 return type は Int hash=100 であるべき"
    );
}
