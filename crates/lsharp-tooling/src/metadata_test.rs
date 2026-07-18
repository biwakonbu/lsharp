use std::path::Path;

pub use lsharp_wasm::test_runner::TestResult;

use lsharp_types::metadata_contract::{ExecutableContract, inventory_contract_suites};

/// metadata test 実行結果。
#[derive(Debug, Clone)]
pub struct MetadataTestRun {
    pub results: Vec<TestResult>,
}

impl MetadataTestRun {
    pub fn has_tests(&self) -> bool {
        !self.results.is_empty()
    }

    pub fn total(&self) -> usize {
        self.results.len()
    }

    pub fn passed(&self) -> usize {
        self.results.iter().filter(|result| result.passed).count()
    }

    pub fn failed(&self) -> usize {
        self.results.iter().filter(|result| !result.passed).count()
    }
}

/// metadata test 種別を CLI 表示向けに整形する。
pub fn test_kind_label(kind: &lsharp_types::metadata_check::TestKind) -> &'static str {
    match kind {
        lsharp_types::metadata_check::TestKind::Case => "case",
        lsharp_types::metadata_check::TestKind::Example => "example",
        lsharp_types::metadata_check::TestKind::Invariant => "invariant",
        lsharp_types::metadata_check::TestKind::Property => "property",
    }
}

/// `:example` / `:invariant` metadata tests を実行する。
pub fn run_metadata_tests(file: &Path) -> miette::Result<MetadataTestRun> {
    let source =
        std::fs::read_to_string(file).map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
    let program = lsharp_syntax::parse(&source).map_err(|e| miette::miette!("{e}"))?;
    if let Some(diagnostic) = lsharp_types::metadata_check::check_metadata(&program)
        .into_iter()
        .find(|diagnostic| diagnostic.severity == lsharp_types::metadata_check::Severity::Error)
    {
        let code = if diagnostic
            .message
            .contains(":case は少なくとも 1 件の expectation")
        {
            "LS2006"
        } else if diagnostic.message.contains("vacuous") {
            "LS2005"
        } else if diagnostic
            .message
            .contains(":assert は少なくとも 1 件の predicate")
        {
            "LS2004"
        } else if diagnostic.message.contains("未定義の識別子")
            || diagnostic.message.contains("未定義の変数")
        {
            "LS1001"
        } else {
            "LS1002"
        };
        return Err(miette::miette!("[{code}] {diagnostic}"));
    }
    let suites = inventory_contract_suites(&program)
        .map_err(|error| miette::miette!("metadata contract inventory に失敗: {error}"))?;
    let tests = lsharp_types::metadata_check::generate_tests(&program);
    let property_count = suites
        .iter()
        .flat_map(|suite| suite.executable())
        .filter(|contract| matches!(contract, ExecutableContract::Property(_)))
        .count();
    let generated_property_count = tests
        .iter()
        .filter(|test| matches!(test.kind, lsharp_types::metadata_check::TestKind::Property))
        .count();
    if property_count != generated_property_count {
        let owner = suites
            .iter()
            .find_map(|suite| {
                suite
                    .executable()
                    .iter()
                    .any(|contract| matches!(contract, ExecutableContract::Property(_)))
                    .then(|| suite.owner().as_str())
            })
            .unwrap_or("anonymous");
        return Err(miette::miette!(
            "[LS3002] {owner}: canonical :property は deterministic smoke profile の範囲外です"
        ));
    }
    if tests.is_empty() {
        return Ok(MetadataTestRun {
            results: Vec::new(),
        });
    }

    let test_source = lsharp_wasm::test_runner::generate_test_program(&program, &tests);
    let test_program = lsharp_syntax::parse(&test_source)
        .map_err(|e| miette::miette!("テストプログラムのパースに失敗: {e}"))?;

    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&test_program)
        .map_err(|e| miette::miette!("テストプログラムの型チェックに失敗: {e}"))?;
    let expr_type_results = infer.expr_type_results_snapshot();

    let mut lower = lsharp_ir::lower::Lower::new();
    let module = lower
        .lower_program_with_expr_types(&test_program, &type_results, &expr_type_results)
        .map_err(|e| miette::miette!("テストプログラムの IR 変換に失敗: {e}"))?;

    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module)
        .map_err(|e| miette::miette!("テストプログラムの Wasm 生成に失敗: {e}"))?;
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes)
        .map_err(|e| miette::miette!("テスト実行に失敗: {e}"))?;
    let results = lsharp_wasm::test_runner::parse_test_output(&output, &tests, &program);

    Ok(MetadataTestRun { results })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "lsharp_tooling_metadata_{name}_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&dir).expect("temp dir creation failed");
        dir
    }

    #[test]
    fn test_run_metadata_tests_returns_empty_when_source_has_no_metadata() {
        let dir = unique_temp_dir("empty");
        let file = dir.join("Main.ls");
        fs::write(&file, "(defn add [x y] (+ x y))\n").unwrap();

        let run = run_metadata_tests(&file).expect("metadata test helper should succeed");
        assert!(!run.has_tests(), "metadata がないので tests は空のはず");
        assert_eq!(run.total(), 0);
        assert_eq!(run.passed(), 0);
        assert_eq!(run.failed(), 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_executes_example_and_invariant() {
        let dir = unique_temp_dir("success");
        let file = dir.join("Main.ls");
        fs::write(
            &file,
            r#"(defn abs
  [x]
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
"#,
        )
        .unwrap();

        let run = run_metadata_tests(&file).expect("metadata test helper should succeed");
        assert!(run.has_tests(), "metadata があるので tests が必要");
        assert_eq!(run.total(), 2);
        assert_eq!(run.passed(), 2);
        assert_eq!(run.failed(), 0);
        assert_eq!(run.results[0].name, "abs_invariant");
        assert_eq!(run.results[1].name, "abs_example_0");
        assert!(run.results.iter().all(|result| result.passed));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_reports_unknown_invariant_variable() {
        let dir = unique_temp_dir("unknown_invariant_variable");
        let file = dir.join("Main.ls");
        fs::write(
            &file,
            r#"(defn succ
  [x]
  :invariant (= result (+ missing 1))
  (+ x 1))
"#,
        )
        .unwrap();

        let error = run_metadata_tests(&file)
            .expect_err("未定義 invariant 変数は stable diagnostic で拒否されるべき");
        assert!(
            error.to_string().contains("[LS1001]"),
            "unknown invariant variable は LS1001 を返すべき: {error}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_allows_local_let_binding_in_invariant() {
        let dir = unique_temp_dir("local_let_invariant");
        let file = dir.join("Main.ls");
        fs::write(
            &file,
            r#"(defn succ
  [x]
  :invariant (let [delta 1] (= result (+ x delta)))
  (+ x 1))
"#,
        )
        .unwrap();

        let run = run_metadata_tests(&file)
            .expect("invariant の local let binding は metadata oracle で有効であるべき");
        assert_eq!(run.total(), 1);
        assert_eq!(run.passed(), 1);
        assert_eq!(run.failed(), 0);
        assert_eq!(run.results[0].name, "succ_invariant");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_rejects_empty_canonical_assertion() {
        let dir = unique_temp_dir("empty_canonical_assertion");
        let file = dir.join("Main.ls");
        fs::write(&file, "(defn noop [] :assert [] true)\n").unwrap();

        let error = run_metadata_tests(&file)
            .expect_err("空の canonical assertion を成功扱いしてはならない");
        assert!(
            error.to_string().contains("[LS2004]"),
            "空の canonical assertion は LS2004 を返すべき: {error}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_rejects_literal_true_canonical_assertion() {
        let dir = unique_temp_dir("literal_true_canonical_assertion");
        let file = dir.join("Main.ls");
        fs::write(&file, "(defn noop [] :assert [true] true)\n").unwrap();

        let error = run_metadata_tests(&file)
            .expect_err("literal true canonical assertion を成功扱いしてはならない");
        assert!(
            error.to_string().contains("[LS2005]"),
            "literal true canonical assertion は LS2005 を返すべき: {error}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_rejects_statically_true_integer_comparison() {
        let dir = unique_temp_dir("static_true_integer_comparison");
        let file = dir.join("Main.ls");
        fs::write(&file, "(defn noop [] :assert [(= 1 1)] true)\n").unwrap();

        let error =
            run_metadata_tests(&file).expect_err("静的に true な整数比較を成功扱いしてはならない");
        assert!(
            error.to_string().contains("[LS2005]"),
            "静的に true な整数比較は LS2005 を返すべき: {error}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_rejects_mismatched_canonical_case_types() {
        let dir = unique_temp_dir("mismatched_canonical_case_types");
        let file = dir.join("Main.ls");
        fs::write(&file, "(defn noop [] :case [(expect 1 true)] true)\n").unwrap();

        let error = run_metadata_tests(&file)
            .expect_err("型不一致 canonical case を成功扱いしてはならない");
        assert!(
            error.to_string().contains("[LS1002]"),
            "型不一致 canonical case は LS1002 を返すべき: {error}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_rejects_empty_canonical_case() {
        let dir = unique_temp_dir("empty_canonical_case");
        let file = dir.join("Main.ls");
        fs::write(&file, "(defn noop [] :case [] 0)\n").unwrap();

        let error =
            run_metadata_tests(&file).expect_err("空の canonical case を成功扱いしてはならない");
        assert!(
            error.to_string().contains("[LS2006]"),
            "空の canonical case は LS2006 を返すべき: {error}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_rejects_canonical_case_parameter_capture() {
        let dir = unique_temp_dir("canonical_case_parameter_capture");
        let file = dir.join("Main.ls");
        fs::write(&file, "(defn identity [x] :case [(expect x 1)] x)\n").unwrap();

        let error = run_metadata_tests(&file)
            .expect_err("case が owner parameter を暗黙 capture したように成功してはならない");
        assert!(
            error.to_string().contains("[LS1001]"),
            "case parameter capture は LS1001 を返すべき: {error}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_executes_deterministic_property_smoke() {
        let dir = unique_temp_dir("deterministic_property_smoke");
        let file = dir.join("Main.ls");
        fs::write(
            &file,
            "(defn identity [x] :property [(for-all [x Int] :cases 5 :postcondition (= result x))] x)\n",
        )
        .unwrap();

        let run = run_metadata_tests(&file).expect("deterministic property smoke は実行できるべき");
        assert_eq!(run.total(), 1);
        assert_eq!(run.passed(), 1);
        assert_eq!(run.failed(), 0);
        assert_eq!(run.results[0].name, "identity_property_0");
        assert_eq!(
            run.results[0].kind,
            lsharp_types::metadata_check::TestKind::Property
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_executes_property_precondition_and_skips_false_samples() {
        let dir = unique_temp_dir("deterministic_property_precondition");
        let file = dir.join("Main.ls");
        fs::write(
            &file,
            "(defn identity [x] :property [(for-all [value Int] :cases 5 :precondition [(>= value 0)] :postcondition (= result value))] x)\n",
        )
        .unwrap();

        let run = run_metadata_tests(&file)
            .expect("single Int precondition property は false sample を skip して実行できるべき");
        assert_eq!(run.total(), 1);
        assert_eq!(run.passed(), 1);
        assert_eq!(run.failed(), 0);
        assert_eq!(run.results[0].name, "identity_property_0");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_rejects_property_outside_deterministic_smoke_profile() {
        let dir = unique_temp_dir("unsupported_property_profile");
        let file = dir.join("Main.ls");
        fs::write(
            &file,
            "(defn identity [x] :property [(for-all [x Int] :cases 6 :postcondition (= result x))] x)\n",
        )
        .unwrap();

        let error =
            run_metadata_tests(&file).expect_err("profile 外の property を成功扱いしてはならない");
        assert!(
            error.to_string().contains("[LS3002]"),
            "profile 外の property は LS3002 を返すべき: {error}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_reports_failing_deterministic_property() {
        let dir = unique_temp_dir("failing_deterministic_property");
        let file = dir.join("Main.ls");
        fs::write(
            &file,
            "(defn identity [x] :property [(for-all [x Int] :cases 1 :postcondition (= result (+ x 1)))] x)\n",
        )
        .unwrap();

        let run = run_metadata_tests(&file).expect("failing property も結果を返すべき");
        assert_eq!(run.total(), 1);
        assert_eq!(run.passed(), 0);
        assert_eq!(run.failed(), 1);
        assert!(run.results[0].error.is_some());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_run_metadata_tests_executes_canonical_cases() {
        let dir = unique_temp_dir("canonical_case_execution");
        let file = dir.join("Main.ls");
        fs::write(
            &file,
            "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))\n",
        )
        .unwrap();

        let run = run_metadata_tests(&file).expect("canonical case runner は実行できるべき");
        assert_eq!(run.total(), 2);
        assert_eq!(run.passed(), 1);
        assert_eq!(run.failed(), 1);
        assert_eq!(run.results[0].name, "succ_case_0");
        assert_eq!(run.results[1].name, "succ_case_1");
        assert!(run.results[0].passed);
        assert!(!run.results[1].passed);

        let _ = fs::remove_dir_all(&dir);
    }
}
