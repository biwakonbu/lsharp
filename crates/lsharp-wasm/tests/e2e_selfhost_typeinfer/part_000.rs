fn typeinfer_runtime_modules() -> (String, String, String, String, String) {
    let ast_ls = std::fs::read_to_string(selfhost_source_path("AST.ls"))
        .expect("canonical AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(selfhost_source_path("Type.ls"))
        .expect("canonical Type.ls が読み込めない");
    let type_scheme_ls = std::fs::read_to_string(selfhost_source_path("TypeScheme.ls"))
        .expect("canonical TypeScheme.ls が読み込めない");
    let type_infer_core_ls = std::fs::read_to_string(selfhost_source_path("TypeInferCore.ls"))
        .expect("canonical TypeInferCore.ls が読み込めない");
    let type_infer_functions_ls =
        std::fs::read_to_string(selfhost_source_path("TypeInferFunctions.ls"))
            .expect("canonical TypeInferFunctions.ls が読み込めない");
    let type_infer_builtins_ls =
        std::fs::read_to_string(selfhost_source_path("TypeInferBuiltins.ls"))
            .expect("canonical TypeInferBuiltins.ls が読み込めない");
    let type_infer_apply_ls = std::fs::read_to_string(selfhost_source_path("TypeInferApply.ls"))
        .expect("canonical TypeInferApply.ls が読み込めない");
    let type_infer_block_ls = std::fs::read_to_string(selfhost_source_path("TypeInferBlock.ls"))
        .expect("canonical TypeInferBlock.ls が読み込めない");
    let type_infer_pattern_ls =
        std::fs::read_to_string(selfhost_source_path("TypeInferPattern.ls"))
            .expect("canonical TypeInferPattern.ls が読み込めない");
    let type_infer_record_ls = std::fs::read_to_string(selfhost_source_path("TypeInferRecord.ls"))
        .expect("canonical TypeInferRecord.ls が読み込めない");
    let type_infer_record_decl_ls =
        std::fs::read_to_string(selfhost_source_path("TypeInferRecordDecl.ls"))
            .expect("canonical TypeInferRecordDecl.ls が読み込めない");
    let type_infer_adt_ls = std::fs::read_to_string(selfhost_source_path("TypeInferAdt.ls"))
        .expect("canonical TypeInferAdt.ls が読み込めない");
    let type_infer_ls = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        type_infer_functions_ls,
        type_infer_builtins_ls,
        std::fs::read_to_string(selfhost_source_path("TypeInfer.ls"))
            .expect("canonical TypeInfer.ls が読み込めない"),
        type_infer_apply_ls,
        type_infer_block_ls,
        type_infer_pattern_ls,
        type_infer_record_ls,
        type_infer_record_decl_ls,
        type_infer_adt_ls
    );
    (
        ast_ls,
        type_ls,
        type_scheme_ls,
        type_infer_core_ls,
        type_infer_ls,
    )
}

fn ast_runtime_module() -> String {
    std::fs::read_to_string(selfhost_source_path("AST.ls"))
        .expect("canonical AST.ls が読み込めない")
}

fn parser_runtime_modules() -> (String, String, String, String) {
    let token_ls = std::fs::read_to_string(selfhost_source_path("Token.ls"))
        .expect("canonical Token.ls が読み込めない");
    let ast_ls = ast_runtime_module();
    let lexer_ls = std::fs::read_to_string(selfhost_source_path("Lexer.ls"))
        .expect("canonical Lexer.ls が読み込めない");
    let parser_ls = std::fs::read_to_string(selfhost_source_path("Parser.ls"))
        .expect("canonical Parser.ls が読み込めない");
    (token_ls, ast_ls, lexer_ls, parser_ls)
}

// === TypeInfer Tests ===

/// selfhost TypeInfer.ls テスト: リテラル型推論
#[test]
fn test_e2e_selfhost_typeinfer_literal() {
    // selfhost compiler でリテラルの型推論が動作することを検証
    // 期待値: Int リテラルが正しく型付けされ実行可能
    let source = r#"
(module Main)
(defn main [] (print 42))
"#;
    // selfhost パイプラインで compile & run
    // TypeInfer.ls が型推論を行い、正しく型付けされた AST を返す
    let result = compile_and_run(source);
    assert_eq!(result.trim(), "42");
}

/// selfhost TypeInfer.ls テスト: float / unit リテラル型推論
#[test]
fn test_e2e_selfhost_typeinfer_float_and_unit_literals() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        float-node (make-lit-float 0 4)
        unit-node (make-lit-unit)
        float-result (infer-expr float-node env (subst-new) counter)
        unit-result (infer-expr unit-node env (subst-new) counter)]
    (do
      (print (result-failed float-result))
      (print (ty-tag (result-type float-result)))
      (print (ty-name (result-type float-result)))
      (print (result-failed unit-result))
      (print (ty-tag (result-type unit-result)))
      (print (ty-name (result-type unit-result)))
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
        "float/unit typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "float infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "float infer の型タグは Con であるべき");
    assert_eq!(
        lines[2], "400",
        "float infer の型名は Float hash=400 であるべき"
    );
    assert_eq!(lines[3], "0", "unit infer は失敗すべきでない");
    assert_eq!(lines[4], "1", "unit infer の型タグは Con であるべき");
    assert_eq!(
        lines[5], "500",
        "unit infer の型名は Unit hash=500 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: ann form は内側の式の型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_ann_expr() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        ann-node (make-ann (make-lit-int 42))
        result (infer-expr ann-node env (subst-new) counter)]
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

    assert!(lines.len() >= 3, "ann typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "ann infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "ann infer の型タグは Con であるべき");
    assert_eq!(
        lines[2], "100",
        "ann infer の型名は Int hash=100 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 未定義変数は undefined error code を返せる
#[test]
fn test_e2e_selfhost_typeinfer_error_undefined_var_code() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        result (infer-expr (make-var 99999) env (subst-new) counter)]
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
        "undefined error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "未定義変数 infer は失敗すべき");
    assert_eq!(lines[1], "1", "未定義変数 error code は E0001 であるべき");
}

/// selfhost TypeInfer.ls テスト: if 条件不一致は if-cond error code を返せる
#[test]
fn test_e2e_selfhost_typeinfer_error_if_cond_code() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        if-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 6)
                (make-lit-int 1))
              (make-lit-int 2))
            (make-lit-int 3))
        result (infer-expr if-node env (subst-new) counter)]
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
        "if cond error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "if cond mismatch infer は失敗すべき");
    assert_eq!(
        lines[1], "2",
        "if cond mismatch error code は E0002 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: if 分岐不一致は if-branch error code を返せる
#[test]
fn test_e2e_selfhost_typeinfer_error_if_branch_code() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        if-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 6)
                (make-lit-bool 1))
              (make-lit-int 2))
            (make-lit-bool 0))
        result (infer-expr if-node env (subst-new) counter)]
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
        "if branch error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "if branch mismatch infer は失敗すべき");
    assert_eq!(
        lines[1], "3",
        "if branch mismatch error code は E0003 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: apply 引数不一致は arg-mismatch error code を返せる
#[test]
fn test_e2e_selfhost_typeinfer_error_apply_arg_mismatch_code() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        apply-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 5)
                (make-lit-int 1))
              1)
            (make-lit-int 2))
        result (infer-expr apply-node env (subst-new) counter)]
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
        "apply arg mismatch error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "apply arg mismatch infer は失敗すべき");
    assert_eq!(
        lines[1], "4",
        "apply arg mismatch error code は E0004 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: apply 内の未定義関数エラーは nested code を伝播できる
#[test]
fn test_e2e_selfhost_typeinfer_error_apply_propagates_func_code() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        apply-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 5)
                (make-var 99999))
              1)
            (make-lit-int 2))
        result (infer-expr apply-node env (subst-new) counter)]
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
        "apply nested undefined error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "apply nested undefined infer は失敗すべき");
    assert_eq!(
        lines[1], "1",
        "apply nested undefined error code は E0001 を伝播すべき"
    );
}

/// selfhost TypeInfer.ls テスト: 自己適用の occurs-check は infinite error code を返せる
#[test]
fn test_e2e_selfhost_typeinfer_error_infinite_type_code() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        x-hash 120
        x-ty (fresh-type-var counter)
        env (type-env-insert env0 x-hash (mono x-ty))
        x-node (make-var x-hash)
        apply-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 5)
                x-node)
              1)
            x-node)
        result (infer-expr apply-node env (subst-new) counter)]
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
        "infinite type error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "自己適用 infer は失敗すべき");
    assert_eq!(
        lines[1], "5",
        "infinite type error code は E0005 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: lambda body の自己適用でも infinite error code を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_lambda_propagates_infinite_code() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        x-hash 120
        x-node (make-var x-hash)
        apply-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 5)
                x-node)
              1)
            x-node)
        lambda-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 8)
                1)
              x-hash)
            apply-node)
        result (infer-expr lambda-node env (subst-new) counter)]
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
        "lambda infinite error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "自己適用 lambda infer は失敗すべき");
    assert_eq!(
        lines[1], "5",
        "lambda body の infinite error code は E0005 を維持すべき"
    );
}

/// selfhost TypeInfer.ls テスト: defn body の自己適用でも infinite error code を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_defn_propagates_infinite_code() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 122
        x-hash 120
        x-node (make-var x-hash)
        apply-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 5)
                x-node)
              1)
            x-node)
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 20)
                  name-hash)
                1)
              x-hash)
            apply-node)
        result (infer-defn defn-node env counter)]
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
        "defn infinite error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "自己適用 defn infer は失敗すべき");
    assert_eq!(
        lines[1], "5",
        "defn body の infinite error code は E0005 を維持すべき"
    );
}

/// selfhost TypeInfer.ls テスト: let init の自己適用でも infinite error code を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_let_propagates_infinite_init_code() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        let-name-hash 121
        x-hash 120
        x-ty (fresh-type-var counter)
        env (type-env-insert env0 x-hash (mono x-ty))
        x-node (make-var x-hash)
        apply-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 5)
                x-node)
              1)
            x-node)
        body-node (make-var let-name-hash)
        let-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 7)
                let-name-hash)
              apply-node)
            body-node)
        result (infer-expr let-node env (subst-new) counter)]
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
        "let infinite error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "自己適用 let infer は失敗すべき");
    assert_eq!(
        lines[1], "5",
        "let init の infinite error code は E0005 を維持すべき"
    );
}

/// selfhost TypeInfer.ls テスト: do 先頭式の自己適用でも infinite error code を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_do_propagates_infinite_code() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        x-hash 120
        x-ty (fresh-type-var counter)
        env (type-env-insert env0 x-hash (mono x-ty))
        x-node (make-var x-hash)
        apply-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 5)
                x-node)
              1)
            x-node)
        do-node
          (vector-push
            (vector-push
              (vector-push (vector-new 4) 9)
              2)
            apply-node)
        result (infer-expr (vector-push do-node (make-lit-bool 1)) env (subst-new) counter)]
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
        "do infinite error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "自己適用 do infer は失敗すべき");
    assert_eq!(
        lines[1], "5",
        "do 先頭式の infinite error code は E0005 を維持すべき"
    );
}

/// selfhost TypeInfer.ls テスト: computation step failure でも infinite error code を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_computation_propagates_infinite_code() {
    let (ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls) =
        typeinfer_runtime_modules();

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        outer-hash 120
        bind-hash 121
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
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push (vector-new 9) 15)
                        901)
                      2)
                    (computation-step-let-bang))
                  bind-hash)
                apply-node)
              (computation-step-return))
            0)
        comp-node (vector-push node (make-var bind-hash))
        result (infer-expr comp-node env (subst-new) counter)]
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
        "computation infinite error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "自己適用 computation infer は失敗すべき");
    assert_eq!(
        lines[1], "5",
        "computation step failure の infinite error code は E0005 を維持すべき"
    );
}
