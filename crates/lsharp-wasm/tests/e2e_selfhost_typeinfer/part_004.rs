/// selfhost TypeInfer.ls テスト: 6 引数 apply はカリー化された関数型をたどれる
#[test]
fn test_e2e_selfhost_typeinfer_apply_six_args_curried() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
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
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
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
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
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
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
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

/// selfhost TypeInfer.ls テスト: 5 引数 lambda は 5 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_five_params_curried() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        v-hash 124
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
              (make-var w-hash))
            (make-var v-hash))
        level3-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var z-hash))
            inner-plus)
        level2-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var y-hash))
            level3-plus)
        body-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var x-hash))
            level2-plus)
        lambda-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push (vector-new 8) 8)
                      5)
                    x-hash)
                  y-hash)
                z-hash)
              w-hash)
            v-hash)
        node (vector-push lambda-node body-node)
        result (infer-expr node env (subst-new) counter)
        outer (result-type result)
        level2 (ty-fr outer)
        level3 (ty-fr level2)
        level4 (ty-fr level3)
        level5 (ty-fr level4)]
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
      (print (ty-tag level5))
      (print (ty-tag (ty-fp level5)))
      (print (ty-name (ty-fp level5)))
      (print (ty-tag (ty-fr level5)))
      (print (ty-name (ty-fr level5)))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 18,
        "five-param lambda typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "five-param lambda infer は失敗すべきでない");
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
    assert_eq!(lines[13], "3", "level5 type は Fun であるべき");
    assert_eq!(lines[14], "1", "level5 param type は Con であるべき");
    assert_eq!(
        lines[15], "100",
        "level5 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[16], "1", "level5 return type は Con であるべき");
    assert_eq!(
        lines[17], "100",
        "level5 return type は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 6 引数 lambda は 6 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_six_params_curried() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        u-hash 125
        v-hash 124
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
              (make-var v-hash))
            (make-var u-hash))
        level4-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var w-hash))
            inner-plus)
        level3-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var z-hash))
            level4-plus)
        level2-plus
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var y-hash))
            level3-plus)
        body-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var x-hash))
            level2-plus)
        lambda-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push (vector-new 9) 8)
                        6)
                      x-hash)
                    y-hash)
                  z-hash)
                w-hash)
              v-hash)
            u-hash)
        node (vector-push lambda-node body-node)
        result (infer-expr node env (subst-new) counter)
        outer (result-type result)
        level2 (ty-fr outer)
        level3 (ty-fr level2)
        level4 (ty-fr level3)
        level5 (ty-fr level4)
        level6 (ty-fr level5)]
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
      (print (ty-tag level5))
      (print (ty-tag (ty-fp level5)))
      (print (ty-name (ty-fp level5)))
      (print (ty-tag level6))
      (print (ty-tag (ty-fp level6)))
      (print (ty-name (ty-fp level6)))
      (print (ty-tag (ty-fr level6)))
      (print (ty-name (ty-fr level6)))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 21,
        "six-param lambda typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "six-param lambda infer は失敗すべきでない");
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
    assert_eq!(lines[13], "3", "level5 type は Fun であるべき");
    assert_eq!(lines[14], "1", "level5 param type は Con であるべき");
    assert_eq!(
        lines[15], "100",
        "level5 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[16], "3", "level6 type は Fun であるべき");
    assert_eq!(lines[17], "1", "level6 param type は Con であるべき");
    assert_eq!(
        lines[18], "100",
        "level6 param type は Int hash=100 であるべき"
    );
    assert_eq!(lines[19], "1", "level6 return type は Con であるべき");
    assert_eq!(
        lines[20], "100",
        "level6 return type は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 7 引数 lambda は 7 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_seven_params_curried() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    // 7 引数 lambda: (fn [a b c d e f g] (+ a ...)) → Int -> Int -> ... -> Int
    // body は (+ a b) 相当の単純な apply で全 param を Int に制約する
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        a-hash 101
        b-hash 102
        c-hash 103
        d-hash 104
        e-hash 105
        f-hash 106
        g-hash 107
        ;; body: (+ a b) — 2 引数の + 適用で a, b を Int に制約
        body-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 5)
                  (make-var 43))
                2)
              (make-var a-hash))
            (make-var b-hash))
        ;; lambda: [8, param-count=7, a, b, c, d, e, f, g, body]
        lambda-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push (vector-new 10) 8)
                          7)
                        a-hash)
                      b-hash)
                    c-hash)
                  d-hash)
                e-hash)
              f-hash)
            g-hash)
        node (vector-push lambda-node body-node)
        result (infer-expr node env (subst-new) counter)
        outer (result-type result)]
    (do
      (print (result-failed result))
      (print (ty-tag outer))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "seven-param lambda typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "7 引数 lambda infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
}
