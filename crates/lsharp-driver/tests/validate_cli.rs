use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn manifest_path(name: &str, body: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("lsharp-validate-{name}-{nonce}.json"));
    fs::write(&path, body).expect("manifest should be writable");
    path
}

#[test]
fn validate_json_reports_unknown_without_verified_shortcut() {
    let path = manifest_path(
        "unknown",
        r#"{"schema_version":1,"nodes":[],"evidence":[],"edges":[]}"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args(["validate", path.to_str().unwrap(), "--format", "json"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_file(&path).ok();

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(value["status"], "unknown");
    assert!(value.get("verified").is_none());
}

#[test]
fn validate_rejects_invalid_manifest_with_nonzero_status() {
    let path = manifest_path(
        "invalid",
        r#"{"schema_version":99,"nodes":[],"evidence":[],"edges":[]}"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args(["validate", path.to_str().unwrap()])
        .output()
        .expect("lsharp validate should run");
    fs::remove_file(&path).ok();

    assert_ne!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).contains("schema_version"));
}

#[test]
fn validate_passes_with_zero_exit_code_for_complete_manifest() {
    let path = manifest_path("pass", include_str!("fixtures/intent-graph-pass.json"));

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args(["validate", path.to_str().unwrap(), "--format", "json"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_file(&path).ok();

    assert_eq!(output.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(value["status"], "pass");
    assert!(value.get("verified").is_none());
}

#[test]
fn validate_fail_has_distinct_exit_code_for_contradiction() {
    let path = manifest_path("fail", include_str!("fixtures/intent-graph-fail.json"));

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args(["validate", path.to_str().unwrap(), "--format", "text"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_file(&path).ok();

    assert_eq!(output.status.code(), Some(1));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.starts_with("status: fail\n"));
    assert!(text.contains("contradicting-observations: 1"));
    assert!(!text.contains("verified"));
}

#[test]
fn cli_help_lists_validate_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("--help")
        .output()
        .expect("lsharp help should run");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("validate"));
}

#[test]
fn validate_uses_project_config_manifest_when_path_is_omitted() {
    let project = project_dir("config-pass");
    fs::create_dir_all(project.join("docs")).expect("project docs should be writable");
    fs::write(
        project.join("lsharp.toml"),
        "[validation]\nmanifest = \"docs/intent-graph.json\"\n",
    )
    .expect("project config should be writable");
    fs::write(
        project.join("docs/intent-graph.json"),
        include_str!("fixtures/intent-graph-pass.json"),
    )
    .expect("manifest should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(&project)
        .args(["validate", "--format", "json"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_dir_all(&project).ok();

    assert_eq!(output.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(value["status"], "pass");
}

#[test]
fn validate_discovers_project_config_from_nested_directory() {
    let project = project_dir("config-nested");
    fs::create_dir_all(project.join("docs")).expect("project docs should be writable");
    fs::create_dir_all(project.join("src/nested")).expect("nested directory should be writable");
    fs::write(
        project.join("lsharp.toml"),
        "[validation]\nmanifest = \"docs/intent-graph.json\"\n",
    )
    .expect("project config should be writable");
    fs::write(
        project.join("docs/intent-graph.json"),
        include_str!("fixtures/intent-graph-pass.json"),
    )
    .expect("manifest should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(project.join("src/nested"))
        .args(["validate", "--format", "json"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_dir_all(&project).ok();

    assert_eq!(output.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(value["status"], "pass");
}

#[test]
fn validate_without_manifest_configuration_fails_closed() {
    let project = project_dir("config-missing");
    fs::create_dir_all(&project).expect("project should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(&project)
        .args(["validate"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_dir_all(&project).ok();

    assert_ne!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).contains("[validation].manifest"));
}

#[test]
fn validate_rejects_project_config_path_traversal() {
    let project = project_dir("config-traversal");
    fs::create_dir_all(&project).expect("project should be writable");
    fs::write(
        project.join("lsharp.toml"),
        "[validation]\nmanifest = \"../outside.json\"\n",
    )
    .expect("project config should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(&project)
        .args(["validate"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_dir_all(&project).ok();

    assert_ne!(output.status.code(), Some(0));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("project root") || stderr.contains(".."));
}

fn project_dir(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "lsharp-validate-{name}-{}-{nonce}",
        std::process::id()
    ))
}
