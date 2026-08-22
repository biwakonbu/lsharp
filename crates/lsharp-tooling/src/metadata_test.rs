use std::path::Path;

pub use lsharp_wasm::test_runner::TestResult;

use crate::diagnostics::driver_io_error;
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
        lsharp_types::metadata_check::TestKind::Assertion => "assertion",
        lsharp_types::metadata_check::TestKind::Example => "example",
        lsharp_types::metadata_check::TestKind::Invariant => "invariant",
        lsharp_types::metadata_check::TestKind::Property => "property",
    }
}

/// `:example` / `:invariant` metadata tests を実行する。
pub fn run_metadata_tests(file: &Path) -> miette::Result<MetadataTestRun> {
    let source = std::fs::read_to_string(file)
        .map_err(|e| driver_io_error(format!("{}: {}", file.display(), e)))?;
    let program = lsharp_syntax::parse(&source)
        .map_err(|e| miette::miette!("[{}] metadata test parse に失敗しました: {e}", e.code()))?;
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
        .map_err(|e| miette::miette!("[{}] テストプログラムのパースに失敗: {e}", e.code()))?;

    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&test_program)
        .map_err(|e| miette::miette!("[{}] テストプログラムの型チェックに失敗: {e}", e.code()))?;
    let expr_type_results = infer.expr_type_results_snapshot();

    let mut lower = lsharp_ir::lower::Lower::new();
    let module = lower
        .lower_program_with_expr_types(&test_program, &type_results, &expr_type_results)
        .map_err(|e| miette::miette!("[{}] テストプログラムの IR 変換に失敗: {e}", e.code()))?;

    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module)
        .map_err(|e| miette::miette!("[{}] テストプログラムの Wasm 生成に失敗: {e}", e.code()))?;
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes)
        .map_err(|e| miette::miette!("テスト実行に失敗: {e}"))?;
    let results = lsharp_wasm::test_runner::parse_test_output(&output, &tests, &program);

    Ok(MetadataTestRun { results })
}

#[cfg(test)]
#[path = "metadata_test_tests.rs"]
mod tests;
