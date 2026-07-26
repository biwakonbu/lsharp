/// selfhost TypeInfer.ls テスト: record type が分かる field access は実フィールド型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_field_access() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        point-hash 700
        point-var 1001
        field-x 120
        field-y 121
        point-ty
          (type-record-add-field
            (type-record-add-field
              (make-type-record point-hash)
              field-x
              (mk-int))
            field-y
            (mk-bool))
        env (type-env-insert env0 point-var (mono point-ty))
        node (make-fieldaccess (make-var point-var) field-x)
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
        "fieldaccess typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "fieldaccess infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "fieldaccess infer の型タグは Con であるべき");
    assert_eq!(
        lines[2], "100",
        "fieldaccess infer の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: record type が分からない field access は fresh var fallback を返せる
#[test]
fn test_e2e_selfhost_typeinfer_field_access_fallback_var() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        point-hash 700
        point-var 1001
        field-x 120
        env (type-env-insert env0 point-var (mono (mk-con point-hash)))
        node (make-fieldaccess (make-var point-var) field-x)
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
        "fieldaccess fallback typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "fieldaccess fallback infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "2",
        "fieldaccess fallback infer の型タグは fresh Var であるべき"
    );
    assert_eq!(
        lines[2], "1000",
        "fieldaccess fallback infer の型変数 ID は 1000 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: record literal に対する field access は実フィールド型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_field_access_on_record_literal() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        point-hash 700
        field-x 120
        record-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 12)
                  point-hash)
                1)
              field-x)
            (make-lit-int 42))
        node (make-fieldaccess record-node field-x)
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
        "record literal fieldaccess typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "record literal fieldaccess infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "record literal fieldaccess infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "100",
        "record literal fieldaccess infer の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 2-field record literal の後続 field access も実フィールド型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_field_access_on_record_literal_second_field() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        point-hash 700
        field-x 120
        field-y 121
        record-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push (vector-new 7) 12)
                      point-hash)
                    2)
                  field-x)
                (make-lit-int 42))
              field-y)
            (make-lit-bool 1))
        node (make-fieldaccess record-node field-y)
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
        "record literal second-field fieldaccess typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "record literal second-field fieldaccess infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "record literal second-field fieldaccess infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "200",
        "record literal second-field fieldaccess infer の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: quote は内側式の最小型推論へ委譲できる
#[test]
fn test_e2e_selfhost_typeinfer_quote_expr() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 3, "quote typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "quote infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "quote infer の型タグは Con であるべき");
    assert_eq!(
        lines[2], "100",
        "quote infer の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: unquote は内側 var の型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_unquote_expr() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "unquote typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "unquote infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "unquote infer の型タグは Con であるべき");
    assert_eq!(
        lines[2], "200",
        "unquote infer の型名は Bool hash=200 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: unquote-splice は内側式の型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_unquote_splice_expr() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
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
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "match binder typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "match binder infer は失敗すべきでない");
    assert_eq!(
        lines[1], "1",
        "match binder infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "100",
        "match binder infer の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: ast-pat-var でも match binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_var_tag_binder() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "match pat-var binder typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "match pat-var binder infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "match pat-var binder infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "100",
        "match pat-var binder infer の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: match の record pattern binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_record_pattern_binder() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "match record binder 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "match record binder infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "2",
        "match record binder infer の型タグは Var であるべき"
    );
    assert_eq!(
        lines[2], "1001",
        "match record binder infer の型変数 ID は 1001 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: match の constructor pattern binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_constructor_pattern_binder() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "match constructor binder 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "match constructor binder infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "match constructor binder infer の型タグは Con であるべき"
    );
    assert_eq!(
        lines[2], "100",
        "match constructor binder infer の型名は Int であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: ast-pat-recordpat でも match binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_record_tag_binder() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "match pat-record binder 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "match pat-record binder infer は失敗すべきでない"
    );
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
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

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

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls, harness
    );
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
