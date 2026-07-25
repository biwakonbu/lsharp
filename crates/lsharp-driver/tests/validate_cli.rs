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
fn validate_source_reports_unknown_without_contract_evidence() {
    let path = source_path(
        "source-unknown",
        r#"
        (defn cancel []
          :intent "intent:checkout/safe-cancel" "Users can cancel an order"
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :motivates "intent:checkout/safe-cancel" "claim:checkout/cancel-rejects-shipped"
          true)
        "#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            "--source",
            path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("lsharp validate --source should run");
    fs::remove_file(&path).ok();

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stderr.is_empty(),
        "unexpected stderr: {:?}",
        output.stderr
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(value["status"], "unknown");
    assert_eq!(
        value["trace_gaps"][0]["code"],
        "trace-gap.claim-without-test"
    );
    assert!(value.get("verified").is_none());
}

#[test]
fn validate_source_rejects_orphan_edges_as_input_errors() {
    let path = source_path(
        "source-orphan",
        r#"(defn cancel [] :motivates "intent:checkout/missing" "claim:checkout/cancel" true)"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args(["validate", "--source", path.to_str().unwrap()])
        .output()
        .expect("lsharp validate --source should run");
    fs::remove_file(&path).ok();

    assert_ne!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).contains("source intent edge"));
}

#[test]
fn validate_source_tested_by_closes_claim_trace_gap() {
    let path = source_path(
        "source-tested-by",
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :tested-by "claim:checkout/cancel-rejects-shipped" "contract:checkout/cancel-case"
          true)
        "#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            "--source",
            path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("lsharp validate --source should run");
    fs::remove_file(&path).ok();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(value["status"], "unknown");
    assert_eq!(value["trace_gaps"].as_array().unwrap().len(), 0);
}

#[test]
fn validate_source_rejects_evidence_edges_without_registry() {
    let path = source_path(
        "source-supports",
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :supports "evidence:checkout/cancel-observation" "claim:checkout/cancel-rejects-shipped"
          true)
        "#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args(["validate", "--source", path.to_str().unwrap()])
        .output()
        .expect("lsharp validate --source should run");
    fs::remove_file(&path).ok();

    assert_ne!(output.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&output.stderr).contains("evidence registry"));
}

#[test]
fn validate_source_accepts_registered_evidence_edges() {
    let path = source_path(
        "source-evidence",
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :tested-by "claim:checkout/cancel-rejects-shipped" "contract:checkout/cancel-case"
          :evidence "evidence:checkout/cancel-observation"
            :subject "claim:checkout/cancel-rejects-shipped"
            :method "case"
            :outcome "pass"
            :runner "cargo-test"
            :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef"
            :artifact-digest "sha256:abc123"
            :cases 1
            :seed 42
            :generator "checkout-cancel-fixture"
            :producer "lsharp-test"
            :tool-version "0.2.0"
            :timestamp "2026-07-25T00:00:00Z"
            :independence "same-author"
          :supports "evidence:checkout/cancel-observation" "claim:checkout/cancel-rejects-shipped"
          true)
        "#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            "--source",
            path.to_str().unwrap(),
            "--format",
            "json",
        ])
        .output()
        .expect("lsharp validate --source should run");
    fs::remove_file(&path).ok();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).expect("valid JSON");
    assert_eq!(value["status"], "unknown");
    assert_eq!(value["trace_gaps"].as_array().unwrap().len(), 0);
}

#[test]
fn validate_source_emits_manifest_without_mixing_report_stdout() {
    let source = source_path(
        "source-emit-manifest",
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :evidence "evidence:checkout/cancel-observation"
            :subject "claim:checkout/cancel-rejects-shipped"
            :method "property"
            :outcome "pass"
            :runner "cargo-test"
            :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef"
            :artifact-digest "sha256:abc123"
            :cases 3
            :seed 42
            :generator "checkout-cancel-fixture"
            :shrinks [8 3 1]
            :coverage [("negative" 2) ("positive" 1)]
            :producer "lsharp-test"
            :tool-version "0.2.0"
            :timestamp "2026-07-25T00:00:00Z"
            :independence "same-author"
          :supports "evidence:checkout/cancel-observation" "claim:checkout/cancel-rejects-shipped"
          true)
        "#,
    );
    let manifest = project_dir("source-emit-manifest-output").join("intent-graph.json");
    fs::create_dir_all(manifest.parent().unwrap()).expect("manifest parent should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            "--source",
            source.to_str().unwrap(),
            "--format",
            "json",
            "--emit-manifest",
            manifest.to_str().unwrap(),
        ])
        .output()
        .expect("lsharp validate --source --emit-manifest should run");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout)
        .expect("stdout は validation report JSON のままであるべき");
    let manifest_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).expect("manifest が出力されるべき"))
            .expect("manifest は valid JSON であるべき");
    fs::remove_file(&source).ok();
    fs::remove_dir_all(manifest.parent().unwrap()).ok();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    assert_eq!(report["status"], "unknown");
    assert_eq!(manifest_value["schema_version"], 1);
    assert_eq!(
        manifest_value["evidence"][0]["execution"]["sampling"]["shrinks"],
        serde_json::json!([8, 3, 1])
    );
    assert_eq!(
        manifest_value["evidence"][0]["execution"]["sampling"]["coverage"],
        serde_json::json!({"negative": 2, "positive": 1})
    );
    assert_eq!(manifest_value["edges"].as_array().unwrap().len(), 1);
}

#[test]
fn validate_source_does_not_emit_manifest_for_adapter_errors() {
    let source = source_path(
        "source-emit-manifest-error",
        r#"(defn cancel []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :supports "evidence:checkout/missing" "claim:checkout/cancel"
          true)"#,
    );
    let manifest = project_dir("source-emit-manifest-error-output").join("intent-graph.json");
    fs::create_dir_all(manifest.parent().unwrap()).expect("manifest parent should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            "--source",
            source.to_str().unwrap(),
            "--emit-manifest",
            manifest.to_str().unwrap(),
        ])
        .output()
        .expect("lsharp validate --source --emit-manifest should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    fs::remove_file(&source).ok();
    fs::remove_dir_all(manifest.parent().unwrap()).ok();

    assert!(!output.status.success());
    assert!(stderr.contains("evidence registry"));
    assert!(
        !manifest.exists(),
        "adapter error 時に manifest を作らないべき"
    );
}

#[test]
fn validate_manifest_input_can_emit_normalized_manifest() {
    let input = manifest_path(
        "emit-manifest-input",
        include_str!("fixtures/intent-graph-pass.json"),
    );
    let output_dir = project_dir("emit-manifest-input-output");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    let output_path = output_dir.join("intent-graph.json");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            input.to_str().unwrap(),
            "--format",
            "json",
            "--emit-manifest",
            output_path.to_str().unwrap(),
        ])
        .output()
        .expect("lsharp validate --emit-manifest should run");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("valid report JSON");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&output_path).expect("manifest should be emitted"))
            .expect("valid manifest JSON");
    fs::remove_file(&input).ok();
    fs::remove_dir_all(&output_dir).ok();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(report["status"], "pass");
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["nodes"].as_array().unwrap().len(), 2);
}

#[cfg(unix)]
#[test]
fn validate_manifest_emit_replaces_symlink_without_following_target() {
    use std::os::unix::fs::symlink;

    let input = manifest_path(
        "emit-manifest-symlink-input",
        include_str!("fixtures/intent-graph-pass.json"),
    );
    let output_dir = project_dir("emit-manifest-symlink-output");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    let sentinel = output_dir.join("sentinel.json");
    let output_path = output_dir.join("intent-graph.json");
    fs::write(&sentinel, b"keep-this-content").expect("sentinel should be writable");
    symlink(&sentinel, &output_path).expect("manifest output symlink should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            input.to_str().unwrap(),
            "--format",
            "json",
            "--emit-manifest",
            output_path.to_str().unwrap(),
        ])
        .output()
        .expect("lsharp validate --emit-manifest should run");
    let manifest = fs::read(&output_path).expect("manifest should replace the symlink");
    let sentinel_contents = fs::read(&sentinel).expect("symlink target should remain readable");
    fs::remove_file(&input).ok();
    fs::remove_dir_all(&output_dir).ok();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(sentinel_contents, b"keep-this-content");
    let manifest_value: serde_json::Value =
        serde_json::from_slice(&manifest).expect("manifest should be valid JSON");
    assert_eq!(manifest_value["schema_version"], 1);
}

#[test]
fn validate_source_cannot_be_combined_with_manifest_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args(["validate", "intent-graph.json", "--source", "source.ls"])
        .output()
        .expect("lsharp validate should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be used with") || stderr.contains("conflict"));
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

fn source_path(name: &str, body: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("lsharp-validate-{name}-{nonce}.ls"));
    fs::write(&path, body).expect("source should be writable");
    path
}
