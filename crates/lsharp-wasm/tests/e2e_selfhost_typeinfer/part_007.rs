/// selfhost TypeInfer.ls テスト: ast-pat-lit は int/bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_lit_tag() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        int-pat
          (vector-push
            (vector-push (vector-new 2) (ast-pat-lit))
            (make-lit-int 7))
        bool-pat
          (vector-push
            (vector-push (vector-new 2) (ast-pat-lit))
            (make-lit-bool 1))
        int-result (infer-pattern int-pat env (subst-new) counter)
        bool-result (infer-pattern bool-pat env (subst-new) counter)]
    (do
      (print (ty-tag (pat-result-type int-result)))
      (print (ty-name (pat-result-type int-result)))
      (print (ty-tag (pat-result-type bool-result)))
      (print (ty-name (pat-result-type bool-result)))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "match pat-lit infer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "int pat-lit infer の型タグは Con であるべき");
    assert_eq!(lines[1], "100", "int pat-lit infer の型名は Int であるべき");
    assert_eq!(
        lines[2], "1",
        "bool pat-lit infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[3], "200",
        "bool pat-lit infer の型名は Bool であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: ast-pat-lit は unit 型も返せる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_lit_unit_tag() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        unit-pat
          (vector-push
            (vector-push (vector-new 2) (ast-pat-lit))
            (make-lit-unit))
        result (infer-pattern unit-pat env (subst-new) counter)]
    (do
      (print (ty-tag (pat-result-type result)))
      (print (ty-name (pat-result-type result)))
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
        "match pat-lit unit 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "unit pat-lit infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[1], "500",
        "unit pat-lit infer の型名は Unit であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: constructor child の ast-pat-lit も unify できる
#[test]
fn test_e2e_selfhost_typeinfer_match_constructor_child_pat_lit() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        some-hash 800
        ctor-hash 1300
        value-hash 1301
        ctor-ty (mk-fun (mk-int) (mk-con some-hash))
        env1 (type-env-insert env0 ctor-hash (mono ctor-ty))
        env (type-env-insert env1 value-hash (mono (mk-con some-hash)))
        child-pat
          (vector-push
            (vector-push (vector-new 2) (ast-pat-lit))
            (make-lit-int 1))
        pat (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 4) (ast-pat-constructor))
                  ctor-hash)
                1)
              child-pat)
        node (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 5) 10)
                     (make-var value-hash))
                   1)
                 pat)
               (make-lit-bool 1))
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
        "match constructor child pat-lit 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "match constructor child pat-lit infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "match constructor child pat-lit infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "200",
        "match constructor child pat-lit infer の型名は Bool であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: record child の ast-pat-lit も unify できる
#[test]
fn test_e2e_selfhost_typeinfer_match_record_child_pat_lit() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        point-hash 700
        point-var 1001
        field-x 120
        point-ty
          (type-record-add-field
            (make-type-record point-hash)
            field-x
            (mk-bool))
        env (type-env-insert env0 point-var (mono point-ty))
        child-pat
          (vector-push
            (vector-push (vector-new 2) (ast-pat-lit))
            (make-lit-bool 1))
        pat (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 4) (ast-pat-recordpat))
                  1)
                field-x)
              child-pat)
        node (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 5) 10)
                     (make-var point-var))
                   1)
                 pat)
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
        "match record child pat-lit 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "match record child pat-lit infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "match record child pat-lit infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "100",
        "match record child pat-lit infer の型名は Int であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: constructor child の unit ast-pat-lit も unify できる
#[test]
fn test_e2e_selfhost_typeinfer_match_constructor_child_pat_unit_lit() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        some-hash 800
        ctor-hash 1300
        value-hash 1301
        ctor-ty (mk-fun (mk-unit) (mk-con some-hash))
        env1 (type-env-insert env0 ctor-hash (mono ctor-ty))
        env (type-env-insert env1 value-hash (mono (mk-con some-hash)))
        child-pat
          (vector-push
            (vector-push (vector-new 2) (ast-pat-lit))
            (make-lit-unit))
        pat (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 4) (ast-pat-constructor))
                  ctor-hash)
                1)
              child-pat)
        node (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 5) 10)
                     (make-var value-hash))
                   1)
                 pat)
               (make-lit-bool 1))
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
        "match constructor child pat-unit 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "match constructor child pat-unit infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "match constructor child pat-unit infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "200",
        "match constructor child pat-unit infer の型名は Bool であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 変数束縛の型推論
#[test]
fn test_e2e_selfhost_typeinfer_variable() {
    // let 束縛の型推論が正しく動作することを検証
    // 期待値: x: Int が推論され、print で出力可能
    let source = r#"
(module Main)
(defn main [] (let [x 42] (print x)))
"#;
    let result = compile_and_run(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost TypeInfer.ls テスト: 関数の型推論 (arrow type)
#[test]
fn test_e2e_selfhost_typeinfer_function() {
    // 関数定義の型推論 (Int -> Int) が動作することを検証
    // 期待値: f: Int -> Int が推論され、適用結果が正しい
    let source = r#"
(module Main)
(defn f [x] (+ x 1))
(defn main [] (print (f 41)))
"#;
    let result = compile_and_run(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost TypeInfer.ls テスト: let 多相 (let-polymorphism)
#[test]
fn test_e2e_selfhost_typeinfer_let_poly() {
    // let-polymorphism が動作することを検証
    // 期待値: id が Int にも Bool にも適用可能
    let source = r#"
(module Main)
(defn id [x] x)
(defn main [] (do (print (id 42)) (print (id true))))
"#;
    let result = compile_and_run(source);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "42");
    assert_eq!(lines[1], "1");
}

/// selfhost TypeInfer.ls テスト: 型の単一化 (unification)
#[test]
fn test_e2e_selfhost_typeinfer_unification() {
    // 型変数の単一化が動作することを検証
    // 期待値: 高階関数 apply の型が正しく推論される
    let source = r#"
(module Main)
(defn apply [f x] (f x))
(defn inc [n] (+ n 1))
(defn main [] (print (apply inc 41)))
"#;
    typecheck_only_expanded(source);
}

/// selfhost TypeInfer.ls テスト: if 式の型推論
#[test]
fn test_e2e_selfhost_typeinfer_if_expr() {
    // if 式の型推論 (条件=Bool, 両枝=同一型) の検証
    // 期待値: if の型チェックが成功し、正しい値が返る
    let source = r#"
(module Main)
(defn main [] (print (if true 42 0)))
"#;
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost TypeInfer.ls テスト: パターンマッチの型推論
#[test]
fn test_e2e_selfhost_typeinfer_pattern_match() {
    // パターンマッチの最小型推論が動作することを検証
    // 期待値: match 式の各腕の型が一致し、文字列結果を安定に利用できることをチェック
    let source = r#"
(module Main)
(defn main []
  (let [x 1]
    (let [result (match x
      [1 "one"]
      [_ "other"])]
      (print (if (string-eq result "one") 1 0)))))
"#;
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "1");
}
