use super::support::*;

/// selfhost TypeInfer.ls テスト: 3-step do! -> let! -> return は後段 binder を渡せる
#[test]
fn test_e2e_selfhost_typeinfer_computation_do_bang_let_bang_return_bool() {
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

/// selfhost TypeInfer.ls テスト: do 9 式は最後の式の型を返す
#[test]
fn test_e2e_selfhost_typeinfer_do_nine_exprs_last_bool() {
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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
