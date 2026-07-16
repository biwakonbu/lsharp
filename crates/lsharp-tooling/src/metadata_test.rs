use std::path::Path;

pub use lsharp_wasm::test_runner::TestResult;

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
        lsharp_types::metadata_check::TestKind::Example => "example",
        lsharp_types::metadata_check::TestKind::Invariant => "invariant",
    }
}

/// `:example` / `:invariant` metadata tests を実行する。
pub fn run_metadata_tests(file: &Path) -> miette::Result<MetadataTestRun> {
    let source =
        std::fs::read_to_string(file).map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
    let program = lsharp_syntax::parse(&source).map_err(|e| miette::miette!("{e}"))?;
    if let Some(diagnostic) = lsharp_types::metadata_check::check_metadata(&program)
        .into_iter()
        .find(|diagnostic| {
            diagnostic.severity == lsharp_types::metadata_check::Severity::Error
        })
    {
        let code = if diagnostic.message.contains("未定義の識別子") {
            "LS1001"
        } else {
            "LS1002"
        };
        return Err(miette::miette!("[{code}] {diagnostic}"));
    }
    let tests = lsharp_types::metadata_check::generate_tests(&program);
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
}
