/// selfhost TypeInfer.ls テスト: record update base failure でも infinite error code を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_record_update_propagates_infinite_code() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        outer-hash 120
        outer-ty (fresh-type-var counter)
        env (type-env-insert env0 outer-hash (mono outer-ty))
        x-node (make-var outer-hash)
        apply-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 5)
                x-node)
              1)
            x-node)
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 14)
                  apply-node)
                1)
              121)
            (make-lit-int 1))
        result (infer-expr node env (subst-new) counter)]
    (do
      (print (result-failed result))
      (print (result-error-code result))
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
        "record update infinite error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "自己適用 record update infer は失敗すべき");
    assert_eq!(
        lines[1], "5",
        "record update base failure の infinite error code は E0005 を維持すべき"
    );
}

/// selfhost TypeInfer.ls テスト: record literal は minimal Con type を返せる
#[test]
fn test_e2e_selfhost_typeinfer_record_literal() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        point-hash 700
        field-x 120
        node (vector-push
               (vector-push
                 (vector-push
                    (vector-push
                      (vector-push (vector-new 5) 12)
                      point-hash)
                    1)
                  field-x)
                (make-lit-int 10))
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
        "record literal typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "record literal infer は失敗すべきでない");
    assert_eq!(
        lines[1], "1",
        "record literal infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "700",
        "record literal infer の型名は Point hash=700 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: record update は base 式の型を維持できる
#[test]
fn test_e2e_selfhost_typeinfer_record_update() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn mk-point-type []
  (type-record-add-field (make-type-record 700) 120 (mk-int)))

(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        point-hash 700
        point-var 1001
        field-x 120
        env (type-env-insert env0 point-var (mono (mk-point-type)))
        node (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 5) 14)
                     (make-var point-var))
                   1)
                 field-x)
               (make-lit-int 42))
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
        "record update typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "record update infer は失敗すべきでない");
    assert_eq!(
        lines[1], "4",
        "record update infer の型タグは Record であるべき"
    );
    assert_eq!(
        lines[2], "700",
        "record update infer の型名は Point hash=700 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: computation expression の最小型推論
#[test]
fn test_e2e_selfhost_typeinfer_computation_expr() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        builder-hash 900
        x-hash 1200
        return-only
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 6) 15)
                  builder-hash)
                1)
              (computation-step-return))
            0)
        return-only-node (vector-push return-only (make-lit-int 42))
        bind-and-return
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push (vector-new 9) 15)
                        builder-hash)
                      2)
                    (computation-step-let-bang))
                  x-hash)
                (make-lit-int 10))
              (computation-step-return))
            0)
        bind-and-return-node
          (vector-push bind-and-return (make-var x-hash))
        result1 (infer-expr return-only-node env (subst-new) counter)
        result2 (infer-expr bind-and-return-node env (subst-new) counter)]
    (do
      (print (result-failed result1))
      (print (ty-tag (result-type result1)))
      (print (ty-name (result-type result1)))
      (print (result-failed result2))
      (print (ty-tag (result-type result2)))
      (print (ty-name (result-type result2)))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "computation typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "return-only computation infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "return-only computation の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "100",
        "return-only computation の型名は Int hash=100 であるべき"
    );
    assert_eq!(lines[3], "0", "let! computation infer は失敗すべきでない");
    assert_eq!(lines[4], "1", "let! computation の型タグは Con であるべき");
    assert_eq!(
        lines[5], "100",
        "let! computation の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: single-step computation は最後の式型へ委譲できる
#[test]
fn test_e2e_selfhost_typeinfer_computation_single_step_bool() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        builder-hash 900
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 6) 15)
                  builder-hash)
                1)
              (computation-step-return))
            0)
        comp-node (vector-push node (make-lit-bool 1))
        result (infer-expr comp-node env (subst-new) counter)]
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
        "single-step computation typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "single-step computation infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "single-step computation の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "200",
        "single-step computation の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 2-step let! computation は binder を最後の式へ渡せる
#[test]
fn test_e2e_selfhost_typeinfer_computation_let_bang_bool_binder() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        builder-hash 901
        x-hash 120
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push (vector-new 9) 15)
                        builder-hash)
                      2)
                    (computation-step-let-bang))
                  x-hash)
                (make-lit-bool 1))
              (computation-step-return))
            0)
        comp-node (vector-push node (make-var x-hash))
        result (infer-expr comp-node env (subst-new) counter)]
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
        "let! bool computation typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "let! bool computation infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "let! bool computation の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "200",
        "let! bool computation の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 2-step do! computation は最後の式型へ委譲できる
#[test]
fn test_e2e_selfhost_typeinfer_computation_do_bang_bool_return() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        builder-hash 902
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push (vector-new 9) 15)
                        builder-hash)
                      2)
                    (computation-step-do-bang))
                  0)
                (make-lit-int 1))
              (computation-step-return))
            0)
        comp-node (vector-push node (make-lit-bool 1))
        result (infer-expr comp-node env (subst-new) counter)]
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
        "do! bool computation typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "do! bool computation infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "do! bool computation の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "200",
        "do! bool computation の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 3-step let! -> do! -> return は binder を維持できる
#[test]
fn test_e2e_selfhost_typeinfer_computation_let_bang_do_bang_return_bool() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        builder-hash 903
        x-hash 120
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
                              (vector-push (vector-new 12) 15)
                              builder-hash)
                            3)
                          (computation-step-let-bang))
                        x-hash)
                      (make-lit-bool 1))
                    (computation-step-do-bang))
                  0)
                (make-lit-int 1))
              (computation-step-return))
            0)
        comp-node (vector-push node (make-var x-hash))
        result (infer-expr comp-node env (subst-new) counter)]
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
        "3-step let! do! computation typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "3-step let! do! computation infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "3-step let! do! computation の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "200",
        "3-step let! do! computation の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 3-step do! -> let! -> return は後段 binder を渡せる
#[test]
fn test_e2e_selfhost_typeinfer_computation_do_bang_let_bang_return_bool() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        builder-hash 904
        x-hash 120
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
                              (vector-push (vector-new 12) 15)
                              builder-hash)
                            3)
                          (computation-step-do-bang))
                        0)
                      (make-lit-int 1))
                    (computation-step-let-bang))
                  x-hash)
                (make-lit-bool 1))
              (computation-step-return))
            0)
        comp-node (vector-push node (make-var x-hash))
        result (infer-expr comp-node env (subst-new) counter)]
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
        "3-step do! let! computation typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "3-step do! let! computation infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "3-step do! let! computation の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "200",
        "3-step do! let! computation の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 6 式 do ブロックは最後の式型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_six_exprs_last_bool() {
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
                      (vector-push (vector-new 8) 9)
                      6)
                    (make-lit-int 1))
                  (make-lit-int 2))
                (make-lit-int 3))
              (make-lit-int 4))
            (make-lit-int 5))
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
        "do 6 exprs typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "do 6 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 6 exprs の型タグは Con であるべき");
    assert_eq!(
        lines[2], "200",
        "do 6 exprs の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 7 式 do ブロックは最後の式型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_seven_exprs_last_bool() {
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
                        (vector-push (vector-new 9) 9)
                        7)
                      (make-lit-int 1))
                    (make-lit-int 2))
                  (make-lit-int 3))
                (make-lit-int 4))
              (make-lit-int 5))
            (make-lit-int 6))
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
        "do 7 exprs typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "do 7 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 7 exprs の型タグは Con であるべき");
    assert_eq!(
        lines[2], "200",
        "do 7 exprs の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 8 式 do ブロックは最後の式型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_eight_exprs_last_bool() {
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
                          (vector-push (vector-new 10) 9)
                          8)
                        (make-lit-int 1))
                      (make-lit-int 2))
                    (make-lit-int 3))
                  (make-lit-int 4))
                (make-lit-int 5))
              (make-lit-int 6))
            (make-lit-int 7))
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
        "do 8 exprs typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "do 8 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 8 exprs の型タグは Con であるべき");
    assert_eq!(
        lines[2], "200",
        "do 8 exprs の型名は Bool hash=200 であるべき"
    );
}
