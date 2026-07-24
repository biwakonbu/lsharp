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
      (print (result-error-start result))
      (print (result-error-end result))
      (print (result-error-name-hash result))
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
    assert_eq!(lines[2], "-1", "spanなし undefined error の start は -1 であるべき");
    assert_eq!(lines[3], "-1", "spanなし undefined error の end は -1 であるべき");
    assert_eq!(lines[4], "99999", "spanなし undefined error の name hash を保持すべき");
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

/// selfhost TypeInfer.ls テスト: 失敗定義を直接失敗と依存失敗へ分類する
#[test]
fn test_e2e_selfhost_typeinfer_analysis_classifies_definition_failure_kinds() {
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(defn primary [] missing) (defn dependent [] primary) (defn independent [] missing-later)"))
        kinds (infer-program-analysis-failure-kinds analysis)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (vector-get kinds 0))
      (print (vector-get kinds 1))
      (print (vector-get kinds 2))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, ["3", "1", "2", "1"]);
}

/// EC-M1-01: 複数段の失敗定義連鎖を dependency failure として分類する
#[test]
fn test_e2e_selfhost_typeinfer_analysis_classifies_multilevel_definition_failure_kinds() {
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(defn primary [] missing) (defn middle [] primary) (defn dependent [] middle) (defn independent [] missing-later)"))
        kinds (infer-program-analysis-failure-kinds analysis)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (vector-get kinds 0))
      (print (vector-get kinds 1))
      (print (vector-get kinds 2))
      (print (vector-get kinds 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, ["4", "1", "2", "2", "1"]);
}

/// selfhost TypeInfer.ls テスト: 最初に失敗した式の source span を保持する
#[test]
fn test_e2e_selfhost_typeinfer_analysis_reports_first_failed_expression_span() {
    let source = "(defn fail [] missing)";
    let expected_start = source.find("missing").expect("fixture must contain missing");
    let expected_end = expected_start + "missing".len();
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(defn fail [] missing)"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-start analysis))
      (print (infer-program-analysis-first-error-end analysis))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.len(), 3, "source span 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1");
    assert_eq!(lines[1], expected_start.to_string());
    assert_eq!(lines[2], expected_end.to_string());
}

/// selfhost TypeInfer.ls テスト: nested if の condition failure も source span を保持する
#[test]
fn test_e2e_selfhost_typeinfer_analysis_reports_nested_if_failure_span() {
    let source = "(defn fail [] (if missing 1 2))";
    let expected_start = source.find("missing").expect("fixture must contain missing");
    let expected_end = expected_start + "missing".len();
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(defn fail [] (if missing 1 2))"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-start analysis))
      (print (infer-program-analysis-first-error-end analysis))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.len(), 3, "nested if span 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1");
    assert_eq!(lines[1], expected_start.to_string());
    assert_eq!(lines[2], expected_end.to_string());
}

/// selfhost TypeInfer.ls テスト: apply の callee failure も source span を保持する
#[test]
fn test_e2e_selfhost_typeinfer_analysis_reports_apply_callee_failure_span() {
    let source = "(defn fail [] (missing 2))";
    let expected_start = source.find("missing").expect("fixture must contain missing");
    let expected_end = expected_start + "missing".len();
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(defn fail [] (missing 2))"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-start analysis))
      (print (infer-program-analysis-first-error-end analysis))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.len(), 3, "apply span 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1");
    assert_eq!(lines[1], expected_start.to_string());
    assert_eq!(lines[2], expected_end.to_string());
}

/// selfhost TypeInfer.ls テスト: apply の argument failure も source span を保持する
#[test]
fn test_e2e_selfhost_typeinfer_analysis_reports_apply_argument_failure_span() {
    let source = "(defn fail [] (not missing))";
    let expected_start = source.find("missing").expect("fixture must contain missing");
    let expected_end = expected_start + "missing".len();
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(defn fail [] (not missing))"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-start analysis))
      (print (infer-program-analysis-first-error-end analysis))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.len(), 3, "apply argument span 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1");
    assert_eq!(lines[1], expected_start.to_string());
    assert_eq!(lines[2], expected_end.to_string());
}

/// selfhost TypeInfer.ls テスト: let initializer failure も source span を保持する
#[test]
fn test_e2e_selfhost_typeinfer_analysis_reports_let_initializer_failure_span() {
    let source = "(defn fail [] (let [value missing] value))";
    let expected_start = source.find("missing").expect("fixture must contain missing");
    let expected_end = expected_start + "missing".len();
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(defn fail [] (let [value missing] value))"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-start analysis))
      (print (infer-program-analysis-first-error-end analysis))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.len(), 3, "let initializer span 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1");
    assert_eq!(lines[1], expected_start.to_string());
    assert_eq!(lines[2], expected_end.to_string());
}

/// selfhost TypeInfer.ls テスト: computation の let! step failure も source span を保持する
#[test]
fn test_e2e_selfhost_typeinfer_analysis_reports_computation_step_failure_span() {
    let source = "(defn fail [] (computation maybe-builder (let! x missing) (return x)))";
    let expected_start = source.find("missing").expect("fixture must contain missing");
    let expected_end = expected_start + "missing".len();
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(defn fail [] (computation maybe-builder (let! x missing) (return x)))"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-start analysis))
      (print (infer-program-analysis-first-error-end analysis))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.len(), 3, "computation span 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1");
    assert_eq!(lines[1], expected_start.to_string());
    assert_eq!(lines[2], expected_end.to_string());
}

/// EC-M1-01: private defn は同一プログラム内の呼び出しから隠れないこと
#[test]
fn test_e2e_selfhost_typeinfer_analysis_accepts_private_definition_call() {
    let source = "(private (defn helper [value] (+ value 1))) (defn main [] (helper 1))";
    let program = lsharp_syntax::parse(source).expect("private defn fixture は parse できるべき");
    let mut oracle = Infer::new();
    assert!(
        oracle.infer_program(&program).is_ok(),
        "Rust oracle は同一プログラム内の private defn 呼び出しを受理するべき"
    );

    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(private (defn helper [value] (+ value 1))) (defn main [] (helper 1))"))
        kinds (infer-program-analysis-failure-kinds analysis)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (vector-length kinds))
      (print (vector-get kinds 0))
      (print (vector-get kinds 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "2", "0", "0"],
        "private defn 呼び出しは selfhost TypeInfer でも成功し、定義 failure を増やさないべき"
    );
}

/// EC-M1-01: do ブロック内の依存失敗も dependency failure として分類する
#[test]
fn test_e2e_selfhost_typeinfer_analysis_classifies_do_dependency_failure_kind() {
    let source = "(defn primary [] missing) (defn dependent [] (do primary 42))";
    let program = lsharp_syntax::parse(source).expect("do dependency fixture は parse できるべき");
    let mut oracle = Infer::new();
    assert!(
        oracle.infer_program(&program).is_err(),
        "Rust oracle は未定義 primary を含む do dependency fixture を拒否するべき"
    );

    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(defn primary [] missing) (defn dependent [] (do primary 42))"))
        kinds (infer-program-analysis-failure-kinds analysis)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (vector-get kinds 0))
      (print (vector-get kinds 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, ["2", "1", "2"]);
}

/// EC-M1-01: import の alias 経由の qualified function lookup を selfhost でも解決すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_resolves_import_alias_qualified_definition() {
    let source = "(module Lib) (defn helper [value] (+ value 1)) (module Main) (import Lib :as L) (defn main [] (L.helper 42))";
    let program = lsharp_syntax::parse(source).expect("qualified import fixture は parse できるべき");
    let mut oracle = Infer::new();
    let helper_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::int()],
        Box::new(lsharp_types::types::Type::int()),
    );
    oracle.inject_external_types(&[(
        "Lib.helper".to_string(),
        lsharp_types::types::TypeScheme::mono(helper_type),
    )]);
    let oracle_result = oracle.infer_program(&program);
    assert!(
        oracle_result.is_ok(),
        "Rust oracle は alias 経由の qualified definition lookup を受理するべき: {:?}",
        oracle_result.err()
    );

    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(module Lib) (defn helper [value] (+ value 1)) (module Main) (import Lib :as L) (defn main [] (L.helper 42))"))
        kinds (infer-program-analysis-failure-kinds analysis)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (vector-length kinds))
      (print (vector-get kinds 0))
      (print (vector-get kinds 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "2", "0", "0"],
        "alias qualified lookup は selfhost TypeInfer でも成功するべき"
    );
}

/// EC-M1-01: import の module 名経由の qualified function lookup を selfhost でも解決すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_resolves_import_module_qualified_definition() {
    let source =
        "(module Lib) (defn helper [value] (+ value 1)) (module Main) (import Lib) (defn main [] (Lib.helper 42))";
    let program =
        lsharp_syntax::parse(source).expect("module-qualified import fixture は parse できるべき");
    let mut oracle = Infer::new();
    let helper_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::int()],
        Box::new(lsharp_types::types::Type::int()),
    );
    oracle.inject_external_types(&[(
        "Lib.helper".to_string(),
        lsharp_types::types::TypeScheme::mono(helper_type),
    )]);
    let oracle_result = oracle.infer_program(&program);
    assert!(
        oracle_result.is_ok(),
        "Rust oracle は module 名経由の qualified definition lookup を受理するべき: {:?}",
        oracle_result.err()
    );

    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(module Lib) (defn helper [value] (+ value 1)) (module Main) (import Lib) (defn main [] (Lib.helper 42))"))
        kinds (infer-program-analysis-failure-kinds analysis)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (vector-length kinds))
      (print (vector-get kinds 0))
      (print (vector-get kinds 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "2", "0", "0"],
        "module-qualified lookup は selfhost TypeInfer でも成功するべき"
    );
}

/// EC-M1-01: import の :open が namespace なしの function lookup を selfhost でも解決すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_resolves_import_open_definition() {
    let open_source = "(module Main) (import Lib :open) (defn main [] (helper 42))";
    let open_program =
        lsharp_syntax::parse(open_source).expect("open import fixture は parse できるべき");
    let helper_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::int()],
        Box::new(lsharp_types::types::Type::int()),
    );
    let mut open_oracle = Infer::new();
    open_oracle.inject_external_types(&[(
        "helper".to_string(),
        lsharp_types::types::TypeScheme::mono(helper_type.clone()),
    )]);
    let open_oracle_result = open_oracle.infer_program(&open_program);
    assert!(
        open_oracle_result.is_ok(),
        "Rust oracle は open import の unqualified definition lookup を受理するべき: {:?}",
        open_oracle_result.err()
    );

    let closed_source = "(module Main) (import Lib) (defn main [] (helper 42))";
    let closed_program =
        lsharp_syntax::parse(closed_source).expect("closed import fixture は parse できるべき");
    let mut closed_oracle = Infer::new();
    closed_oracle.inject_external_types(&[(
        "Lib.helper".to_string(),
        lsharp_types::types::TypeScheme::mono(helper_type),
    )]);
    assert!(
        closed_oracle.infer_program(&closed_program).is_err(),
        "Rust oracle は open でない import の unqualified lookup を拒否するべき"
    );

    let harness = r#"
(defn main []
  (let [open-analysis
          (infer-program-analysis
            (parse-program "(module Lib) (defn helper [value] (+ value 1)) (module Main) (import Lib :open) (defn main [] (helper 42))"))
        closed-analysis
          (infer-program-analysis
            (parse-program "(module Lib) (defn helper [value] (+ value 1)) (module Main) (import Lib) (defn main [] (helper 42))"))
        leak-analysis
          (infer-program-analysis
            (parse-program "(module Lib) (defn helper [value] (+ value 1)) (module Mid) (import Lib :open) (defn mid [] (helper 42)) (module Main) (defn main [] (helper 42))"))]
    (do
      (print (infer-program-analysis-diagnostic-count open-analysis))
      (print (infer-program-analysis-diagnostic-count closed-analysis))
      (print (infer-program-analysis-diagnostic-count leak-analysis))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "1"],
        "import :open だけが namespace なしの lookup を許可し、後続 module へ漏らさないべき"
    );
}

/// EC-M1-01: import の :only が qualified lookup の公開 symbol 境界を守ること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_filters_import_only_qualified_definition() {
    let selected_source =
        "(module Lib) (defn helper [value] (+ value 1)) (defn hidden [value] (+ value 2)) (module Main) (import Lib :only [helper]) (defn main [] (Lib.helper 42))";
    let selected_oracle_source =
        "(module Main) (import Lib :only [helper]) (defn main [] (Lib.helper 42))";
    let selected_program = lsharp_syntax::parse(selected_oracle_source)
        .expect("selected :only oracle fixture は parse できるべき");
    let mut selected_oracle = Infer::new();
    let helper_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::int()],
        Box::new(lsharp_types::types::Type::int()),
    );
    let selected_only = vec!["helper".to_string()];
    selected_oracle.inject_external_types_for_import(
        "Lib",
        Some(selected_only.as_slice()),
        &std::collections::HashSet::new(),
        &[(
            "Lib.helper".to_string(),
            lsharp_types::types::TypeScheme::mono(helper_type),
        )],
    );
    assert!(
        selected_oracle.infer_program(&selected_program).is_ok(),
        "Rust oracle は :only で選択された definition を受理するべき"
    );

    let excluded_source =
        "(module Lib) (defn helper [value] (+ value 1)) (defn hidden [value] (+ value 2)) (module Main) (import Lib :only [helper]) (defn main [] (Lib.hidden 42))";
    let excluded_oracle_source =
        "(module Main) (import Lib :only [helper]) (defn main [] (Lib.hidden 42))";
    let excluded_program = lsharp_syntax::parse(excluded_oracle_source)
        .expect("excluded :only oracle fixture は parse できるべき");
    let mut excluded_oracle = Infer::new();
    let hidden_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::int()],
        Box::new(lsharp_types::types::Type::int()),
    );
    let excluded_only = vec!["helper".to_string()];
    excluded_oracle.inject_external_types_for_import(
        "Lib",
        Some(excluded_only.as_slice()),
        &std::collections::HashSet::new(),
        &[(
            "Lib.hidden".to_string(),
            lsharp_types::types::TypeScheme::mono(hidden_type),
        )],
    );
    assert!(
        excluded_oracle.infer_program(&excluded_program).is_err(),
        "Rust oracle は :only で除外された definition を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        selected-kinds (infer-program-analysis-failure-kinds selected)
        excluded
          (infer-program-analysis
            (parse-program "{}"))
        excluded-kinds (infer-program-analysis-failure-kinds excluded)]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (vector-length selected-kinds))
      (print (vector-get selected-kinds 0))
      (print (vector-get selected-kinds 1))
      (print (infer-program-analysis-diagnostic-count excluded))
      (print (vector-length excluded-kinds))
      (print (vector-get excluded-kinds 0))
      (print (vector-get excluded-kinds 1))
      0)))
"#,
        selected_source,
        excluded_source
    );

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "3", "0", "0", "1", "3", "0", "0"],
        "import :only は selected symbol だけを qualified lookup へ公開するべき"
    );
}

/// EC-M1-01: import の :as + :only が alias-qualified lookup の公開境界を守ること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_qualified_definition() {
    let selected_source =
        "(module Lib) (defn helper [value] (+ value 1)) (defn hidden [value] (+ value 2)) (module Main) (import Lib :as L :only [helper]) (defn main [] (L.helper 42))";
    let selected_oracle_source =
        "(module Main) (import Lib :as L :only [helper]) (defn main [] (L.helper 42))";
    let selected_program = lsharp_syntax::parse(selected_oracle_source)
        .expect("selected alias + :only oracle fixture は parse できるべき");
    let mut selected_oracle = Infer::new();
    let helper_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::int()],
        Box::new(lsharp_types::types::Type::int()),
    );
    let selected_only = vec!["helper".to_string()];
    selected_oracle.inject_external_types_for_import(
        "Lib",
        Some(selected_only.as_slice()),
        &std::collections::HashSet::new(),
        &[(
            "Lib.helper".to_string(),
            lsharp_types::types::TypeScheme::mono(helper_type),
        )],
    );
    assert!(
        selected_oracle.infer_program(&selected_program).is_ok(),
        "Rust oracle は alias + :only で選択された definition を受理するべき"
    );

    let excluded_source =
        "(module Lib) (defn helper [value] (+ value 1)) (defn hidden [value] (+ value 2)) (module Main) (import Lib :as L :only [helper]) (defn main [] (L.hidden 42))";
    let excluded_oracle_source =
        "(module Main) (import Lib :as L :only [helper]) (defn main [] (L.hidden 42))";
    let excluded_program = lsharp_syntax::parse(excluded_oracle_source)
        .expect("excluded alias + :only oracle fixture は parse できるべき");
    let mut excluded_oracle = Infer::new();
    let hidden_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::int()],
        Box::new(lsharp_types::types::Type::int()),
    );
    let excluded_only = vec!["helper".to_string()];
    excluded_oracle.inject_external_types_for_import(
        "Lib",
        Some(excluded_only.as_slice()),
        &std::collections::HashSet::new(),
        &[(
            "Lib.hidden".to_string(),
            lsharp_types::types::TypeScheme::mono(hidden_type),
        )],
    );
    assert!(
        excluded_oracle.infer_program(&excluded_program).is_err(),
        "Rust oracle は alias + :only で除外された definition を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        selected-kinds (infer-program-analysis-failure-kinds selected)
        excluded
          (infer-program-analysis
            (parse-program "{}"))
        excluded-kinds (infer-program-analysis-failure-kinds excluded)]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (vector-length selected-kinds))
      (print (vector-get selected-kinds 0))
      (print (vector-get selected-kinds 1))
      (print (infer-program-analysis-diagnostic-count excluded))
      (print (vector-length excluded-kinds))
      (print (vector-get excluded-kinds 0))
      (print (vector-get excluded-kinds 1))
      0)))
"#,
        selected_source,
        excluded_source
    );

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "3", "0", "0", "1", "3", "0", "0"],
        "import :as + :only は selected symbol だけを alias-qualified lookup へ公開するべき"
    );
}

/// EC-M1-01: import の :open は public function だけを unqualified lookup へ公開すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_filters_import_open_unqualified_definition() {
    let selected_source =
        "(module Lib) (defn helper [value] (+ value 1)) (private (defn secret [value] (+ value 2))) (module Main) (import Lib :open) (defn main [] (helper 42))";
    let selected_oracle_source = "(module Main) (import Lib :open) (defn main [] (helper 42))";
    let selected_program = lsharp_syntax::parse(selected_oracle_source)
        .expect("selected :open oracle fixture は parse できるべき");
    let mut selected_oracle = Infer::new();
    let helper_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::int()],
        Box::new(lsharp_types::types::Type::int()),
    );
    selected_oracle.inject_external_types(&[(
        "helper".to_string(),
        lsharp_types::types::TypeScheme::mono(helper_type),
    )]);
    assert!(
        selected_oracle.infer_program(&selected_program).is_ok(),
        "Rust oracle は :open の public definition を unqualified lookup で受理するべき"
    );

    let closed_source =
        "(module Lib) (defn helper [value] (+ value 1)) (module Main) (import Lib) (defn main [] (helper 42))";
    let closed_oracle_source = "(module Main) (import Lib) (defn main [] (helper 42))";
    let closed_program = lsharp_syntax::parse(closed_oracle_source)
        .expect("closed import oracle fixture は parse できるべき");
    let mut closed_oracle = Infer::new();
    assert!(
        closed_oracle.infer_program(&closed_program).is_err(),
        "Rust oracle は :open なしの unqualified definition を拒否するべき"
    );

    let private_source =
        "(module Lib) (defn helper [value] (+ value 1)) (private (defn secret [value] (+ value 2))) (module Main) (import Lib :open) (defn main [] (secret 42))";
    let private_oracle_source = "(module Main) (import Lib :open) (defn main [] (secret 42))";
    let private_program = lsharp_syntax::parse(private_oracle_source)
        .expect("private :open oracle fixture は parse できるべき");
    let mut private_oracle = Infer::new();
    assert!(
        private_oracle.infer_program(&private_program).is_err(),
        "Rust oracle は :open でも private definition を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        selected-kinds (infer-program-analysis-failure-kinds selected)
        closed
          (infer-program-analysis
            (parse-program "{}"))
        closed-kinds (infer-program-analysis-failure-kinds closed)
        blocked
          (infer-program-analysis
            (parse-program "{}"))
        blocked-kinds (infer-program-analysis-failure-kinds blocked)]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (vector-length selected-kinds))
      (print (vector-get selected-kinds 0))
      (print (vector-get selected-kinds 1))
      (print (infer-program-analysis-diagnostic-count closed))
      (print (vector-length closed-kinds))
      (print (vector-get closed-kinds 0))
      (print (vector-get closed-kinds 1))
      (print (infer-program-analysis-diagnostic-count blocked))
      (print (vector-length blocked-kinds))
      (print (vector-get blocked-kinds 0))
      (print (vector-get blocked-kinds 1))
      0)))
"#,
        selected_source, closed_source, private_source
    );

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "3", "0", "0", "1", "2", "0", "1", "1", "3", "0", "0"],
        "import :open は public symbol だけを unqualified lookup へ公開するべき"
    );
}

/// EC-M1-01: import の :open + :only が selected symbol だけを unqualified lookup へ公開すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_filters_import_open_only_unqualified_definition() {
    let selected_source =
        "(module Lib) (defn helper [value] (+ value 1)) (defn hidden [value] (+ value 2)) (module Main) (import Lib :open :only [helper]) (defn main [] (helper 42))";
    let selected_oracle_source =
        "(module Main) (import Lib :open :only [helper]) (defn main [] (helper 42))";
    let selected_program = lsharp_syntax::parse(selected_oracle_source)
        .expect("selected :open + :only oracle fixture は parse できるべき");
    let mut selected_oracle = Infer::new();
    let helper_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::int()],
        Box::new(lsharp_types::types::Type::int()),
    );
    let selected_only = vec!["helper".to_string()];
    selected_oracle.inject_external_types_for_import(
        "Lib",
        Some(selected_only.as_slice()),
        &std::collections::HashSet::new(),
        &[(
            "helper".to_string(),
            lsharp_types::types::TypeScheme::mono(helper_type.clone()),
        )],
    );
    assert!(
        selected_oracle.infer_program(&selected_program).is_ok(),
        "Rust oracle は :open + :only の selected definition を受理するべき"
    );

    let excluded_source =
        "(module Lib) (defn helper [value] (+ value 1)) (defn hidden [value] (+ value 2)) (module Main) (import Lib :open :only [helper]) (defn main [] (hidden 42))";
    let excluded_oracle_source =
        "(module Main) (import Lib :open :only [helper]) (defn main [] (hidden 42))";
    let excluded_program = lsharp_syntax::parse(excluded_oracle_source)
        .expect("excluded :open + :only oracle fixture は parse できるべき");
    let mut excluded_oracle = Infer::new();
    excluded_oracle.inject_external_types_for_import(
        "Lib",
        Some(selected_only.as_slice()),
        &std::collections::HashSet::new(),
        &[(
            "hidden".to_string(),
            lsharp_types::types::TypeScheme::mono(helper_type),
        )],
    );
    assert!(
        excluded_oracle.infer_program(&excluded_program).is_err(),
        "Rust oracle は :open + :only で除外された definition を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        excluded
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (infer-program-analysis-diagnostic-count excluded))
      0)))
"#,
        selected_source, excluded_source
    );

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1"],
        "import :open + :only は selected symbol だけを unqualified lookup へ公開するべき"
    );
}

/// EC-M1-01: import の module prefix 経由で ADT constructor を解決すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_resolves_import_qualified_adt_constructor() {
    let source =
        "(module Lib) (type Option (Some Int) None) (module Main) (import Lib) (defn main [] (Lib.Some 42))";
    let oracle_source = "(module Main) (import Lib) (defn main [] (Lib.Some 42))";
    let oracle_program =
        lsharp_syntax::parse(oracle_source).expect("qualified ADT constructor fixture は parse できるべき");
    let mut oracle = Infer::new();
    let some_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::int()],
        Box::new(lsharp_types::types::Type::Con("Lib.Option".to_string())),
    );
    oracle.inject_external_types(&[(
        "Lib.Some".to_string(),
        lsharp_types::types::TypeScheme::mono(some_type),
    )]);
    assert!(
        oracle.infer_program(&oracle_program).is_ok(),
        "Rust oracle は qualified ADT constructor lookup を受理するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "{}"))
        kinds (infer-program-analysis-failure-kinds analysis)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (vector-length kinds))
      (print (vector-get kinds 0))
      0)))
"#,
        source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "0"],
        "qualified ADT constructor は module prefix 付き export lookup で解決されるべき"
    );
}

/// EC-M1-01: import の alias + :only が ADT constructor export 境界を守ること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_adt_constructor() {
    let selected_source =
        "(module Lib) (type Option (Some Int) (Other Bool) None) (module Main) (import Lib :as L :only [Some]) (defn main [] (L.Some 42))";
    let selected_oracle_source =
        "(module Main) (import Lib :as L :only [Some]) (defn main [] (L.Some 42))";
    let selected_program = lsharp_syntax::parse(selected_oracle_source)
        .expect("selected alias + :only ADT oracle fixture は parse できるべき");
    let some_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::int()],
        Box::new(lsharp_types::types::Type::Con("Lib.Option".to_string())),
    );
    let mut selected_oracle = Infer::new();
    selected_oracle.inject_external_types_for_import(
        "Lib",
        Some(&["Some".to_string()]),
        &std::collections::HashSet::new(),
        &[(
            "Lib.Some".to_string(),
            lsharp_types::types::TypeScheme::mono(some_type),
        )],
    );
    assert!(
        selected_oracle.infer_program(&selected_program).is_ok(),
        "Rust oracle は alias + :only の selected ADT constructor を受理するべき"
    );

    let excluded_source =
        "(module Lib) (type Option (Some Int) (Other Bool) None) (module Main) (import Lib :as L :only [Some]) (defn main [] (L.Other true))";
    let excluded_oracle_source =
        "(module Main) (import Lib :as L :only [Some]) (defn main [] (L.Other true))";
    let excluded_program = lsharp_syntax::parse(excluded_oracle_source)
        .expect("excluded alias + :only ADT oracle fixture は parse できるべき");
    let other_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::bool()],
        Box::new(lsharp_types::types::Type::Con("Lib.Option".to_string())),
    );
    let mut excluded_oracle = Infer::new();
    excluded_oracle.inject_external_types_for_import(
        "Lib",
        Some(&["Some".to_string()]),
        &std::collections::HashSet::new(),
        &[(
            "Lib.Other".to_string(),
            lsharp_types::types::TypeScheme::mono(other_type),
        )],
    );
    assert!(
        excluded_oracle.infer_program(&excluded_program).is_err(),
        "Rust oracle は alias + :only で除外された ADT constructor を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        selected-kinds (infer-program-analysis-failure-kinds selected)
        excluded
          (infer-program-analysis
            (parse-program "{}"))
        excluded-kinds (infer-program-analysis-failure-kinds excluded)]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (vector-length selected-kinds))
      (print (vector-get selected-kinds 0))
      (print (infer-program-analysis-diagnostic-count excluded))
      (print (vector-length excluded-kinds))
      (print (vector-get excluded-kinds 0))
      0)))
"#,
        selected_source, excluded_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "0", "1", "1", "1"],
        "alias + :only は selected ADT constructor だけを qualified export へ公開するべき"
    );
}

/// EC-M1-01: import の module prefix 経由で record field accessor を解決すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_resolves_import_qualified_record_accessor() {
    let source = "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib) (defn get-x [] Lib.Point.x)";
    let oracle_source = "(module Main) (import Lib) (defn get-x [] Lib.Point.x)";
    let oracle_program = lsharp_syntax::parse(oracle_source)
        .expect("qualified record accessor fixture は parse できるべき");
    let mut oracle = Infer::new();
    let accessor_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::Con("Lib.Point".to_string())],
        Box::new(lsharp_types::types::Type::int()),
    );
    oracle.inject_external_types(&[(
        "Lib.Point.x".to_string(),
        lsharp_types::types::TypeScheme::mono(accessor_type),
    )]);
    assert!(
        oracle.infer_program(&oracle_program).is_ok(),
        "Rust oracle は qualified record accessor lookup を受理するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "{}"))
        kinds (infer-program-analysis-failure-kinds analysis)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (vector-length kinds))
      (print (vector-get kinds 0))
      0)))
"#,
        source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "0"],
        "qualified record accessor は module prefix 付き export lookup で解決されるべき"
    );
}

/// EC-M1-01: import の alias + :only が record field accessor export 境界を守ること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_record_accessor() {
    let selected_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :as L :only [Point.x]) (defn get-x [] L.Point.x)";
    let selected_oracle_source = "(module Main) (import Lib :as L :only [Point.x]) (defn get-x [] L.Point.x)";
    let selected_program = lsharp_syntax::parse(selected_oracle_source)
        .expect("selected alias + :only record accessor oracle fixture は parse できるべき");
    let accessor_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::Con("Lib.Point".to_string())],
        Box::new(lsharp_types::types::Type::int()),
    );
    let mut selected_oracle = Infer::new();
    selected_oracle.inject_external_types_for_import(
        "Lib",
        Some(&["Point.x".to_string()]),
        &std::collections::HashSet::new(),
        &[(
            "Lib.Point.x".to_string(),
            lsharp_types::types::TypeScheme::mono(accessor_type),
        )],
    );
    assert!(
        selected_oracle.infer_program(&selected_program).is_ok(),
        "Rust oracle は alias + :only の selected record accessor を受理するべき"
    );

    let excluded_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :as L :only [Point.x]) (defn get-y [] L.Point.y)";
    let excluded_oracle_source = "(module Main) (import Lib :as L :only [Point.x]) (defn get-y [] L.Point.y)";
    let excluded_program = lsharp_syntax::parse(excluded_oracle_source)
        .expect("excluded alias + :only record accessor oracle fixture は parse できるべき");
    let excluded_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::Con("Lib.Point".to_string())],
        Box::new(lsharp_types::types::Type::int()),
    );
    let mut excluded_oracle = Infer::new();
    excluded_oracle.inject_external_types_for_import(
        "Lib",
        Some(&["Point.x".to_string()]),
        &std::collections::HashSet::new(),
        &[(
            "Lib.Point.x".to_string(),
            lsharp_types::types::TypeScheme::mono(excluded_type),
        )],
    );
    assert!(
        excluded_oracle.infer_program(&excluded_program).is_err(),
        "Rust oracle は :only で除外された record accessor を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        selected-kinds (infer-program-analysis-failure-kinds selected)
        excluded
          (infer-program-analysis
            (parse-program "{}"))
        excluded-kinds (infer-program-analysis-failure-kinds excluded)]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (vector-length selected-kinds))
      (print (vector-get selected-kinds 0))
      (print (infer-program-analysis-diagnostic-count excluded))
      (print (vector-length excluded-kinds))
      (print (vector-get excluded-kinds 0))
      0)))
"#,
        selected_source, excluded_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "0", "1", "1", "1"],
        "alias + :only は selected record accessor だけを qualified export へ公開するべき"
    );
}

/// EC-M1-01: import の :open が record field accessor だけを unqualified lookup へ公開すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_filters_import_open_record_accessor() {
    let open_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :open) (defn get-x [] Point.x)";
    let open_oracle_source = "(module Main) (import Lib :open) (defn get-x [] Point.x)";
    let open_program =
        lsharp_syntax::parse(open_oracle_source).expect("open record accessor oracle は parse できるべき");
    let accessor_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::Con("Lib.Point".to_string())],
        Box::new(lsharp_types::types::Type::int()),
    );
    let mut open_oracle = Infer::new();
    open_oracle.inject_external_types(&[(
        "Point.x".to_string(),
        lsharp_types::types::TypeScheme::mono(accessor_type.clone()),
    )]);
    assert!(
        open_oracle.infer_program(&open_program).is_ok(),
        "Rust oracle は :open の unqualified record accessor を受理するべき"
    );

    let closed_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib) (defn get-x [] Point.x)";
    let closed_oracle_source = "(module Main) (import Lib) (defn get-x [] Point.x)";
    let closed_program =
        lsharp_syntax::parse(closed_oracle_source).expect("closed record accessor oracle は parse できるべき");
    let mut closed_oracle = Infer::new();
    closed_oracle.inject_external_types(&[(
        "Lib.Point.x".to_string(),
        lsharp_types::types::TypeScheme::mono(accessor_type),
    )]);
    assert!(
        closed_oracle.infer_program(&closed_program).is_err(),
        "Rust oracle は :open なしの unqualified record accessor を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [open-analysis
          (infer-program-analysis
            (parse-program "{}"))
        closed-analysis
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count open-analysis))
      (print (infer-program-analysis-diagnostic-count closed-analysis))
      0)))
"#,
        open_source, closed_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1"],
        "import :open だけが unqualified record accessor を公開するべき"
    );
}

/// EC-M1-01: import の :open + :only が selected record field accessor だけを公開すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_filters_import_open_only_record_accessor() {
    let selected_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :open :only [Point.x]) (defn get-x [] Point.x)";
    let selected_oracle_source = "(module Main) (import Lib :open :only [Point.x]) (defn get-x [] Point.x)";
    let selected_program = lsharp_syntax::parse(selected_oracle_source)
        .expect("selected open + only record accessor oracle は parse できるべき");
    let accessor_type = lsharp_types::types::Type::Fun(
        vec![lsharp_types::types::Type::Con("Lib.Point".to_string())],
        Box::new(lsharp_types::types::Type::int()),
    );
    let mut selected_oracle = Infer::new();
    selected_oracle.inject_external_types(&[(
        "Point.x".to_string(),
        lsharp_types::types::TypeScheme::mono(accessor_type.clone()),
    )]);
    assert!(
        selected_oracle.infer_program(&selected_program).is_ok(),
        "Rust oracle は open + only の selected record accessor を受理するべき"
    );

    let excluded_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :open :only [Point.x]) (defn get-y [] Point.y)";
    let excluded_oracle_source = "(module Main) (import Lib :open :only [Point.x]) (defn get-y [] Point.y)";
    let excluded_program = lsharp_syntax::parse(excluded_oracle_source)
        .expect("excluded open + only record accessor oracle は parse できるべき");
    let mut excluded_oracle = Infer::new();
    excluded_oracle.inject_external_types(&[(
        "Lib.Point.y".to_string(),
        lsharp_types::types::TypeScheme::mono(accessor_type),
    )]);
    assert!(
        excluded_oracle.infer_program(&excluded_program).is_err(),
        "Rust oracle は open + only で除外された record accessor を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        excluded
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (infer-program-analysis-diagnostic-count excluded))
      0)))
"#,
        selected_source, excluded_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1"],
        "import :open + :only は selected record accessor だけを unqualified lookup へ公開するべき"
    );
}

/// EC-M1-01: import の alias + :only が record constructor export 境界を守ること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_record_constructor() {
    let selected_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (type Hidden (record (: x Int) (: y Int))) (module Main) (import Lib :as L :only [Point]) (defn main [] (L.Point 1 2))";
    let selected_oracle_source =
        "(module Main) (import Lib :as L :only [Point]) (defn main [] (L.Point 1 2))";
    let selected_program = lsharp_syntax::parse(selected_oracle_source)
        .expect("selected alias + :only record constructor oracle は parse できるべき");
    let point_type = lsharp_types::types::Type::Fun(
        vec![
            lsharp_types::types::Type::int(),
            lsharp_types::types::Type::int(),
        ],
        Box::new(lsharp_types::types::Type::Con("Lib.Point".to_string())),
    );
    let mut selected_oracle = Infer::new();
    selected_oracle.inject_external_types_for_import(
        "Lib",
        Some(&["Point".to_string()]),
        &std::collections::HashSet::new(),
        &[(
            "Lib.Point".to_string(),
            lsharp_types::types::TypeScheme::mono(point_type),
        )],
    );
    assert!(
        selected_oracle.infer_program(&selected_program).is_ok(),
        "Rust oracle は alias + :only の selected record constructor を受理するべき"
    );

    let excluded_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (type Hidden (record (: x Int) (: y Int))) (module Main) (import Lib :as L :only [Point]) (defn main [] (L.Hidden 1 2))";
    let excluded_oracle_source =
        "(module Main) (import Lib :as L :only [Point]) (defn main [] (L.Hidden 1 2))";
    let excluded_program = lsharp_syntax::parse(excluded_oracle_source)
        .expect("excluded alias + :only record constructor oracle は parse できるべき");
    let hidden_type = lsharp_types::types::Type::Fun(
        vec![
            lsharp_types::types::Type::int(),
            lsharp_types::types::Type::int(),
        ],
        Box::new(lsharp_types::types::Type::Con("Lib.Hidden".to_string())),
    );
    let mut excluded_oracle = Infer::new();
    excluded_oracle.inject_external_types_for_import(
        "Lib",
        Some(&["Point".to_string()]),
        &std::collections::HashSet::new(),
        &[(
            "Lib.Hidden".to_string(),
            lsharp_types::types::TypeScheme::mono(hidden_type),
        )],
    );
    assert!(
        excluded_oracle.infer_program(&excluded_program).is_err(),
        "Rust oracle は alias + :only で除外された record constructor を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        excluded
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (infer-program-analysis-diagnostic-count excluded))
      0)))
"#,
        selected_source, excluded_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1"],
        "alias + :only は selected record constructor だけを qualified export へ公開するべき"
    );
}

/// EC-M1-01: import の module prefix 経由で record constructor を解決すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_resolves_import_qualified_record_constructor() {
    let source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib) (defn main [] (Lib.Point 1 2))";
    let oracle_source = "(module Main) (import Lib) (defn main [] (Lib.Point 1 2))";
    let oracle_program = lsharp_syntax::parse(oracle_source)
        .expect("qualified record constructor fixture は parse できるべき");
    let mut oracle = Infer::new();
    let point_type = lsharp_types::types::Type::Fun(
        vec![
            lsharp_types::types::Type::int(),
            lsharp_types::types::Type::int(),
        ],
        Box::new(lsharp_types::types::Type::Con("Lib.Point".to_string())),
    );
    oracle.inject_external_types(&[(
        "Lib.Point".to_string(),
        lsharp_types::types::TypeScheme::mono(point_type),
    )]);
    assert!(
        oracle.infer_program(&oracle_program).is_ok(),
        "Rust oracle は qualified record constructor lookup を受理するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "{}"))
        kinds (infer-program-analysis-failure-kinds analysis)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (vector-length kinds))
      (print (vector-get kinds 0))
      0)))
"#,
        source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "0"],
        "qualified record constructor は module prefix 付き export lookup で解決されるべき"
    );
}

/// EC-M1-01: qualified record constructor の結果を qualified record 型として注釈できること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_resolves_import_qualified_record_type_annotation() {
    let source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib) (defn main [] (: (Lib.Point 1 2) Lib.Point))";
    let oracle_source =
        "(module Main) (import Lib) (defn main [] (: (Lib.Point 1 2) Lib.Point))";
    let oracle_program = lsharp_syntax::parse(oracle_source)
        .expect("qualified record type annotation fixture は parse できるべき");
    let mut oracle = Infer::new();
    let point_type = lsharp_types::types::Type::Fun(
        vec![
            lsharp_types::types::Type::int(),
            lsharp_types::types::Type::int(),
        ],
        Box::new(lsharp_types::types::Type::Con("Lib.Point".to_string())),
    );
    oracle.inject_external_types(&[(
        "Lib.Point".to_string(),
        lsharp_types::types::TypeScheme::mono(point_type),
    )]);
    assert!(
        oracle.infer_program(&oracle_program).is_ok(),
        "Rust oracle は qualified record type annotation を受理するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      0)))
"#,
        source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0"],
        "qualified record constructor の結果は同じ qualified record 型 annotation と unify されるべき"
    );
}

/// EC-M1-01: import alias と `:only` の record 型 annotation を解決すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_resolves_import_alias_only_qualified_record_type_annotation() {
    let source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :as L :only [Point]) (defn main [] (: (L.Point 1 2) L.Point))";
    let invalid_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :as L :only [Point]) (defn main [] (: (L.Point 1 2) Int))";
    let oracle_source =
        "(type L.Point (record (: x Int) (: y Int))) (defn main [] (: (L.Point 1 2) L.Point))";
    let oracle_program = lsharp_syntax::parse(oracle_source)
        .expect("alias-qualified record type annotation fixture は parse できるべき");
    let mut oracle = Infer::new();
    assert!(
        oracle.infer_program(&oracle_program).is_ok(),
        "Rust oracle は alias-qualified record type annotation を受理するべき"
    );
    let invalid_oracle_source =
        "(type L.Point (record (: x Int) (: y Int))) (defn main [] (: (L.Point 1 2) Int))";
    let invalid_oracle_program = lsharp_syntax::parse(invalid_oracle_source)
        .expect("invalid alias-qualified record type annotation fixture は parse できるべき");
    let mut invalid_oracle = Infer::new();
    assert!(
        invalid_oracle.infer_program(&invalid_oracle_program).is_err(),
        "Rust oracle は alias-qualified record type annotation の mismatch を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        invalid
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (infer-program-analysis-diagnostic-count invalid))
      0)))
"#,
        source, invalid_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1"],
        "alias-qualified record type annotation は `:only` で公開された schema と unify するべき"
    );
}

/// EC-M1-01: defn の alias-qualified record return signature を解決すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_resolves_import_alias_qualified_record_defn_signature() {
    let source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :as L :only [Point]) (defn make [] : L.Point (L.Point 1 2))";
    let invalid_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :as L :only [Point]) (defn make [] : L.Point (L.Point true 2))";
    let oracle_source =
        "(type L.Point (record (: x Int) (: y Int))) (defn make [] : L.Point (L.Point 1 2))";
    let oracle_program = lsharp_syntax::parse(oracle_source)
        .expect("alias-qualified record defn signature fixture は parse できるべき");
    let mut oracle = Infer::new();
    assert!(
        oracle.infer_program(&oracle_program).is_ok(),
        "Rust oracle は alias-qualified record defn signature を受理するべき"
    );
    let invalid_oracle_source =
        "(type L.Point (record (: x Int) (: y Int))) (defn make [] : L.Point (L.Point true 2))";
    let invalid_oracle_program = lsharp_syntax::parse(invalid_oracle_source)
        .expect("invalid alias-qualified record defn signature fixture は parse できるべき");
    let mut invalid_oracle = Infer::new();
    assert!(
        invalid_oracle.infer_program(&invalid_oracle_program).is_err(),
        "Rust oracle は alias-qualified record constructor の field mismatch を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        invalid
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (infer-program-analysis-diagnostic-count invalid))
      0)))
"#,
        source, invalid_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1"],
        "defn signature は alias-qualified record constructor の結果 schema と unify するべき"
    );
}

/// EC-M1-01: nested function type の record 引数を alias-qualified schema として解決すること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_resolves_nested_alias_qualified_record_signature() {
    let source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :as L) (defn get-x [] : (-> L.Point Int) (fn [point] (L.Point.x point)))";
    let invalid_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :as L) (defn get-x [] : (-> L.Point Bool) (fn [point] (L.Point.x point)))";
    let oracle_source =
        "(type L.Point (record (: x Int) (: y Int))) (defn get-x [] : (-> L.Point Int) (fn [point] (L.Point.x point)))";
    let oracle_program = lsharp_syntax::parse(oracle_source)
        .expect("nested alias-qualified record signature fixture は parse できるべき");
    let mut oracle = Infer::new();
    assert!(
        oracle.infer_program(&oracle_program).is_ok(),
        "Rust oracle は nested alias-qualified record signature を受理するべき"
    );
    let invalid_oracle_source =
        "(type L.Point (record (: x Int) (: y Int))) (defn get-x [] : (-> L.Point Bool) (fn [point] (L.Point.x point)))";
    let invalid_oracle_program = lsharp_syntax::parse(invalid_oracle_source)
        .expect("invalid nested alias-qualified record signature fixture は parse できるべき");
    let mut invalid_oracle = Infer::new();
    assert!(
        invalid_oracle.infer_program(&invalid_oracle_program).is_err(),
        "Rust oracle は nested alias-qualified record signature の return mismatch を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        invalid
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (infer-program-analysis-diagnostic-count invalid))
      0)))
"#,
        source, invalid_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1"],
        "nested function type は alias-qualified record schema と accessor result を unify するべき"
    );
}

/// EC-M1-01: imported record は qualified record literal として構築できること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_resolves_import_qualified_record_literal() {
    let source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib) (defn main [] {Lib.Point x 1 y 2})";
    let invalid_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib) (defn main [] {Lib.Point x true y 2})";
    let oracle_source =
        "(type Lib.Point (record (: x Int) (: y Int))) (defn main [] {Lib.Point x 1 y 2})";
    let oracle_program = lsharp_syntax::parse(oracle_source)
        .expect("qualified record literal fixture は parse できるべき");
    let mut oracle = Infer::new();
    assert!(
        oracle.infer_program(&oracle_program).is_ok(),
        "Rust oracle は qualified record literal を受理するべき"
    );
    let invalid_oracle_source =
        "(type Lib.Point (record (: x Int) (: y Int))) (defn main [] {Lib.Point x true y 2})";
    let invalid_oracle_program = lsharp_syntax::parse(invalid_oracle_source)
        .expect("invalid qualified record literal fixture は parse できるべき");
    let mut invalid_oracle = Infer::new();
    assert!(
        invalid_oracle.infer_program(&invalid_oracle_program).is_err(),
        "Rust oracle は qualified record literal の field type mismatch を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        invalid
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (infer-program-analysis-diagnostic-count invalid))
      0)))
"#,
        source, invalid_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1"],
        "qualified record literal は import された record schema と field 型を unify するべき"
    );
}

/// EC-M1-01: import alias の record は alias-qualified record literal として構築できること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_resolves_import_alias_qualified_record_literal() {
    let source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :as L) (defn main [] {L.Point x 1 y 2})";
    let invalid_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (module Main) (import Lib :as L) (defn main [] {L.Point x true y 2})";
    let oracle_source =
        "(type L.Point (record (: x Int) (: y Int))) (defn main [] {L.Point x 1 y 2})";
    let oracle_program = lsharp_syntax::parse(oracle_source)
        .expect("alias-qualified record literal fixture は parse できるべき");
    let mut oracle = Infer::new();
    assert!(
        oracle.infer_program(&oracle_program).is_ok(),
        "Rust oracle は alias-qualified record literal を受理するべき"
    );
    let invalid_oracle_source =
        "(type L.Point (record (: x Int) (: y Int))) (defn main [] {L.Point x true y 2})";
    let invalid_oracle_program = lsharp_syntax::parse(invalid_oracle_source)
        .expect("invalid alias-qualified record literal fixture は parse できるべき");
    let mut invalid_oracle = Infer::new();
    assert!(
        invalid_oracle.infer_program(&invalid_oracle_program).is_err(),
        "Rust oracle は alias-qualified record literal の field type mismatch を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        invalid
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (infer-program-analysis-diagnostic-count invalid))
      0)))
"#,
        source, invalid_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1"],
        "alias-qualified record literal は import alias の record schema と field 型を unify するべき"
    );
}

/// EC-M1-01: alias + :only で除外された record literal を受理しないこと
#[test]
fn test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_record_literal() {
    let selected_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (type Hidden (record (: x Int) (: y Int))) (module Main) (import Lib :as L :only [Point]) (defn main [] {L.Point x 1 y 2})";
    let excluded_source =
        "(module Lib) (type Point (record (: x Int) (: y Int))) (type Hidden (record (: x Int) (: y Int))) (module Main) (import Lib :as L :only [Point]) (defn main [] {L.Hidden x 1 y 2})";
    let selected_oracle_source =
        "(type L.Point (record (: x Int) (: y Int))) (defn main [] {L.Point x 1 y 2})";
    let selected_oracle_program = lsharp_syntax::parse(selected_oracle_source)
        .expect("selected alias + :only record literal oracle は parse できるべき");
    let mut selected_oracle = Infer::new();
    assert!(
        selected_oracle.infer_program(&selected_oracle_program).is_ok(),
        "Rust oracle は alias + :only の selected record literal を受理するべき"
    );

    // Rust oracle は record registry の可視集合を、除外された宣言を省略して表現する。
    let excluded_oracle_source = "(defn main [] {L.Hidden x 1 y 2})";
    let excluded_oracle_program = lsharp_syntax::parse(excluded_oracle_source)
        .expect("excluded alias + :only record literal oracle は parse できるべき");
    let mut excluded_oracle = Infer::new();
    assert!(
        excluded_oracle.infer_program(&excluded_oracle_program).is_err(),
        "Rust oracle は alias + :only で除外された record literal を拒否するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [selected
          (infer-program-analysis
            (parse-program "{}"))
        excluded
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count selected))
      (print (infer-program-analysis-diagnostic-count excluded))
      0)))
"#,
        selected_source, excluded_source
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1"],
        "alias + :only は selected record literal だけを schema lookup へ公開するべき"
    );
}
