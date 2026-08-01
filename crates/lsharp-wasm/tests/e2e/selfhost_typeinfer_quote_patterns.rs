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
    assert_eq!(
        lines[2], "100",
        "quote infer の型名は Int hash=100 であるべき"
    );
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

/// selfhost TypeInfer.ls テスト: 可視 constructor と record schema の field 型で binder を具体化する
#[test]
fn test_e2e_selfhost_typeinfer_record_pattern_uses_declared_field_type() {
    let harness = r#"
(defn main []
  (let [counter0 (make-var-counter)
        aliases (var-counter-alias-env counter0)
        point-hash 700
        field-x 120
        x-hash 1200
        point-ty (type-record-add-field (make-type-record point-hash) field-x (mk-int))
        record-env (map-insert-object-safe (map-new) point-hash (mono point-ty))
        counter (var-counter-with-alias-env-and-record-env counter0 aliases record-env)
        env
          (type-env-insert
            (type-env-new)
            point-hash
            (mono (typeinfer-record-constructor-type point-ty)))
        hidden-env (type-env-new)
        child-pat (vector-push (vector-push (vector-new 2) (ast-pat-var)) x-hash)
        pat (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push (vector-new 5) (ast-pat-recordpat))
                    1)
                  field-x)
                child-pat)
              point-hash)
        result (infer-pattern pat env (subst-new) counter)
        hidden-result (infer-pattern pat hidden-env (subst-new) counter)
        bound (type-env-lookup (pat-result-env result) x-hash)
        bound-ty (apply-subst (result-subst result) (scheme-type bound))]
    (do
      (print (result-failed result))
      (print (ty-tag bound-ty))
      (print (ty-name bound-ty))
      (print (result-failed hidden-result))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "record pattern schema infer 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "record pattern schema infer は失敗すべきでない"
    );
    assert_eq!(
        lines[1], "1",
        "record pattern binder は Int へ具体化されるべき"
    );
    assert_eq!(
        lines[2], "100",
        "record pattern binder の型名は Int hash=100 であるべき"
    );
    assert_eq!(
        lines[3], "1",
        "record schema registry だけでは非可視 record pattern を受理してはいけない"
    );
}

/// selfhost TypeInfer.ls テスト: parametric record pattern の field binder を具体化する
#[test]
fn test_e2e_selfhost_typeinfer_parametric_record_pattern_binds_field_type() {
    let valid_source =
        "(type (Box a) (record (: value a))) (defn unbox [point] : Int (match point [{Box value x} x] [_ 0]))";
    let invalid_source =
        "(type (Box a) (record (: value a))) (defn unbox [point] : Int (match point [{Box value true} true] [_ 0]))";
    let valid_program = lsharp_syntax::parse(valid_source)
        .expect("parametric record pattern fixture は parse できるべき");
    let mut valid_oracle = lsharp_types::infer::Infer::new();
    assert!(
        valid_oracle.infer_program(&valid_program).is_ok(),
        "Rust oracle は parametric record pattern の field binder を受理するべき"
    );
    let invalid_program = lsharp_syntax::parse(invalid_source)
        .expect("invalid parametric record pattern fixture は parse できるべき");
    let mut invalid_oracle = lsharp_types::infer::Infer::new();
    assert!(
        invalid_oracle.infer_program(&invalid_program).is_err(),
        "Rust oracle は parametric record pattern の戻り型不一致を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [valid
          (infer-program-analysis
            (parse-program "{}"))
        invalid
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid))
      (print (infer-program-analysis-diagnostic-count invalid))
      0)))
"#,
        valid_source, invalid_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    assert_eq!(
        output.trim().lines().collect::<Vec<_>>(),
        ["0", "1"],
        "parametric record pattern は field binder を型引数へ接続し、不一致を拒否するべき"
    );
}

/// selfhost TypeInfer.ls テスト: nested parametric record pattern の field 型を伝播する
#[test]
fn test_e2e_selfhost_typeinfer_nested_parametric_record_pattern_binds_field_type() {
    let valid_source = "(type (Box a) (record (: value a))) (type (Outer a) (record (: inner (Box a)))) (defn unbox [point] : Int (match point [{Outer inner {Box value x}} x] [_ 0]))";
    let invalid_source = "(type (Box a) (record (: value a))) (type (Outer a) (record (: inner (Box a)))) (defn unbox [point] : Int (match point [{Outer inner {Box value true}} true] [_ 0]))";
    let valid_program = lsharp_syntax::parse(valid_source)
        .expect("nested parametric record pattern fixture は parse できるべき");
    let mut valid_oracle = lsharp_types::infer::Infer::new();
    assert!(
        valid_oracle.infer_program(&valid_program).is_ok(),
        "Rust oracle は nested parametric record pattern を受理するべき"
    );
    let invalid_program = lsharp_syntax::parse(invalid_source)
        .expect("invalid nested parametric record pattern fixture は parse できるべき");
    let mut invalid_oracle = lsharp_types::infer::Infer::new();
    assert!(
        invalid_oracle.infer_program(&invalid_program).is_err(),
        "Rust oracle は nested parametric record pattern の戻り型不一致を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [valid
          (infer-program-analysis
            (parse-program "{}"))
        invalid
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid))
      (print (infer-program-analysis-diagnostic-count invalid))
      0)))
"#,
        valid_source, invalid_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    assert_eq!(
        output.trim().lines().collect::<Vec<_>>(),
        ["0", "1"],
        "nested parametric record pattern は内側のfield型を外側の型引数へ伝播するべき"
    );
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
