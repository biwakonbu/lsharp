use super::support::*;
use std::path::PathBuf;

fn manifest_write_failure_fixture_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "lsharp_test_embedded_cli_manifest_write_failure_{}",
        std::process::id()
    ))
}

/// EC-M3-03: EmbeddedCli の manifest write failure は report を成功扱いにしない。
#[test]
fn test_e2e_selfhost_embedded_cli_validate_source_rejects_manifest_write_failure() {
    let dir = manifest_write_failure_fixture_dir();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(
        dir.join("input.ls"),
        r#"
(defn valid []
  :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
  true)
"#,
    )
    .expect("fixture input.ls の書き込みに失敗");

    let run_dir = dir.clone();
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        let wasm = compile_only(selfhost_embedded_cli_runtime_bundle());
        lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
            &wasm,
            Some(&run_dir),
            &[
                "validate",
                "--source",
                "input.ls",
                "--format",
                "json",
                "--emit-manifest",
                "missing/intent-graph.json",
            ],
            "",
        )
        .expect("EmbeddedCli validate の実行に失敗")
    });
    let manifest_exists = dir.join("missing/intent-graph.json").exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        output.exit_code, 1,
        "manifest write failure は exit 1 を返すべき"
    );
    assert!(
        output
            .stdout
            .contains("source validation manifest write failed"),
        "write failure の診断を出すべき: {}",
        output.stdout
    );
    assert!(
        !output.stdout.contains("\"status\""),
        "write failure では validation report を出さないべき: {}",
        output.stdout
    );
    assert!(
        !manifest_exists,
        "write failure では manifest を残さないべき"
    );
}
