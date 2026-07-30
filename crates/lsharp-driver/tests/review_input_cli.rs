use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

fn project_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let project = std::env::temp_dir().join(format!("lsharp-review-input-{name}-{nonce}"));
    fs::create_dir_all(&project).expect("project directory should be writable");
    fs::write(
        project.join("lsharp.toml"),
        "[project]\nname = \"review-input\"\n",
    )
    .expect("project config should be writable");
    fs::write(
        project.join("manifest.json"),
        r#"{"schema_version":1,"nodes":[],"evidence":[],"edges":[]}"#,
    )
    .expect("manifest should be writable");
    project
}

fn review_wire(trust_store: Option<&str>, unknown_field: bool) -> String {
    let trust_store = trust_store
        .map(|value| format!(",\"trust_store\":[{value}]"))
        .unwrap_or_default();
    let unknown_field = if unknown_field {
        ",\"unexpected\":true"
    } else {
        ""
    };
    format!(
        "{{\"schema_version\":1,\"attestations\":[],\"lifecycle\":[]{trust_store}{unknown_field}}}"
    )
}

fn trust_key() -> &'static str {
    "{\"provider\":\"github\",\"key_id\":\"org/reviews-2026\",\"algorithm\":\"ed25519\",\"public_key\":\"BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc\"}"
}

fn run_validate(project: &Path, trust_store: Option<&Path>, lifecycle: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
    command
        .current_dir(project)
        .args(["validate", "manifest.json", "--format", "json"]);
    if let Some(path) = trust_store {
        command.args(["--trust-store", path.to_str().expect("UTF-8 path")]);
    }
    if let Some(path) = lifecycle {
        command.args(["--review-lifecycle", path.to_str().expect("UTF-8 path")]);
    }
    command.output().expect("lsharp validate should run")
}

fn assert_input_error(output: Output, fragments: &[&str]) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "unexpected exit: {stderr}");
    assert!(
        output.stdout.is_empty(),
        "review input errors must not emit a report: {:?}",
        output.stdout
    );
    for fragment in fragments {
        assert!(
            stderr.contains(fragment),
            "diagnostic missing {fragment:?}: {stderr}"
        );
    }
}

#[test]
fn validate_projects_explicit_attestation_state_to_report_and_manifest() {
    let project = project_dir("attestation-state");
    fs::write(
        project.join("manifest.json"),
        r#"{
          "schema_version": 1,
          "nodes": [],
          "reviews": [{
            "namespace": "checkout",
            "key": "reviewer-001",
            "provenance_digest": "sha256:review",
            "visibility": "public"
          }],
          "evidence": [],
          "edges": []
        }"#,
    )
    .expect("manifest should be writable");
    let trust_store = project.join("trust.json");
    fs::write(
        &trust_store,
        r#"{
          "schema_version": 1,
          "attestations": [{
            "review_id": "review:checkout/reviewer-001",
            "subject_digest": "sha256:graph",
            "source_commit": "commit-1",
            "provenance_digest": "sha256:review",
            "provider": "github",
            "key_id": "org/reviews-2026",
            "algorithm": "ed25519",
            "signature": "AQID",
            "issued_at": "2026-07-29T00:00:00Z",
            "sequence": 1
          }],
          "lifecycle": [],
          "trust_store": []
        }"#,
    )
    .expect("attestation wire should be writable");
    let emitted_manifest = project.join("verified-manifest.json");
    let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
    command.current_dir(&project).args([
        "validate",
        "manifest.json",
        "--format",
        "json",
        "--trust-store",
        "trust.json",
        "--emit-manifest",
        "verified-manifest.json",
    ]);
    let output = command.output().expect("lsharp validate should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "unexpected exit: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be JSON");
    assert_eq!(
        report["review_verifications"],
        serde_json::json!([{
            "review_id": "review:checkout/reviewer-001",
            "state": "unverified"
        }])
    );
    let manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(emitted_manifest).expect("projected manifest should be written"),
    )
    .expect("projected manifest should be JSON");
    assert_eq!(manifest["reviews"][0]["verification_state"], "unverified");

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_rejects_explicit_signature_error_without_report_or_manifest() {
    let project = project_dir("invalid-signature");
    fs::write(
        project.join("manifest.json"),
        r#"{
          "schema_version": 1,
          "nodes": [],
          "reviews": [{
            "namespace": "checkout",
            "key": "reviewer-001",
            "provenance_digest": "sha256:review",
            "visibility": "public"
          }],
          "evidence": [],
          "edges": []
        }"#,
    )
    .expect("manifest should be writable");
    let trust_wire = r#"{
              "schema_version": 1,
              "attestations": [{
                "review_id": "review:checkout/reviewer-001",
                "subject_digest": "sha256:graph",
                "source_commit": "commit-1",
                "provenance_digest": "sha256:review",
                "provider": "github",
                "key_id": "org/reviews-2026",
                "algorithm": "ed25519",
                "signature": "AQID",
                "issued_at": "2026-07-29T00:00:00Z",
                "sequence": 1
              }],
              "lifecycle": [],
              "trust_store": [__TRUST_KEY__]
            }"#
    .replace("__TRUST_KEY__", trust_key());
    fs::write(project.join("trust.json"), trust_wire).expect("attestation wire should be writable");
    let emitted_manifest = project.join("invalid-manifest.json");
    let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
    command.current_dir(&project).args([
        "validate",
        "manifest.json",
        "--format",
        "json",
        "--trust-store",
        "trust.json",
        "--emit-manifest",
        "invalid-manifest.json",
    ]);
    let output = command.output().expect("lsharp validate should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "unexpected exit: {stderr}");
    assert!(
        output.stdout.is_empty(),
        "invalid signature must not emit report"
    );
    assert!(
        stderr.contains("signature length"),
        "diagnostic missing: {stderr}"
    );
    assert!(
        !emitted_manifest.exists(),
        "invalid signature must not emit manifest"
    );

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_rejects_review_input_outside_project_root_before_report() {
    let project = project_dir("outside");
    let outside = project
        .parent()
        .expect("temp project should have a parent")
        .join("lsharp-review-input-outside-wire.json");
    fs::write(&outside, review_wire(Some(trust_key()), false)).expect("outside wire should write");

    let output = run_validate(
        &project,
        Some(Path::new("../lsharp-review-input-outside-wire.json")),
        None,
    );
    assert_input_error(output, &["trust store", "project root"]);

    fs::remove_file(outside).ok();
    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_rejects_unknown_review_input_fields_without_report() {
    let project = project_dir("unknown-field");
    let trust_store = project.join("trust.json");
    fs::write(&trust_store, review_wire(Some(trust_key()), true))
        .expect("trust store wire should write");

    let output = run_validate(&project, Some(Path::new("trust.json")), None);
    assert_input_error(output, &["unknown field", "document.unexpected"]);

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_rejects_duplicate_review_input_fields_without_report() {
    let project = project_dir("duplicate-field");
    let trust_store = project.join("trust.json");
    fs::write(
        &trust_store,
        r#"{"schema_version":1,"schema_version":1,"attestations":[],"lifecycle":[],"trust_store":[]}"#,
    )
    .expect("duplicate trust store wire should write");

    let output = run_validate(&project, Some(Path::new("trust.json")), None);
    assert_input_error(output, &["duplicate field", "document.schema_version"]);

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_rejects_duplicate_trust_keys_without_report() {
    let project = project_dir("duplicate-key");
    let trust_store = project.join("trust.json");
    let key = trust_key();
    fs::write(
        &trust_store,
        format!(
            "{{\"schema_version\":1,\"attestations\":[],\"lifecycle\":[],\"trust_store\":[{key},{key}]}}"
        ),
    )
    .expect("duplicate trust key wire should write");

    let output = run_validate(&project, Some(Path::new("trust.json")), None);
    assert_input_error(output, &["duplicate", "review trust store"]);

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_accepts_explicit_project_relative_inputs_without_implicit_defaults() {
    let project = project_dir("explicit");
    let trust_store = project.join("trust.json");
    let lifecycle = project.join("lifecycle.json");
    fs::write(&trust_store, review_wire(Some(trust_key()), false))
        .expect("trust store wire should write");
    fs::write(&lifecycle, review_wire(None, false)).expect("lifecycle wire should write");

    let output = run_validate(
        &project,
        Some(Path::new("trust.json")),
        Some(Path::new("lifecycle.json")),
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "unexpected exit: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("unknown report should be JSON");
    assert_eq!(report["status"], "unknown");

    fs::remove_dir_all(project).ok();
}
