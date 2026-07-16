//! EC-M1-02 の migration baseline として Rust metadata runner の現行挙動を固定する。
//! この snapshot は v0.2 の最終 contract semantics ではなく、checker/runner 差分の inventory。

use lsharp_syntax::parse;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const BOOLEAN_CONTRACTS: &str = include_str!("fixtures/metadata/runner_boolean_contracts.ls");
const NON_BOOL_EXAMPLE: &str = include_str!("fixtures/metadata/runner_non_bool_example.ls");

struct TempSourceFile {
    path: PathBuf,
}

impl TempSourceFile {
    fn new(label: &str, source: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time は UNIX epoch 以降であるべき")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "lsharp_metadata_runner_{label}_{}_{}.ls",
            std::process::id(),
            nonce
        ));
        fs::write(&path, source).expect("一時 L# source を書き込めるべき");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempSourceFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn result_inventory(result: &lsharp_tooling::metadata_test::TestResult) -> Value {
    json!({
        "name": result.name,
        "function_name": result.function_name,
        "kind": lsharp_tooling::metadata_test::test_kind_label(&result.kind),
        "passed": result.passed,
        "error": result.error.as_deref(),
    })
}

fn first_bracketed_code(message: &str) -> Option<&str> {
    let start = message.find('[')? + 1;
    let end = start + message[start..].find(']')?;
    Some(&message[start..end])
}

#[test]
fn rust_runner_metadata_semantics_are_snapshotted() {
    let boolean_program =
        parse(BOOLEAN_CONTRACTS).expect("Bool contract fixture は parse できるべき");
    let boolean_tests = lsharp_types::metadata_check::generate_tests(&boolean_program);
    let generated_boolean_source =
        lsharp_wasm::test_runner::generate_test_program(&boolean_program, &boolean_tests);
    let observed_samples: Vec<_> = ["0", "1", "5", "-1", "42"]
        .into_iter()
        .filter(|sample| generated_boolean_source.contains(&format!("(let [x {sample}]")))
        .collect();

    let boolean_file = TempSourceFile::new("boolean", BOOLEAN_CONTRACTS);
    let boolean_run = lsharp_tooling::metadata_test::run_metadata_tests(boolean_file.path())
        .expect("Bool contract fixture は実行結果を返すべき");

    let non_bool_program =
        parse(NON_BOOL_EXAMPLE).expect("non-Bool example fixture は parse できるべき");
    let non_bool_diagnostics = lsharp_types::metadata_check::check_metadata(&non_bool_program);
    let non_bool_tests = lsharp_types::metadata_check::generate_tests(&non_bool_program);
    let generated_non_bool_source =
        lsharp_wasm::test_runner::generate_test_program(&non_bool_program, &non_bool_tests);
    let non_bool_file = TempSourceFile::new("non_bool", NON_BOOL_EXAMPLE);
    let non_bool_run = lsharp_tooling::metadata_test::run_metadata_tests(non_bool_file.path());
    let (non_bool_succeeded, non_bool_error) = match non_bool_run {
        Ok(_) => (true, None),
        Err(error) => (false, Some(error.to_string())),
    };
    let non_bool_code = non_bool_error.as_deref().and_then(first_bracketed_code);

    let snapshot = json!({
        "inventory_status": "current_behavior_not_final_v0_2_contract",
        "supported_boolean_contracts": {
            "generated_test_order": boolean_tests
                .iter()
                .map(|test| lsharp_tooling::metadata_test::test_kind_label(&test.kind))
                .collect::<Vec<_>>(),
            "generated_print_guard_count": generated_boolean_source.matches("(print (if ").count(),
            "generated_invariant_samples": observed_samples,
            "public_summary": {
                "total": boolean_run.total(),
                "passed": boolean_run.passed(),
                "failed": boolean_run.failed(),
            },
            "public_results": boolean_run
                .results
                .iter()
                .map(result_inventory)
                .collect::<Vec<_>>(),
        },
        "non_bool_example_boundary": {
            "checker_diagnostic_count": non_bool_diagnostics.len(),
            "generated_as_if_condition": generated_non_bool_source
                .contains("(print (if (succ 0) 1 0))"),
            "runner_succeeded": non_bool_succeeded,
            "runner_error_code": non_bool_code,
            "runner_error_uses_public_ls_code": non_bool_code
                .is_some_and(|code| code.starts_with("LS")),
            "runner_error_mentions_type_check_phase": non_bool_error
                .as_deref()
                .is_some_and(|error| error.contains("型チェック")),
        },
    });

    insta::assert_snapshot!(
        "rust_runner_metadata_semantics",
        serde_json::to_string_pretty(&snapshot).expect("snapshot JSON を serialize できるべき")
    );
}
