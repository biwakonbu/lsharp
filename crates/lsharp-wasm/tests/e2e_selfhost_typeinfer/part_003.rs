/// selfhost TypeInfer.ls テスト: do 9 式は最後の式の型を返す
#[test]
fn test_e2e_selfhost_typeinfer_do_nine_exprs_last_bool() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
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
                            (vector-push (vector-new 11) 9)
                            9)
                          (make-lit-int 1))
                        (make-lit-int 2))
                      (make-lit-int 3))
                    (make-lit-int 4))
                  (make-lit-int 5))
                (make-lit-int 6))
              (make-lit-int 7))
            (make-lit-int 8))
        do-node (vector-push node (make-lit-bool 1))
        result (infer-expr do-node env (subst-new) counter)]
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
        "do 9 exprs typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "do 9 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 9 exprs の型タグは Con であるべき");
    assert_eq!(
        lines[2], "200",
        "do 9 exprs の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: do ブロック 10 式は最後の Bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_ten_exprs_last_bool() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        ;; do node: [9, expr-count=10, e1, e2, ..., e10]
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
                            (vector-push
                              (vector-push (vector-new 12) 9)
                              10)
                            (make-lit-int 1))
                          (make-lit-int 2))
                        (make-lit-int 3))
                      (make-lit-int 4))
                    (make-lit-int 5))
                  (make-lit-int 6))
                (make-lit-int 7))
              (make-lit-int 8))
            (make-lit-int 9))
        do-node (vector-push node (make-lit-bool 1))
        result (infer-expr do-node env (subst-new) counter)]
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
        "do 10 exprs typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "do 10 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 10 exprs の型タグは Con であるべき");
    assert_eq!(
        lines[2], "200",
        "do 10 exprs の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: do ブロック 11 式は最後の Bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_eleven_exprs_last_bool() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        ;; do node: [9, expr-count=11, e1, e2, ..., e11]
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
                            (vector-push
                              (vector-push
                                (vector-push (vector-new 13) 9)
                                11)
                              (make-lit-int 1))
                            (make-lit-int 2))
                          (make-lit-int 3))
                        (make-lit-int 4))
                      (make-lit-int 5))
                    (make-lit-int 6))
                  (make-lit-int 7))
                (make-lit-int 8))
              (make-lit-int 9))
            (make-lit-int 10))
        do-node (vector-push node (make-lit-bool 1))
        result (infer-expr do-node env (subst-new) counter)]
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
        "do 11 exprs typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "do 11 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 11 exprs の型タグは Con であるべき");
    assert_eq!(
        lines[2], "200",
        "do 11 exprs の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: do ブロック 12 式は最後の Bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_twelve_exprs_last_bool() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        ;; do node: [9, expr-count=12, e1, e2, ..., e12]
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
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push (vector-new 14) 9)
                                  12)
                                (make-lit-int 1))
                              (make-lit-int 2))
                            (make-lit-int 3))
                          (make-lit-int 4))
                        (make-lit-int 5))
                      (make-lit-int 6))
                    (make-lit-int 7))
                  (make-lit-int 8))
                (make-lit-int 9))
              (make-lit-int 10))
            (make-lit-int 11))
        do-node (vector-push node (make-lit-bool 1))
        result (infer-expr do-node env (subst-new) counter)]
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
        "do 12 exprs typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "do 12 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 12 exprs の型タグは Con であるべき");
    assert_eq!(
        lines[2], "200",
        "do 12 exprs の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: do ブロック 13 式は最後の Bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_thirteen_exprs_last_bool() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        ;; do node: [9, expr-count=13, e1, e2, ..., e13]
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
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push (vector-new 15) 9)
                                    13)
                                  (make-lit-int 1))
                                (make-lit-int 2))
                              (make-lit-int 3))
                            (make-lit-int 4))
                          (make-lit-int 5))
                        (make-lit-int 6))
                      (make-lit-int 7))
                    (make-lit-int 8))
                  (make-lit-int 9))
                (make-lit-int 10))
              (make-lit-int 11))
            (make-lit-int 12))
        do-node (vector-push node (make-lit-bool 1))
        result (infer-expr do-node env (subst-new) counter)]
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
        "do 13 exprs typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "do 13 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 13 exprs の型タグは Con であるべき");
    assert_eq!(
        lines[2], "200",
        "do 13 exprs の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: do ブロック 14 式は最後の Bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_fourteen_exprs_last_bool() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        ;; do node: [9, expr-count=14, e1, e2, ..., e14]
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
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push (vector-new 16) 9)
                                      14)
                                    (make-lit-int 1))
                                  (make-lit-int 2))
                                (make-lit-int 3))
                              (make-lit-int 4))
                            (make-lit-int 5))
                          (make-lit-int 6))
                        (make-lit-int 7))
                      (make-lit-int 8))
                    (make-lit-int 9))
                  (make-lit-int 10))
                (make-lit-int 11))
              (make-lit-int 12))
            (make-lit-int 13))
        do-node (vector-push node (make-lit-bool 1))
        result (infer-expr do-node env (subst-new) counter)]
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
        "do 14 exprs typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "do 14 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 14 exprs の型タグは Con であるべき");
    assert_eq!(
        lines[2], "200",
        "do 14 exprs の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 2 引数 lambda はカリー化された関数型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_two_params_curried() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
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
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
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
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
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
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
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
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
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
