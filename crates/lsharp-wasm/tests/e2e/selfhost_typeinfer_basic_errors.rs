use super::support::*;

/// selfhost TypeInfer.ls テスト: ann form は内側の式の型を返せる
#[test]
fn test_e2e_selfhost_typeinfer_ann_expr() {
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

/// selfhost TypeInfer.ls テスト: typed ann は primitive 型との一致を検査する
#[test]
fn test_e2e_selfhost_typeinfer_typed_ann_unifies_primitive_type() {
    let harness = r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        int-type-expr (make-type-named 73679)
        bool-type-expr (make-type-named 2076426)
        string-type-expr (make-type-named 2486848561)
        float-type-expr (make-type-named 67973692)
        unit-type-expr (make-type-named 2641316)
        accepted (infer-expr (make-ann-typed (make-lit-int 42) int-type-expr) env (subst-new) counter)
        accepted-string (infer-expr (make-ann-typed (vector-push (vector-new 1) (ast-lit-string)) string-type-expr) env (subst-new) counter)
        accepted-float (infer-expr (make-ann-typed (make-lit-float 0 0) float-type-expr) env (subst-new) counter)
        accepted-unit (infer-expr (make-ann-typed (make-lit-unit) unit-type-expr) env (subst-new) counter)
        rejected (infer-expr (make-ann-typed (make-lit-int 42) bool-type-expr) env (subst-new) counter)]
    (do
      (print (result-failed accepted))
      (print (ty-name (result-type accepted)))
      (print (result-failed accepted-string))
      (print (ty-name (result-type accepted-string)))
      (print (result-failed accepted-float))
      (print (ty-name (result-type accepted-float)))
      (print (result-failed accepted-unit))
      (print (ty-name (result-type accepted-unit)))
      (print (result-failed rejected))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "100", "0", "300", "0", "400", "0", "500", "1"],
        "typed ann は全 primitive 型が一致するときだけ成功するべき"
    );
}

/// selfhost TypeInfer.ls テスト: raw TypeApp / TypeFun annotation を internal type へ解決できる
#[test]
fn test_e2e_selfhost_typeinfer_typed_ann_unifies_type_app_and_fun() {
    let harness = r#"
(defn raw-type-named [name-hash]
  (vector-push (vector-push (vector-new 2) 60) name-hash))

(defn raw-type-app1 [name-hash arg]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 61)
        name-hash)
      1)
    arg))

(defn raw-type-fun1 [param ret]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 62)
        1)
      param)
    ret))

(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        env1 (type-env-insert env0 901 (mono (mk-ref (mk-int))))
        env2 (type-env-insert env1 902 (mono (mk-fun (mk-int) (mk-string))))
        int-expr (raw-type-named 73679)
        string-expr (raw-type-named 2486848561)
        ref-int-expr (raw-type-app1 82035 int-expr)
        int-to-string-expr (raw-type-fun1 int-expr string-expr)
        ref-result (infer-expr (make-ann-typed (make-var 901) ref-int-expr) env2 (subst-new) counter)
        fun-result (infer-expr (make-ann-typed (make-var 902) int-to-string-expr) env2 (subst-new) counter)]
    (do
      (print (result-failed ref-result))
      (print (ty-tag (result-type ref-result)))
      (print (ty-name (result-type ref-result)))
      (print (result-failed fun-result))
      (print (ty-tag (result-type fun-result)))
      (print (ty-name (ty-fp (result-type fun-result))))
      (print (ty-name (ty-fr (result-type fun-result))))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "5", "800", "0", "3", "100", "300"],
        "raw TypeApp / TypeFun annotation は internal type と一致するべき"
    );
}

/// selfhost TypeInfer.ls テスト: raw TypeVar annotation は同名 nominal type へ解決できる
#[test]
fn test_e2e_selfhost_typeinfer_typed_ann_unifies_type_var() {
    let harness = r#"
(defn raw-type-var [name-hash]
  (vector-push (vector-push (vector-new 2) 63) name-hash))

(defn main []
  (let [counter (make-var-counter)
        env0 (init-builtin-env counter)
        env (type-env-insert env0 903 (mono (mk-con 97)))
        result (infer-expr (make-ann-typed (make-var 903) (raw-type-var 97)) env (subst-new) counter)]
    (do
      (print (result-failed result))
      (print (ty-tag (result-type result)))
      (print (ty-name (result-type result)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "97"],
        "raw TypeVar annotation は同名 nominal type と一致するべき"
    );
}

/// selfhost TypeInfer.ls テスト: defn signature は param / return 型の不一致を拒否する
#[test]
fn test_e2e_selfhost_typeinfer_typed_defn_signature_rejects_mismatch() {
    let harness = r#"
(defn make-typed-defn [name-hash param-hash param-type return-type body]
  (let [signature
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 65)
                1)
              param-type)
            return-type)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push (vector-new 6) 20)
              name-hash)
            1)
          param-hash)
        body)
      signature)))

(defn main []
  (let [counter (make-var-counter)
        x-hash 120
        bad-defn (make-typed-defn 121 x-hash (make-type-named 2076426) (make-type-named 73679) (make-var x-hash))
        program (vector-push (vector-new 1) bad-defn)
        analysis (infer-program-analysis program)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-code analysis))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "6"],
        "typed defn signature は param と return の不一致を一般型エラーにするべき"
    );
}

/// selfhost TypeInfer.ls テスト: 未定義変数は undefined error code を返せる
#[test]
fn test_e2e_selfhost_typeinfer_error_undefined_var_code() {
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

/// selfhost TypeInfer.ls テスト: 最初に失敗した top-level 定義の位置を保持する
#[test]
fn test_e2e_selfhost_typeinfer_analysis_reports_first_failed_definition_index() {
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(defn ok [] 42) (defn fail [] missing) (defn later [] missing-later)"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-index analysis))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["2", "1"],
        "最初の失敗定義の index と診断数を保持するべき"
    );
}

/// selfhost TypeInfer.ls テスト: 最初に失敗した top-level 定義の name hash を保持する
#[test]
fn test_e2e_selfhost_typeinfer_analysis_reports_first_failed_definition_name_hash() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn ok [] 42) (defn fail [] missing) (defn later [] missing-later)")
        analysis (infer-program-analysis program)
        failed-decl (vector-get program 1)
        expected-name-hash (vector-get failed-decl 1)
        actual-name-hash (infer-program-analysis-first-error-name-hash analysis)]
    (do
      (print (= expected-name-hash actual-name-hash))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1"],
        "最初の失敗定義の name hash を AST と一致させるべき"
    );
}
