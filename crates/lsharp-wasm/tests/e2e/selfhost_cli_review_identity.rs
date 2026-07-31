use super::support::*;
use std::path::PathBuf;

fn identity_text_fixture_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "lsharp_test_embedded_cli_identity_text_{}",
        std::process::id()
    ))
}

/// EC-M3-05: optional trust/lifecycle digest は text report で `-` として明示する。
#[test]
fn test_e2e_selfhost_embedded_cli_validate_text_projects_optional_identity_as_dash() {
    let source = r#"
(defn review []
  :review "review:checkout/reviewer-001" "sha256:review-001" "redacted"
  true)
"#;
    let dir = identity_text_fixture_dir();
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(dir.join("input.ls"), source).expect("fixture input.ls の書き込みに失敗");

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
                "text",
                "--review-subject-digest",
                "sha256:graph",
                "--review-source-commit",
                "commit-1",
                "--review-artifact-digest",
                "sha256:artifact",
                "--review-now",
                "2026-08-15T00:00:00Z",
            ],
            "",
        )
        .expect("EmbeddedCli text identity の実行に失敗")
    });
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        output.exit_code, 2,
        "未検証 review は identity を付けても unknown のままにするべき: stdout={:?}",
        output.stdout
    );
    assert_eq!(
        output.stdout.trim_end(),
        "status: unknown\n\
open-questions: 0\n\
independent-reviews: 0\n\
contradicting-observations: 0\n\
stale-reviews: 0\n\
stale-evidence: 0\n\
review-evidence-identity: subject=sha256:graph source=commit-1 artifact=sha256:artifact trust-store=- lifecycle=- now=2026-08-15T00:00:00Z",
        "optional identity の text report は Rust oracle と同じ deterministic projection であるべき"
    );
}
