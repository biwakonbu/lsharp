//! selfhost TypeInfer / match parity integration tests extracted from e2e.rs

mod common;
use common::*;

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[2], "400", "float infer の型名は Float hash=400 であるべき");
    assert_eq!(lines[3], "0", "unit infer は失敗すべきでない");
    assert_eq!(lines[4], "1", "unit infer の型タグは Con であるべき");
    assert_eq!(lines[5], "500", "unit infer の型名は Unit hash=500 であるべき");
}

/// selfhost TypeInfer.ls テスト: ann form は内側の式の型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_ann_expr() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[2], "100", "ann infer の型名は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 未定義変数は undefined error code を返せる
#[test]
fn test_e2e_selfhost_typeinfer_error_undefined_var_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[1], "2", "if cond mismatch error code は E0002 であるべき");
}

/// selfhost TypeInfer.ls テスト: if 分岐不一致は if-branch error code を返せる
#[test]
fn test_e2e_selfhost_typeinfer_error_if_branch_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[1], "3", "if branch mismatch error code は E0003 であるべき");
}

/// selfhost TypeInfer.ls テスト: apply 引数不一致は arg-mismatch error code を返せる
#[test]
fn test_e2e_selfhost_typeinfer_error_apply_arg_mismatch_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[1], "5", "infinite type error code は E0005 であるべき");
}

/// selfhost TypeInfer.ls テスト: lambda body の自己適用でも infinite error code を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_lambda_propagates_infinite_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

/// selfhost TypeInfer.ls テスト: match body failure でも infinite error code を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_match_propagates_infinite_body_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        scrut-hash 1300
        bind-hash 1200
        scrut-ty (fresh-type-var counter)
        env (type-env-insert env0 scrut-hash (mono scrut-ty))
        x-node (make-var bind-hash)
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
                  (vector-push (vector-new 5) 10)
                  (make-var scrut-hash))
                1)
              (make-var bind-hash))
            apply-node)
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
        "match infinite error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "自己適用 match infer は失敗すべき");
    assert_eq!(
        lines[1], "5",
        "match body failure の infinite error code は E0005 を維持すべき"
    );
}

/// selfhost TypeInfer.ls テスト: match arm 同士の結果型不一致は E0006 を返す
#[test]
fn test_e2e_selfhost_typeinfer_error_match_arm_result_mismatch_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        x-hash 1200
        y-hash 1201
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push (vector-new 7) 10)
                      (make-lit-int 1))
                    2)
                  (make-var x-hash))
                (make-lit-int 2))
              (make-var y-hash))
            (make-lit-bool 1))
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
        "match arm result mismatch error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "match arm result mismatch infer は失敗すべき");
    assert_eq!(
        lines[1], "6",
        "match arm result mismatch error code は E0006 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: scrutinee と pattern の型不一致は E0006 を返す
#[test]
fn test_e2e_selfhost_typeinfer_error_match_pattern_scrutinee_mismatch_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        some-hash 800
        ctor-hash 1300
        x-hash 1200
        ctor-ty (mk-fun (mk-int) (mk-con some-hash))
        env (type-env-insert env0 ctor-hash (mono ctor-ty))
        pat
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 11)
                ctor-hash)
              1)
            (make-var x-hash))
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 10)
                  (make-lit-int 1))
                1)
              pat)
            (make-lit-int 2))
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
        "match pattern/scrutinee mismatch error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "match pattern/scrutinee mismatch infer は失敗すべき"
    );
    assert_eq!(
        lines[1], "6",
        "match pattern/scrutinee mismatch error code は E0006 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: 未定義コンストラクタ pattern は E0001 を返す
#[test]
fn test_e2e_selfhost_typeinfer_error_match_undefined_constructor_pattern_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        missing-ctor 7777
        pat
          (vector-push
            (vector-push
              (vector-push (vector-new 3) 11)
              missing-ctor)
            0)
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 10)
                  (make-lit-int 1))
                1)
              pat)
            (make-lit-int 2))
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
        "undefined constructor pattern error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "undefined constructor pattern infer は失敗すべき");
    assert_eq!(
        lines[1], "1",
        "undefined constructor pattern error code は E0001 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: constructor subpattern の未定義 ctor も E0001 を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_match_constructor_child_pattern_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        outer-ctor 8000
        some-hash 700
        ctor-ty (mk-fun (mk-int) (mk-con some-hash))
        env (type-env-insert env0 outer-ctor (mono ctor-ty))
        child-pat
          (vector-push
            (vector-push
              (vector-push (vector-new 3) 11)
              8888)
            0)
        pat
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 11)
                outer-ctor)
              1)
            child-pat)
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 10)
                  (make-lit-int 1))
                1)
              pat)
            (make-lit-int 2))
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
        "constructor child pattern error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "constructor child pattern infer は失敗すべき");
    assert_eq!(
        lines[1], "1",
        "constructor child pattern error code は E0001 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: constructor pattern の引数数不一致は E0006 を返す
#[test]
fn test_e2e_selfhost_typeinfer_error_match_constructor_arity_mismatch_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        some-hash 800
        ctor-hash 1300
        value-hash 1301
        x-hash 1200
        env1 (type-env-insert env0 ctor-hash (mono (mk-con some-hash)))
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
               (make-lit-int 2))
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
        "constructor pattern arity mismatch error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "constructor pattern arity mismatch infer は失敗すべき");
    assert_eq!(
        lines[1], "6",
        "constructor pattern arity mismatch error code は E0006 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: ast-pat-constructor の未定義 ctor も E0001 を返す
#[test]
fn test_e2e_selfhost_typeinfer_error_match_pat_constructor_tag_undefined_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        missing-ctor 7777
        pat
          (vector-push
            (vector-push
              (vector-push (vector-new 3) (ast-pat-constructor))
              missing-ctor)
            0)
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 10)
                  (make-lit-int 1))
                1)
              pat)
            (make-lit-int 2))
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
        "canonical undefined constructor pattern error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "canonical undefined constructor pattern infer は失敗すべき"
    );
    assert_eq!(
        lines[1], "1",
        "canonical undefined constructor pattern error code は E0001 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: ast-pat-constructor の引数数不一致も E0006 を返す
#[test]
fn test_e2e_selfhost_typeinfer_error_match_pat_constructor_tag_arity_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        some-hash 800
        ctor-hash 1300
        value-hash 1301
        x-hash 1200
        env1 (type-env-insert env0 ctor-hash (mono (mk-con some-hash)))
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
               (make-lit-int 2))
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
        "canonical constructor pattern arity mismatch error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "canonical constructor pattern arity mismatch infer は失敗すべき"
    );
    assert_eq!(
        lines[1], "6",
        "canonical constructor pattern arity mismatch error code は E0006 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: ast-pat-recordpat の child failure も E0001 を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_match_pat_record_tag_child_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        field-x 120
        bad-child
          (vector-push
            (vector-push
              (vector-push (vector-new 3) (ast-pat-constructor))
              9999)
            0)
        pat
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) (ast-pat-recordpat))
                1)
              field-x)
            bad-child)
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 10)
                  (make-lit-int 1))
                1)
              pat)
            (make-lit-int 2))
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
        "canonical record child pattern error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "canonical record child pattern infer は失敗すべき"
    );
    assert_eq!(
        lines[1], "1",
        "canonical record child pattern error code は E0001 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: record subpattern の未定義 ctor も E0001 を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_match_record_child_pattern_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        scrut-hash 1300
        scrut-ty (fresh-type-var counter)
        env (type-env-insert env0 scrut-hash (mono scrut-ty))
        child-pat
          (vector-push
            (vector-push
              (vector-push (vector-new 3) 11)
              9999)
            0)
        pat
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 12)
                1)
              121)
            child-pat)
        node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 10)
                  (make-var scrut-hash))
                1)
              pat)
            (make-lit-int 2))
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
        "record child pattern error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "record child pattern infer は失敗すべき");
    assert_eq!(
        lines[1], "1",
        "record child pattern error code は E0001 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: record literal field failure でも infinite error code を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_record_literal_propagates_infinite_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
                  (vector-push (vector-new 5) 12)
                  700)
                1)
              121)
            apply-node)
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
        "record literal infinite error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "自己適用 record literal infer は失敗すべき");
    assert_eq!(
        lines[1], "5",
        "record literal field failure の infinite error code は E0005 を維持すべき"
    );
}

/// selfhost TypeInfer.ls テスト: field access base failure でも infinite error code を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_field_access_propagates_infinite_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
        node (make-fieldaccess apply-node 121)
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
        "field access infinite error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "自己適用 field access infer は失敗すべき");
    assert_eq!(
        lines[1], "5",
        "field access base failure の infinite error code は E0005 を維持すべき"
    );
}

/// selfhost TypeInfer.ls テスト: record update base failure でも infinite error code を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_record_update_propagates_infinite_code() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 3, "record literal typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "record literal infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "record literal infer の型タグは Con であるべき");
    assert_eq!(lines[2], "700", "record literal infer の型名は Point hash=700 であるべき");
}

/// selfhost TypeInfer.ls テスト: record update は base 式の型を維持できる
#[test]
fn test_e2e_selfhost_typeinfer_record_update() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn mk-point-type []
  (vector-push (vector-push (vector-new 2) 1) 700))

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

    assert!(lines.len() >= 3, "record update typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "record update infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "record update infer の型タグは Con であるべき");
    assert_eq!(lines[2], "700", "record update infer の型名は Point hash=700 であるべき");
}

/// selfhost TypeInfer.ls テスト: computation expression の最小型推論
#[test]
fn test_e2e_selfhost_typeinfer_computation_expr() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 6, "computation typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "return-only computation infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "return-only computation の型タグは Con であるべき");
    assert_eq!(lines[2], "100", "return-only computation の型名は Int hash=100 であるべき");
    assert_eq!(lines[3], "0", "let! computation infer は失敗すべきでない");
    assert_eq!(lines[4], "1", "let! computation の型タグは Con であるべき");
    assert_eq!(lines[5], "100", "let! computation の型名は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: single-step computation は最後の式型へ委譲できる
#[test]
fn test_e2e_selfhost_typeinfer_computation_single_step_bool() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[0], "0", "let! bool computation infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "let! bool computation の型タグは Con であるべき");
    assert_eq!(lines[2], "200", "let! bool computation の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: 2-step do! computation は最後の式型へ委譲できる
#[test]
fn test_e2e_selfhost_typeinfer_computation_do_bang_bool_return() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[0], "0", "do! bool computation infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do! bool computation の型タグは Con であるべき");
    assert_eq!(lines[2], "200", "do! bool computation の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: 3-step let! -> do! -> return は binder を維持できる
#[test]
fn test_e2e_selfhost_typeinfer_computation_let_bang_do_bang_return_bool() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[0], "0", "3-step let! do! computation infer は失敗すべきでない");
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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[0], "0", "3-step do! let! computation infer は失敗すべきでない");
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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 3, "do 6 exprs typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "do 6 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 6 exprs の型タグは Con であるべき");
    assert_eq!(lines[2], "200", "do 6 exprs の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: 7 式 do ブロックは最後の式型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_seven_exprs_last_bool() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 3, "do 7 exprs typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "do 7 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 7 exprs の型タグは Con であるべき");
    assert_eq!(lines[2], "200", "do 7 exprs の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: 8 式 do ブロックは最後の式型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_eight_exprs_last_bool() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 3, "do 8 exprs typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "do 8 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 8 exprs の型タグは Con であるべき");
    assert_eq!(lines[2], "200", "do 8 exprs の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: do 9 式は最後の式の型を返す
#[test]
fn test_e2e_selfhost_typeinfer_do_nine_exprs_last_bool() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 3, "do 9 exprs typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "do 9 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 9 exprs の型タグは Con であるべき");
    assert_eq!(lines[2], "200", "do 9 exprs の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: do ブロック 10 式は最後の Bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_ten_exprs_last_bool() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 3, "do 10 exprs typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "do 10 exprs infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "do 10 exprs の型タグは Con であるべき");
    assert_eq!(lines[2], "200", "do 10 exprs の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: do ブロック 11 式は最後の Bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_eleven_exprs_last_bool() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[2], "200", "do 11 exprs の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: do ブロック 12 式は最後の Bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_twelve_exprs_last_bool() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[2], "200", "do 12 exprs の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: do ブロック 13 式は最後の Bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_thirteen_exprs_last_bool() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[2], "200", "do 13 exprs の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: do ブロック 14 式は最後の Bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_do_fourteen_exprs_last_bool() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[2], "200", "do 14 exprs の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: 2 引数 lambda はカリー化された関数型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_two_params_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[3], "100", "outer param type は Int hash=100 であるべき");
    assert_eq!(lines[4], "3", "outer return type も Fun であるべき");
    assert_eq!(lines[5], "1", "inner param type は Con であるべき");
    assert_eq!(lines[6], "100", "inner param type は Int hash=100 であるべき");
    assert_eq!(lines[7], "1", "inner return type は Con であるべき");
    assert_eq!(lines[8], "100", "inner return type は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 2 引数 defn はカリー化された関数型になる
#[test]
fn test_e2e_selfhost_typeinfer_defn_two_params_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[3], "100", "outer param type は Int hash=100 であるべき");
    assert_eq!(lines[4], "3", "outer return type も Fun であるべき");
    assert_eq!(lines[5], "1", "inner param type は Con であるべき");
    assert_eq!(lines[6], "100", "inner param type は Int hash=100 であるべき");
    assert_eq!(lines[7], "1", "inner return type は Con であるべき");
    assert_eq!(lines[8], "100", "inner return type は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 3 引数 apply はカリー化された関数型をたどれる
#[test]
fn test_e2e_selfhost_typeinfer_apply_three_args_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[2], "100", "3 引数 apply の型名は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 4 引数 apply はカリー化された関数型をたどれる
#[test]
fn test_e2e_selfhost_typeinfer_apply_four_args_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[2], "100", "4 引数 apply の型名は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 5 引数 apply はカリー化された関数型をたどれる
#[test]
fn test_e2e_selfhost_typeinfer_apply_five_args_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[2], "100", "5 引数 apply の型名は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 6 引数 apply はカリー化された関数型をたどれる
#[test]
fn test_e2e_selfhost_typeinfer_apply_six_args_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[2], "100", "6 引数 apply の型名は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 7 引数 apply はカリー化された関数型をたどれる
#[test]
fn test_e2e_selfhost_typeinfer_apply_seven_args_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[2], "100", "7 引数 apply の型名は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 3 引数 lambda は 3 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_three_params_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[3], "100", "outer param type は Int hash=100 であるべき");
    assert_eq!(lines[4], "3", "mid type は Fun であるべき");
    assert_eq!(lines[5], "1", "mid param type は Con であるべき");
    assert_eq!(lines[6], "100", "mid param type は Int hash=100 であるべき");
    assert_eq!(lines[7], "3", "inner type は Fun であるべき");
    assert_eq!(lines[8], "1", "inner param type は Con であるべき");
    assert_eq!(lines[9], "100", "inner param type は Int hash=100 であるべき");
    assert_eq!(lines[10], "1", "inner return type は Con であるべき");
    assert_eq!(lines[11], "100", "inner return type は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 4 引数 lambda は 4 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_four_params_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[3], "100", "outer param type は Int hash=100 であるべき");
    assert_eq!(lines[4], "3", "level2 type は Fun であるべき");
    assert_eq!(lines[5], "1", "level2 param type は Con であるべき");
    assert_eq!(lines[6], "100", "level2 param type は Int hash=100 であるべき");
    assert_eq!(lines[7], "3", "level3 type は Fun であるべき");
    assert_eq!(lines[8], "1", "level3 param type は Con であるべき");
    assert_eq!(lines[9], "100", "level3 param type は Int hash=100 であるべき");
    assert_eq!(lines[10], "3", "level4 type は Fun であるべき");
    assert_eq!(lines[11], "1", "level4 param type は Con であるべき");
    assert_eq!(lines[12], "100", "level4 param type は Int hash=100 であるべき");
    assert_eq!(lines[13], "1", "level4 return type は Con であるべき");
    assert_eq!(lines[14], "100", "level4 return type は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 5 引数 lambda は 5 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_five_params_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[3], "100", "outer param type は Int hash=100 であるべき");
    assert_eq!(lines[4], "3", "level2 type は Fun であるべき");
    assert_eq!(lines[5], "1", "level2 param type は Con であるべき");
    assert_eq!(lines[6], "100", "level2 param type は Int hash=100 であるべき");
    assert_eq!(lines[7], "3", "level3 type は Fun であるべき");
    assert_eq!(lines[8], "1", "level3 param type は Con であるべき");
    assert_eq!(lines[9], "100", "level3 param type は Int hash=100 であるべき");
    assert_eq!(lines[10], "3", "level4 type は Fun であるべき");
    assert_eq!(lines[11], "1", "level4 param type は Con であるべき");
    assert_eq!(lines[12], "100", "level4 param type は Int hash=100 であるべき");
    assert_eq!(lines[13], "3", "level5 type は Fun であるべき");
    assert_eq!(lines[14], "1", "level5 param type は Con であるべき");
    assert_eq!(lines[15], "100", "level5 param type は Int hash=100 であるべき");
    assert_eq!(lines[16], "1", "level5 return type は Con であるべき");
    assert_eq!(lines[17], "100", "level5 return type は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 6 引数 lambda は 6 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_six_params_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[3], "100", "outer param type は Int hash=100 であるべき");
    assert_eq!(lines[4], "3", "level2 type は Fun であるべき");
    assert_eq!(lines[5], "1", "level2 param type は Con であるべき");
    assert_eq!(lines[6], "100", "level2 param type は Int hash=100 であるべき");
    assert_eq!(lines[7], "3", "level3 type は Fun であるべき");
    assert_eq!(lines[8], "1", "level3 param type は Con であるべき");
    assert_eq!(lines[9], "100", "level3 param type は Int hash=100 であるべき");
    assert_eq!(lines[10], "3", "level4 type は Fun であるべき");
    assert_eq!(lines[11], "1", "level4 param type は Con であるべき");
    assert_eq!(lines[12], "100", "level4 param type は Int hash=100 であるべき");
    assert_eq!(lines[13], "3", "level5 type は Fun であるべき");
    assert_eq!(lines[14], "1", "level5 param type は Con であるべき");
    assert_eq!(lines[15], "100", "level5 param type は Int hash=100 であるべき");
    assert_eq!(lines[16], "3", "level6 type は Fun であるべき");
    assert_eq!(lines[17], "1", "level6 param type は Con であるべき");
    assert_eq!(lines[18], "100", "level6 param type は Int hash=100 であるべき");
    assert_eq!(lines[19], "1", "level6 return type は Con であるべき");
    assert_eq!(lines[20], "100", "level6 return type は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 7 引数 lambda は 7 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_lambda_seven_params_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

/// selfhost TypeInfer.ls テスト: 3 引数 defn は 3 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_defn_three_params_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 140
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
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push (vector-new 7) 20)
                    name-hash)
                  3)
                x-hash)
              y-hash)
            z-hash)
        node (vector-push defn-node body-node)
        result (infer-defn node env counter)
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
        "three-param defn typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "three-param defn infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
    assert_eq!(lines[2], "1", "outer param type は Con であるべき");
    assert_eq!(lines[3], "100", "outer param type は Int hash=100 であるべき");
    assert_eq!(lines[4], "3", "mid type は Fun であるべき");
    assert_eq!(lines[5], "1", "mid param type は Con であるべき");
    assert_eq!(lines[6], "100", "mid param type は Int hash=100 であるべき");
    assert_eq!(lines[7], "3", "inner type は Fun であるべき");
    assert_eq!(lines[8], "1", "inner param type は Con であるべき");
    assert_eq!(lines[9], "100", "inner param type は Int hash=100 であるべき");
    assert_eq!(lines[10], "1", "inner return type は Con であるべき");
    assert_eq!(lines[11], "100", "inner return type は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 4 引数 defn は 4 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_defn_four_params_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 140
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
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push (vector-new 8) 20)
                      name-hash)
                    4)
                  x-hash)
                y-hash)
              z-hash)
            w-hash)
        node (vector-push defn-node body-node)
        result (infer-defn node env counter)
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
        "four-param defn typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "four-param defn infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
    assert_eq!(lines[2], "1", "outer param type は Con であるべき");
    assert_eq!(lines[3], "100", "outer param type は Int hash=100 であるべき");
    assert_eq!(lines[4], "3", "level2 type は Fun であるべき");
    assert_eq!(lines[5], "1", "level2 param type は Con であるべき");
    assert_eq!(lines[6], "100", "level2 param type は Int hash=100 であるべき");
    assert_eq!(lines[7], "3", "level3 type は Fun であるべき");
    assert_eq!(lines[8], "1", "level3 param type は Con であるべき");
    assert_eq!(lines[9], "100", "level3 param type は Int hash=100 であるべき");
    assert_eq!(lines[10], "3", "level4 type は Fun であるべき");
    assert_eq!(lines[11], "1", "level4 param type は Con であるべき");
    assert_eq!(lines[12], "100", "level4 param type は Int hash=100 であるべき");
    assert_eq!(lines[13], "1", "level4 return type は Con であるべき");
    assert_eq!(lines[14], "100", "level4 return type は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 5 引数 defn は 5 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_defn_five_params_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 140
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
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push (vector-new 9) 20)
                      name-hash)
                    5)
                  x-hash)
                y-hash)
              z-hash)
            w-hash)
        node (vector-push defn-node v-hash)
        node (vector-push node body-node)
        result (infer-defn node env counter)
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
        "five-param defn typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "five-param defn infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
    assert_eq!(lines[2], "1", "outer param type は Con であるべき");
    assert_eq!(lines[3], "100", "outer param type は Int hash=100 であるべき");
    assert_eq!(lines[4], "3", "level2 type は Fun であるべき");
    assert_eq!(lines[5], "1", "level2 param type は Con であるべき");
    assert_eq!(lines[6], "100", "level2 param type は Int hash=100 であるべき");
    assert_eq!(lines[7], "3", "level3 type は Fun であるべき");
    assert_eq!(lines[8], "1", "level3 param type は Con であるべき");
    assert_eq!(lines[9], "100", "level3 param type は Int hash=100 であるべき");
    assert_eq!(lines[10], "3", "level4 type は Fun であるべき");
    assert_eq!(lines[11], "1", "level4 param type は Con であるべき");
    assert_eq!(lines[12], "100", "level4 param type は Int hash=100 であるべき");
    assert_eq!(lines[13], "3", "level5 type は Fun であるべき");
    assert_eq!(lines[14], "1", "level5 param type は Con であるべき");
    assert_eq!(lines[15], "100", "level5 param type は Int hash=100 であるべき");
    assert_eq!(lines[16], "1", "level5 return type は Con であるべき");
    assert_eq!(lines[17], "100", "level5 return type は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 6 引数 defn は 6 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_defn_six_params_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 140
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
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push (vector-new 10) 20)
                        name-hash)
                      6)
                    x-hash)
                  y-hash)
                z-hash)
              w-hash)
            v-hash)
        node (vector-push defn-node u-hash)
        node (vector-push node body-node)
        result (infer-defn node env counter)
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
        "six-param defn typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "six-param defn infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
    assert_eq!(lines[2], "1", "outer param type は Con であるべき");
    assert_eq!(lines[3], "100", "outer param type は Int hash=100 であるべき");
    assert_eq!(lines[4], "3", "level2 type は Fun であるべき");
    assert_eq!(lines[5], "1", "level2 param type は Con であるべき");
    assert_eq!(lines[6], "100", "level2 param type は Int hash=100 であるべき");
    assert_eq!(lines[7], "3", "level3 type は Fun であるべき");
    assert_eq!(lines[8], "1", "level3 param type は Con であるべき");
    assert_eq!(lines[9], "100", "level3 param type は Int hash=100 であるべき");
    assert_eq!(lines[10], "3", "level4 type は Fun であるべき");
    assert_eq!(lines[11], "1", "level4 param type は Con であるべき");
    assert_eq!(lines[12], "100", "level4 param type は Int hash=100 であるべき");
    assert_eq!(lines[13], "3", "level5 type は Fun であるべき");
    assert_eq!(lines[14], "1", "level5 param type は Con であるべき");
    assert_eq!(lines[15], "100", "level5 param type は Int hash=100 であるべき");
    assert_eq!(lines[16], "3", "level6 type は Fun であるべき");
    assert_eq!(lines[17], "1", "level6 param type は Con であるべき");
    assert_eq!(lines[18], "100", "level6 param type は Int hash=100 であるべき");
    assert_eq!(lines[19], "1", "level6 return type は Con であるべき");
    assert_eq!(lines[20], "100", "level6 return type は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: 7 引数 defn は 7 段のカリー化型になる
#[test]
fn test_e2e_selfhost_typeinfer_defn_seven_params_curried() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

    // 7 引数 defn: (defn f [a b c d e f g] (+ a b)) → Fun(Int, Fun(Int, ...))
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        name-hash 99
        a-hash 101
        b-hash 102
        c-hash 103
        d-hash 104
        e-hash 105
        f-hash 106
        g-hash 107
        ;; body: (+ a b)
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
        ;; defn: [20, name-hash, param-count=7, a, b, c, d, e, f, g, body]
        defn-node
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push (vector-new 11) 20)
                            name-hash)
                          7)
                        a-hash)
                      b-hash)
                    c-hash)
                  d-hash)
                e-hash)
              f-hash)
            g-hash)
        node (vector-push defn-node body-node)
        result (infer-defn node env counter)
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
        "seven-param defn typeinfer 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "7 引数 defn infer は失敗すべきでない");
    assert_eq!(lines[1], "3", "outer type は Fun であるべき");
}

/// selfhost AST.ls テスト: field access constructor / traversal
#[test]
fn test_e2e_selfhost_ast_fieldaccess_helpers() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [p-hash 99
        field-hash 120
        node (make-fieldaccess (make-var p-hash) field-hash)]
    (do
      (print (if (= (vector-get node 0) (ast-fieldaccess)) 1 0))
      (print (if (= (vector-get (vector-get node 1) 0) (ast-var)) 1 0))
      (print (if (= (vector-get node 2) field-hash) 1 0))
      (print (ast-contains-var node p-hash))
      (print (ast-count-nodes node))
      0)))
"#;

    let combined = format!("{}\n{}", ast_ls, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 5, "fieldaccess AST helper 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "fieldaccess は ast-fieldaccess であるべき");
    assert_eq!(lines[1], "1", "fieldaccess inner は var であるべき");
    assert_eq!(lines[2], "1", "fieldaccess field hash が保持されるべき");
    assert_eq!(lines[3], "1", "fieldaccess inner var が探索できるべき");
    assert_eq!(lines[4], "2", "fieldaccess の node count は 2 であるべき");
}

/// selfhost Parser.ls テスト: field access expression を最小 payload でパースできる
#[test]
fn test_e2e_selfhost_parser_field_access_expr() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(project_root.join("selfhost/Lexer.ls"))
        .expect("selfhost/Lexer.ls が読み込めない");
    let parser_ls =
        std::fs::read_to_string(project_root.join("selfhost/Parser.ls"))
            .expect("selfhost/Parser.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(. p x)") 0)
        inner (vector-get node 1)]
    (do
      (print (if (= (vector-get node 0) (ast-fieldaccess)) 1 0))
      (print (if (= (vector-get inner 0) (ast-var)) 1 0))
      (print (if (= (vector-get inner 1) (name-hash "p" 0 1)) 1 0))
      (print (if (= (vector-get node 2) (name-hash "x" 0 1)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 4, "fieldaccess parser 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "fieldaccess は ast-fieldaccess であるべき");
    assert_eq!(lines[1], "1", "fieldaccess inner は var であるべき");
    assert_eq!(lines[2], "1", "fieldaccess inner hash が一致すべき");
    assert_eq!(lines[3], "1", "fieldaccess field hash が一致すべき");
}

/// selfhost TypeInfer.ls テスト: record type が分かる field access は実フィールド型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_field_access() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 3, "fieldaccess typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "fieldaccess infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "fieldaccess infer の型タグは Con であるべき");
    assert_eq!(lines[2], "100", "fieldaccess infer の型名は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: record type が分からない field access は fresh var fallback を返せる
#[test]
fn test_e2e_selfhost_typeinfer_field_access_fallback_var() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[0], "0", "fieldaccess fallback infer は失敗すべきでない");
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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    assert_eq!(lines[2], "100", "quote infer の型名は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: unquote は内側 var の型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_unquote_expr() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 3, "unquote typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "unquote infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "unquote infer の型タグは Con であるべき");
    assert_eq!(lines[2], "200", "unquote infer の型名は Bool hash=200 であるべき");
}

/// selfhost TypeInfer.ls テスト: unquote-splice は内側式の型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_unquote_splice_expr() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 3, "match binder typeinfer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "match binder infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "match binder infer の型タグは Con であるべき");
    assert_eq!(lines[2], "100", "match binder infer の型名は Int hash=100 であるべき");
}

/// selfhost TypeInfer.ls テスト: ast-pat-var でも match binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_var_tag_binder() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 3, "match record binder 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "match record binder infer は失敗すべきでない");
    assert_eq!(lines[1], "2", "match record binder infer の型タグは Var であるべき");
    assert_eq!(lines[2], "1001", "match record binder infer の型変数 ID は 1001 であるべき");
}

/// selfhost TypeInfer.ls テスト: match の constructor pattern binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_constructor_pattern_binder() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 3, "match constructor binder 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "match constructor binder infer は失敗すべきでない");
    assert_eq!(lines[1], "1", "match constructor binder infer の型タグは Con であるべき");
    assert_eq!(lines[2], "100", "match constructor binder infer の型名は Int であるべき");
}

/// selfhost TypeInfer.ls テスト: ast-pat-recordpat でも match binder を body で参照できる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_record_tag_binder() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

/// selfhost TypeInfer.ls テスト: ast-pat-lit は int/bool 型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_lit_tag() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 4, "match pat-lit infer 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "int pat-lit infer の型タグは Con であるべき");
    assert_eq!(lines[1], "100", "int pat-lit infer の型名は Int であるべき");
    assert_eq!(lines[2], "1", "bool pat-lit infer の型タグは Con であるべき");
    assert_eq!(lines[3], "200", "bool pat-lit infer の型名は Bool であるべき");
}

/// selfhost TypeInfer.ls テスト: ast-pat-lit は unit 型も返せる
#[test]
fn test_e2e_selfhost_typeinfer_match_pat_lit_unit_tag() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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

    assert!(lines.len() >= 2, "match pat-lit unit 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "unit pat-lit infer の型タグは Con であるべき");
    assert_eq!(lines[1], "500", "unit pat-lit infer の型名は Unit であるべき");
}

/// selfhost TypeInfer.ls テスト: constructor child の ast-pat-lit も unify できる
#[test]
fn test_e2e_selfhost_typeinfer_match_constructor_child_pat_lit() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
            .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
            .expect("selfhost/TypeInfer.ls が読み込めない");

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
    // 期待値: match 式の各腕の型が一致することをチェック
    let source = r#"
(module Main)
(defn main []
  (let [x 1]
    (print (match x
      [1 "one"]
      [_ "other"]))))
"#;
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "520");
}
