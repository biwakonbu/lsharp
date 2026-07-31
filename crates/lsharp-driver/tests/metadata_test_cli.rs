use serde_json::Value;
use std::process::{Command, Output};

fn run_test_json(name: &str, source_text: &str) -> Output {
    let root = std::env::temp_dir().join(format!(
        "lsharp_metadata_test_json_{}_{}",
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("metadata test fixture directory の作成に失敗");
    let source = root.join("input.ls");
    std::fs::write(&source, source_text).expect("metadata test fixture の書き込みに失敗");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "test",
            source.to_str().expect("fixture path は UTF-8 であるべき"),
            "--format",
            "json",
        ])
        .output()
        .expect("lsharp test --format json の実行に失敗");
    let _ = std::fs::remove_dir_all(&root);
    output
}

#[test]
fn test_command_json_emits_two_axis_conformance_report() {
    let output = run_test_json(
        "passing",
        "(defn succ [x] :case [(expect (succ 1) 2)] :assert [(= (succ 1) 2)] (+ x 1))\n",
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "passing test --format json は exit 0 で終了するべき: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("JSON stdout は UTF-8 であるべき");
    let lines: Vec<&str> = stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "test --format json は report 1 行だけを返すべき"
    );
    let report: Value =
        serde_json::from_str(lines[0]).expect("test JSON report は valid JSON であるべき");
    assert!(
        report.get("verified").is_none(),
        "assurance report は top-level verified を返してはならない"
    );
    assert_eq!(report["implementation_conformance"]["status"], "pass");
    assert_eq!(
        report["implementation_conformance"]["method"],
        "explicit-case"
    );
    assert_eq!(report["implementation_conformance"]["cases"], 2);
    assert_eq!(
        report["implementation_conformance"]["coverage"]["executed"],
        2
    );
    assert_eq!(
        report["implementation_conformance"]["coverage"]["failed"],
        0
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["runner"],
        "rust"
    );
    assert_eq!(report["intent_validation"]["status"], "unknown");
}

#[test]
fn test_command_json_returns_runtime_failure_as_conformance_failure() {
    let output = run_test_json(
        "assertion-failure",
        "(defn broken [] :assert [(= 1 2)] true)\n",
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "failing test --format json は runtime failure の exit 2 を返すべき: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("JSON stdout は UTF-8 であるべき");
    let report: Value =
        serde_json::from_str(stdout.trim()).expect("failure report は valid JSON であるべき");
    assert_eq!(report["implementation_conformance"]["status"], "fail");
    assert_eq!(report["implementation_conformance"]["method"], "assert");
    assert_eq!(report["implementation_conformance"]["cases"], 1);
    assert_eq!(
        report["implementation_conformance"]["coverage"]["executed"],
        1
    );
    assert_eq!(
        report["implementation_conformance"]["coverage"]["failed"],
        1
    );
    assert_eq!(
        report["implementation_conformance"]["diagnostics"]["count"],
        0
    );
}

#[test]
fn test_command_json_projects_metadata_preflight_failure() {
    let output = run_test_json(
        "preflight-failure",
        "(defn succ [x] :invariant (+ x 1) (+ x 1))\n",
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "metadata preflight failure は JSON runtime exit 2 を返すべき: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("JSON stdout は UTF-8 であるべき");
    let report: Value =
        serde_json::from_str(stdout.trim()).expect("preflight report は valid JSON であるべき");
    assert_eq!(report["implementation_conformance"]["status"], "fail");
    assert_eq!(report["implementation_conformance"]["method"], "none");
    assert_eq!(report["implementation_conformance"]["cases"], 0);
    assert_eq!(
        report["implementation_conformance"]["coverage"]["failed"],
        1
    );
    assert_eq!(
        report["implementation_conformance"]["diagnostics"]["count"],
        1
    );
    assert_eq!(
        report["implementation_conformance"]["diagnostics"]["firstErrorCode"],
        1002
    );
}
