use super::support::*;


/// selfhost TypeInfer.ls テスト: quote は内側式の最小型推論へ委譲できる
#[test]
fn test_e2e_selfhost_typeinfer_quote_expr() {

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        node (make-quote (make-lit-int 42))
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

    assert!(lines.len() >= 3, "quote typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "quote infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "quote infer の型タグは Con であるべき");
    assert_eq!(lines[2], "100", "quote infer の型名は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: unquote は内側 var の型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_unquote_expr() {

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        x-hash 1700
        env (type-env-insert env0 x-hash (mono (mk-bool)))
        node (make-unquote (make-var x-hash))
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

    assert!(lines.len() >= 3, "unquote typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "unquote infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "unquote infer の型タグは Con であるべき");
    assert_eq!(lines[2], "200", "unquote infer の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: unquote-splice は内側式の型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_unquote_splice_expr() {

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        node (make-unquote-splice (make-lit-bool 1))
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
        "unquote-splice typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "unquote-splice infer は失敗すべきでない");
    assert_eq!(
        lines[1], "1",
        "unquote-splice infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "200",
        "unquote-splice infer の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: match の var pattern binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_var_binder() {

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        x-hash 1200
        node (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 5) 10)
                     (make-lit-int 1))
                   1)
                 (make-var x-hash))
               (make-var x-hash))
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

    assert!(lines.len() >= 3, "match binder typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "match binder infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "match binder infer の型タグは Con であるべき");
    assert_eq!(lines[2], "100", "match binder infer の型名は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: ast-pat-var でも match binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_var_tag_binder() {

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        x-hash 1201
        pat (vector-push (vector-push (vector-new 2) (ast-pat-var)) x-hash)
        node (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 5) 10)
                     (make-lit-int 1))
                   1)
                 pat)
               (make-var x-hash))
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
        "match pat-var binder typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "match pat-var binder infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "match pat-var binder infer の型タグは Con であるべき");
    assert_eq!(
        lines[2], "100",
        "match pat-var binder infer の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: match の record pattern binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_record_pattern_binder() {

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        point-hash 700
        point-var 1001
        field-x 120
        x-hash 1200
        env (type-env-insert env0 point-var (mono (mk-con point-hash)))
        pat (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 4) 12)
                  1)
                field-x)
              (make-var x-hash))
        node (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 5) 10)
                     (make-var point-var))
                   1)
                 pat)
               (make-var x-hash))
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

    assert!(lines.len() >= 3, "match record binder 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "match record binder infer は失敗すべきでない");
    assert_eq!(lines[1], "2", "match record binder infer の型タグは Var であるべき");
    assert_eq!(lines[2], "1001", "match record binder infer の型変数 ID は 1001 であるべき");
}

/// selfhost TypeInfer.ls テスト: match の constructor pattern binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_constructor_pattern_binder() {

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        some-hash 800
        ctor-hash 1300
        value-hash 1301
        x-hash 1200
        ctor-ty (mk-fun (mk-int) (mk-con some-hash))
        env1 (type-env-insert env0 ctor-hash (mono ctor-ty))
        env (type-env-insert env1 value-hash (mono (mk-con some-hash)))
        pat (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 4) 11)
                  ctor-hash)
                1)
              (make-var x-hash))
        node (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 5) 10)
                     (make-var value-hash))
                   1)
                 pat)
               (make-var x-hash))
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

    assert!(lines.len() >= 3, "match constructor binder 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "match constructor binder infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "match constructor binder infer の型タグは Con であるべき");
    assert_eq!(lines[2], "100", "match constructor binder infer の型名は Int であるべき");
}

/// selfhost TypeInfer.ls テスト: ast-pat-recordpat でも match binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_record_tag_binder() {

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        point-hash 700
        point-var 1001
        field-x 120
        x-hash 1200
        env (type-env-insert env0 point-var (mono (mk-con point-hash)))
        child-pat (vector-push (vector-push (vector-new 2) (ast-pat-var)) x-hash)
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
               (make-var x-hash))
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
        "match pat-record binder 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "match pat-record binder infer は失敗すべきでない");
    assert_eq!(
        lines[1], "2",
        "match pat-record binder infer の型タグは Var であるべき"
    );
    assert_eq!(
        lines[2], "1001",
        "match pat-record binder infer の型変数 ID は 1001 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: ast-pat-constructor でも match binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_constructor_tag_binder() {

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        some-hash 800
        ctor-hash 1300
        value-hash 1301
        x-hash 1200
        ctor-ty (mk-fun (mk-int) (mk-con some-hash))
        env1 (type-env-insert env0 ctor-hash (mono ctor-ty))
        env (type-env-insert env1 value-hash (mono (mk-con some-hash)))
        child-pat (vector-push (vector-push (vector-new 2) (ast-pat-var)) x-hash)
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
               (make-var x-hash))
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
        "match pat-constructor binder 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "match pat-constructor binder infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "match pat-constructor binder infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "100",
        "match pat-constructor binder infer の型名は Int であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: ast-pat-lit は int/bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_lit_tag() {

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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 4, "match pat-lit infer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "int pat-lit infer の型タグは Con であるべき");
    assert_eq!(lines[1], "100", "int pat-lit infer の型名は Int であるべき");
    assert_eq!(lines[2], "1", "bool pat-lit infer の型タグは Con であるべき");
    assert_eq!(lines[3], "200", "bool pat-lit infer の型名は Bool であるべき");
}

/// selfhost TypeInfer.ls テスト: ast-pat-lit は unit 型も返せる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_lit_unit_tag() {

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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "match pat-lit unit 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "unit pat-lit infer の型タグは Con であるべき");
    assert_eq!(lines[1], "500", "unit pat-lit infer の型名は Unit であるべき");
}

/// selfhost TypeInfer.ls テスト: constructor child の ast-pat-lit も unify できる
#[test]
fn test_e2e_selfhost_typeinfer_match_constructor_child_pat_lit() {

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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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
