// =================================================// selfhost Lexer.ls 拡張テスト (Step 3)
// =================================================
#[test]
fn test_e2e_selfhost_metadata_check_rejects_non_bool_canonical_assertion() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn positive [] :assert [(+ 1 2)] true)")
        result (check-canonical-assertions program)
        valid-program (parse-program "(defn positive [x] (> x 0)) (defn checked [] :assert [(positive 1)] true)")
        valid-result (check-canonical-assertions valid-program)
        undefined-program (parse-program "(defn positive [] :assert [missing] true)")
        undefined-result (check-canonical-assertions undefined-program)
        parameter-program (parse-program "(defn positive [x] :assert [x] x)")
        parameter-result (check-canonical-assertions parameter-program)
        empty-program (parse-program "(defn noop [] :assert [] true)")
        empty-result (check-canonical-assertions empty-program)
        literal-true-program (parse-program "(defn noop [] :assert [true] true)")
        literal-true-result (check-canonical-assertions literal-true-program)
        static-true-program (parse-program "(defn noop [] :assert [(= 1 1)] true)")
        static-true-result (check-canonical-assertions static-true-program)
        module-empty-program (parse-program "(module Demo (defn noop [] :assert [] true))")
        module-empty-result (check-canonical-assertions module-empty-program)
        private-empty-program (parse-program "(module Demo (private (defn noop [] :assert [] true)))")
        private-empty-result (check-canonical-assertions private-empty-program)
        nested-empty-program (parse-program "(module Outer (module Inner (defn noop [] :assert [] true)))")
        nested-empty-result (check-canonical-assertions nested-empty-program)
        module-program (parse-program "(module Demo (defn positive [] :assert [(+ 1 2)] true))")
        module-result (check-canonical-assertions module-program)
        private-program (parse-program "(module Demo (private (defn positive [] :assert [(+ 1 2)] true)))")
        private-result (check-canonical-assertions private-program)
        module-helper-program (parse-program "(module Demo (defn helper [x] x) (defn positive [] :assert [(helper 1)] true))")
        module-helper-result (check-canonical-assertions module-helper-program)
        private-helper-program (parse-program "(module Demo (private (defn helper [x] x)) (defn positive [] :assert [(helper 1)] true))")
        private-helper-result (check-canonical-assertions private-helper-program)
        nested-module-program (parse-program "(module Outer (module Inner (defn positive [] :assert [(+ 1 2)] true)))")
        nested-module-result (check-canonical-assertions nested-module-program)
        nested-helper-program (parse-program "(module Outer (module Inner (defn helper [x] x) (defn positive [] :assert [(helper 1)] true)))")
        nested-helper-result (check-canonical-assertions nested-helper-program)]
    (do
      (print (vector-get result 0))
      (print (vector-get result 1))
      (print (vector-get valid-result 0))
      (print (vector-get valid-result 1))
      (print (vector-get undefined-result 0))
      (print (vector-get undefined-result 1))
      (print (vector-get parameter-result 0))
      (print (vector-get parameter-result 1))
      (print (vector-get empty-result 0))
      (print (vector-get empty-result 1))
      (print (vector-get literal-true-result 0))
      (print (vector-get literal-true-result 1))
      (print (vector-get static-true-result 0))
      (print (vector-get static-true-result 1))
      (print (vector-get module-empty-result 0))
      (print (vector-get module-empty-result 1))
      (print (vector-get private-empty-result 0))
      (print (vector-get private-empty-result 1))
      (print (vector-get nested-empty-result 0))
      (print (vector-get nested-empty-result 1))
      (print (vector-get module-result 0))
      (print (vector-get module-result 1))
      (print (vector-get private-result 0))
      (print (vector-get private-result 1))
      (print (vector-get module-helper-result 0))
      (print (vector-get module-helper-result 1))
      (print (vector-get private-helper-result 0))
      (print (vector-get private-helper-result 1))
      (print (vector-get nested-module-result 0))
      (print (vector-get nested-module-result 1))
      (print (vector-get nested-helper-result 0))
      (print (vector-get nested-helper-result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "1", "1002", "0", "0", "1", "1001", "1", "1001", "1", "2004", "1", "2005", "1", "2005",
            "1", "2004", "1", "2004", "1", "2004", "1", "1002", "1", "1002", "1", "1002", "1",
            "1002", "1", "1002", "1", "1002",
        ]
    );
}

#[test]
fn test_e2e_selfhost_metadata_check_rejects_invalid_canonical_case() {
    let harness = r#"
(defn main []
  (let [valid-program (parse-program "(defn noop [] :case [(expect 1 1) (expect true false)] true)")
        valid-result (check-canonical-cases valid-program)
        string-program (parse-program "(defn noop [] :case [(expect \"a\" \"a\")] true)")
        string-result (check-canonical-cases string-program)
        mismatch-program (parse-program "(defn noop [] :case [(expect 1 true)] true)")
        mismatch-result (check-canonical-cases mismatch-program)
        parameter-program (parse-program "(defn identity [x] :case [(expect x 1)] x)")
        parameter-result (check-canonical-cases parameter-program)
        empty-program (parse-program "(defn noop [] :case [] 0)")
        empty-result (check-canonical-cases empty-program)
        module-program (parse-program "(module Demo (defn noop [] :case [(expect 1 true)] true))")
        module-result (check-canonical-cases module-program)
        private-program (parse-program "(module Demo (private (defn noop [] :case [] 0)))")
        private-result (check-canonical-cases private-program)]
    (do
      (print (vector-get valid-result 0))
      (print (vector-get valid-result 1))
      (print (vector-get string-result 0))
      (print (vector-get string-result 1))
      (print (vector-get mismatch-result 0))
      (print (vector-get mismatch-result 1))
      (print (vector-get parameter-result 0))
      (print (vector-get parameter-result 1))
      (print (vector-get empty-result 0))
      (print (vector-get empty-result 1))
      (print (vector-get module-result 0))
      (print (vector-get module-result 1))
      (print (vector-get private-result 0))
      (print (vector-get private-result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "0", "0", "1", "1002", "1", "1002", "1", "1001", "1", "2006", "1", "1002", "1", "2006",
        ],
        "selfhost case checker は Rust metadata checker と同じ failure boundary を返すべき"
    );
}

#[test]
fn test_e2e_selfhost_metadata_migration_classifies_legacy_forms() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn succ [x] :example [(succ 0) (= (succ 1) 2)] :invariant (= result (+ x 1)) (+ x 1))")
        result (classify-legacy-contracts program)]
    (do
      (print (vector-length result))
      (print (vector-get (vector-get result 0) 0))
      (print (vector-get (vector-get result 0) 1))
      (print (vector-get (vector-get result 1) 0))
      (print (vector-get (vector-get result 1) 1))
      (print (vector-get (vector-get result 2) 0))
      (print (vector-get (vector-get result 2) 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["3", "2001", "1", "2001", "2", "2002", "3"],
        "selfhost migration classifier は Rust classifier と同じ disposition を返すべき"
    );
}

#[test]
fn test_e2e_selfhost_metadata_migration_marks_polymorphic_example_manual_review() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn identity [x] :example [(fn [value] value)] x)")
        result (classify-legacy-contracts program)
        row (vector-get result 0)]
    (do
      (print (vector-length result))
      (print (vector-get row 0))
      (print (vector-get row 1))
      (print (vector-get row 6))
      (print-string (vector-get row 5))
      (print-string "\n")
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "1",
            "2003",
            "4",
            "1",
            "legacy :example は silent conversion できません。manual review が必要です: 型 (t1000) -> t1000 を concrete に確定できません",
        ],
        "selfhost migration classifier は未確定の polymorphic :example を Rust と同じ manual-review 境界へ送るべき"
    );
}

#[test]
fn test_e2e_selfhost_negative_int_parses_as_int() {
    let harness = r#"
(defn main []
  (let [program (parse-program "-1")
        expr (vector-get program 0)]
    (do
      (print (vector-length program))
      (print (vector-get expr 0))
      (print (vector-get expr 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines[0], "1", "program は 1 式を返すべき");
    assert_eq!(lines[1], "1", "-1 は int node (tag=1) であるべき");
    assert_eq!(lines[2], "-1", "-1 の値が保持されるべき");
}

/// parser-to-inference bundle: parser が保持した defn signature を type inference が検査する
#[test]
fn test_e2e_selfhost_parser_typed_defn_signature_rejects_mismatch() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn invalid [(: x Bool)] : Int x)")
        analysis (infer-program-analysis program)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-code analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "6"],
        "parser 経由の typed defn signature は型不一致を診断するべき"
    );
}

/// parser-to-inference bundle: applied / function signature を自己ホスト型推論へ渡せる
#[test]
fn test_e2e_selfhost_parser_typed_defn_signature_unifies_type_app_and_fun() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn ref-id [(: x (Ref (Vector Int)))] : (Ref (Vector Int)) x) (defn fn-id [(: f (-> Int String Bool))] : (-> Int String Bool) f)")
        analysis (infer-program-analysis program)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-code analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0"],
        "parser 経由の TypeApp / TypeFun signature は型推論に渡せるべき"
    );
}

/// parser-to-inference bundle: TypeVar signature は同名を共有し異名を別変数として束縛する
#[test]
fn test_e2e_selfhost_parser_typed_defn_signature_unifies_type_var() {
    let harness = r#"
(defn main []
  (let [valid-analysis (infer-program-analysis (parse-program "(defn id [(: x a)] : a x)"))
        invalid-analysis (infer-program-analysis (parse-program "(defn invalid [(: x a)] : b x)"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid-analysis))
      (print (infer-program-analysis-first-error-code valid-analysis))
      (print (infer-program-analysis-diagnostic-count invalid-analysis))
      (print (infer-program-analysis-first-error-code invalid-analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "0", "0"],
        "parser 経由の TypeVar signature は signature scope 内の型変数を Rust と同じく解決するべき"
    );
}

/// parser-to-inference bundle: defn signature の型変数を具体化ごとに一般化する
#[test]
fn test_e2e_selfhost_scoped_type_var_defn_signature_is_polymorphic() {
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(defn id [(: x a)] : a x) (defn main [] (do (print (id 42)) (print (id true)) 0))"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-code analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0"],
        "defn signature の scoped type variable は具体化ごとに多相であるべき"
    );
}

/// parser-to-inference bundle: 1 signature 内の複数 type variable を独立に一般化する
#[test]
fn test_e2e_selfhost_scoped_multiple_type_vars_defn_signature_is_polymorphic() {
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(defn choose-first [(: x a) (: y b)] : a x) (defn main [] (do (print (choose-first 42 true)) (print (choose-first true 42)) 0))"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-code analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0"],
        "1 つの defn signature 内の複数 scoped type variable は独立に具体化できるべき"
    );
}

/// parser-to-inference bundle: program analysis は最初の defn の型を保持する
#[test]
fn test_e2e_selfhost_program_analysis_preserves_first_defn_type() {
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis (parse-program "(defn main [] 42)"))
        fun-ty (infer-program-analysis-type analysis)
        ty (ty-fr fun-ty)]
    (do
      (print (ty-tag fun-ty))
      (print (ty-tag ty))
      (print (ty-name ty))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["3", "1", "100"],
        "0 引数 defn は Unit -> Int として登録され (I-45)、戻り型に Int を保持すべき"
    );
}
