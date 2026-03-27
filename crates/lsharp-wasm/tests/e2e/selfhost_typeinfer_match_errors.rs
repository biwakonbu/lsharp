use super::support::*;

/// selfhost TypeInfer.ls テスト: match body failure でも infinite error code を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_match_propagates_infinite_body_code() {
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "match arm result mismatch error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "match arm result mismatch infer は失敗すべき"
    );
    assert_eq!(
        lines[1], "6",
        "match arm result mismatch error code は E0006 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: scrutinee と pattern の型不一致は E0006 を返す
#[test]
fn test_e2e_selfhost_typeinfer_error_match_pattern_scrutinee_mismatch_code() {
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "undefined constructor pattern error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "undefined constructor pattern infer は失敗すべき"
    );
    assert_eq!(
        lines[1], "1",
        "undefined constructor pattern error code は E0001 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: constructor subpattern の未定義 ctor も E0001 を保つ
#[test]
fn test_e2e_selfhost_typeinfer_error_match_constructor_child_pattern_code() {
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "constructor child pattern error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "constructor child pattern infer は失敗すべき"
    );
    assert_eq!(
        lines[1], "1",
        "constructor child pattern error code は E0001 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: constructor pattern の引数数不一致は E0006 を返す
#[test]
fn test_e2e_selfhost_typeinfer_error_match_constructor_arity_mismatch_code() {
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "constructor pattern arity mismatch error code 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "constructor pattern arity mismatch infer は失敗すべき"
    );
    assert_eq!(
        lines[1], "6",
        "constructor pattern arity mismatch error code は E0006 であるべき"
    );
}

/// selfhost TypeInfer.ls テスト: ast-pat-constructor の未定義 ctor も E0001 を返す
#[test]
fn test_e2e_selfhost_typeinfer_error_match_pat_constructor_tag_undefined_code() {
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
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
