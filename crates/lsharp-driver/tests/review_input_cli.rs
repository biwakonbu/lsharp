use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::{Signer, SigningKey};
use lsharp_types::intent::review_attestation::{AttestationAlgorithm, ReviewAttestation};

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

fn base64url_no_padding(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        output.push(ALPHABET[(first >> 2) as usize] as char);
        output.push(
            ALPHABET[((first & 0b11) << 4 | chunk.get(1).copied().unwrap_or(0) >> 4) as usize]
                as char,
        );
        if let Some(second) = chunk.get(1) {
            output.push(
                ALPHABET
                    [((second & 0b1111) << 2 | chunk.get(2).copied().unwrap_or(0) >> 6) as usize]
                    as char,
            );
        }
        if let Some(third) = chunk.get(2) {
            output.push(ALPHABET[(third & 0b0011_1111) as usize] as char);
        }
    }
    output
}

fn signed_review_wire() -> (String, String) {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation = ReviewAttestation::new(
        "review:checkout/reviewer-001",
        "sha256:graph",
        "commit-1",
        "sha256:review",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-07-29T00:00:00Z",
        Some("2026-08-01T00:00:00Z".to_string()),
        1,
        vec![0; 64],
    )
    .expect("attestation should be valid");
    let signature = signing_key.sign(&attestation.canonical_bytes());
    let signature = base64url_no_padding(&signature.to_bytes());
    let public_key = base64url_no_padding(&signing_key.verifying_key().to_bytes());
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
            "signature": "__SIGNATURE__",
            "issued_at": "2026-07-29T00:00:00Z",
            "expires_at": "2026-08-01T00:00:00Z",
            "sequence": 1
          }],
          "lifecycle": [],
          "trust_store": [{
            "provider": "github",
            "key_id": "org/reviews-2026",
            "algorithm": "ed25519",
            "public_key": "__PUBLIC_KEY__"
          }]
        }"#
    .replace("__SIGNATURE__", &signature)
    .replace("__PUBLIC_KEY__", &public_key);
    let lifecycle_wire = r#"{
      "schema_version": 1,
      "attestations": [],
      "lifecycle": [{
        "review_id": "review:checkout/reviewer-001",
        "sequence": 1,
        "state": "active",
        "effective_at": "2026-07-29T00:00:00Z"
      }]
    }"#
    .to_string();
    (trust_wire, lifecycle_wire)
}

fn signed_delayed_review_wire() -> (String, String) {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation = ReviewAttestation::new(
        "review:checkout/reviewer-001",
        "sha256:graph",
        "commit-1",
        "sha256:review",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z",
        Some("2026-09-01T00:00:00Z".to_string()),
        2,
        vec![0; 64],
    )
    .expect("delayed attestation should be valid");
    let signature =
        base64url_no_padding(&signing_key.sign(&attestation.canonical_bytes()).to_bytes());
    let public_key = base64url_no_padding(&signing_key.verifying_key().to_bytes());
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
            "signature": "__SIGNATURE__",
            "issued_at": "2026-08-01T00:00:00Z",
            "expires_at": "2026-09-01T00:00:00Z",
            "sequence": 2
          }],
          "lifecycle": [],
          "trust_store": [{
            "provider": "github",
            "key_id": "org/reviews-2026",
            "algorithm": "ed25519",
            "public_key": "__PUBLIC_KEY__"
          }]
        }"#
    .replace("__SIGNATURE__", &signature)
    .replace("__PUBLIC_KEY__", &public_key);
    let lifecycle_wire = r#"{
      "schema_version": 1,
      "attestations": [],
      "lifecycle": [
        {
          "review_id": "review:checkout/reviewer-001",
          "sequence": 1,
          "state": "proposed",
          "effective_at": "2026-08-01T00:00:00Z"
        },
        {
          "review_id": "review:checkout/reviewer-001",
          "sequence": 2,
          "state": "active",
          "effective_at": "2026-08-02T00:00:00Z"
        }
      ]
    }"#
    .to_string();
    (trust_wire, lifecycle_wire)
}

fn lifecycle_wire_with_events(events: &str) -> String {
    format!(
        r#"{{
          "schema_version": 1,
          "attestations": [],
          "lifecycle": [{events}]
        }}"#
    )
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
fn validate_projects_manifest_only_review_as_unverified_for_explicit_context() {
    let project = project_dir("manifest-only-review");
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
    let emitted_manifest = project.join("manifest-only-output.json");
    let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
    command.current_dir(&project).args([
        "validate",
        "manifest.json",
        "--format",
        "json",
        "--review-subject-digest",
        "sha256:graph",
        "--review-source-commit",
        "commit-1",
        "--review-now",
        "2026-07-30T00:00:00Z",
        "--emit-manifest",
        "manifest-only-output.json",
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
fn validate_keeps_manifest_only_review_legacy_shape_without_explicit_input() {
    let project = project_dir("manifest-only-legacy");
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
    let emitted_manifest = project.join("manifest-only-legacy-output.json");
    let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
    command.current_dir(&project).args([
        "validate",
        "manifest.json",
        "--format",
        "json",
        "--emit-manifest",
        "manifest-only-legacy-output.json",
    ]);
    let output = command.output().expect("lsharp validate should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "unexpected exit: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be JSON");
    assert!(
        report.get("review_verifications").is_none(),
        "legacy invocation must not synthesize verification facts"
    );
    let manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(emitted_manifest).expect("projected manifest should be written"),
    )
    .expect("projected manifest should be JSON");
    assert!(
        manifest["reviews"][0].get("verification_state").is_none(),
        "legacy manifest shape must omit verification_state"
    );

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
fn validate_rejects_partial_review_verification_context_before_report() {
    let project = project_dir("partial-review-context");
    let emitted_manifest = project.join("partial-context-manifest.json");
    let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
    command.current_dir(&project).args([
        "validate",
        "manifest.json",
        "--format",
        "json",
        "--review-now",
        "2026-08-15T00:00:00Z",
        "--emit-manifest",
        "partial-context-manifest.json",
    ]);
    let output = command.output().expect("lsharp validate should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "unexpected exit: {stderr}");
    assert!(
        output.stdout.is_empty(),
        "context errors must not emit report"
    );
    assert!(
        stderr.contains("review verification context"),
        "diagnostic missing: {stderr}"
    );
    assert!(
        !emitted_manifest.exists(),
        "context errors must not emit manifest"
    );

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_projects_review_evidence_identity_for_explicit_artifact_context() {
    let project = project_dir("evidence-identity");
    fs::write(
        project.join("trust.json"),
        review_wire(Some(trust_key()), false),
    )
    .expect("trust wire should be writable");

    let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
    command.current_dir(&project).args([
        "validate",
        "manifest.json",
        "--format",
        "json",
        "--trust-store",
        "trust.json",
        "--review-subject-digest",
        "sha256:graph",
        "--review-source-commit",
        "commit-1",
        "--review-artifact-digest",
        "sha256:artifact",
        "--review-now",
        "2026-08-15T00:00:00Z",
    ]);
    let output = command.output().expect("lsharp validate should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "unexpected exit: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be JSON");
    assert_eq!(
        report["review_evidence_identity"]["subject_digest"],
        "sha256:graph"
    );
    assert_eq!(
        report["review_evidence_identity"]["source_commit"],
        "commit-1"
    );
    assert_eq!(
        report["review_evidence_identity"]["artifact_digest"],
        "sha256:artifact"
    );
    assert!(
        report["review_evidence_identity"]["trust_store_digest"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:"))
    );
    assert!(report["review_evidence_identity"]["lifecycle_digest"].is_null());

    let mut text_command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
    text_command.current_dir(&project).args([
        "validate",
        "manifest.json",
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
    ]);
    let text_output = text_command.output().expect("text validate should run");
    let text = String::from_utf8_lossy(&text_output.stdout);
    assert_eq!(text_output.status.code(), Some(2));
    assert!(text.contains(
        "review-evidence-identity: subject=sha256:graph source=commit-1 artifact=sha256:artifact trust-store=- lifecycle=- now=2026-08-15T00:00:00Z"
    ));

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_emits_review_evidence_identity_in_manifest_for_explicit_artifact_context() {
    let project = project_dir("evidence-identity-manifest");
    fs::write(
        project.join("trust.json"),
        review_wire(Some(trust_key()), false),
    )
    .expect("trust wire should be writable");

    let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
    command.current_dir(&project).args([
        "validate",
        "manifest.json",
        "--format",
        "json",
        "--trust-store",
        "trust.json",
        "--review-subject-digest",
        "sha256:graph",
        "--review-source-commit",
        "commit-1",
        "--review-artifact-digest",
        "sha256:artifact",
        "--review-now",
        "2026-08-15T00:00:00Z",
        "--emit-manifest",
        "evidence-identity-manifest.json",
    ]);
    let output = command.output().expect("lsharp validate should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "unexpected exit: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(project.join("evidence-identity-manifest.json"))
            .expect("manifest output should exist"),
    )
    .expect("manifest output should be JSON");
    assert_eq!(
        manifest["review_evidence_identity"]["subject_digest"],
        "sha256:graph"
    );
    assert_eq!(
        manifest["review_evidence_identity"]["source_commit"],
        "commit-1"
    );
    assert_eq!(
        manifest["review_evidence_identity"]["artifact_digest"],
        "sha256:artifact"
    );
    assert!(
        manifest["review_evidence_identity"]["trust_store_digest"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:"))
    );
    assert!(manifest["review_evidence_identity"]["lifecycle_digest"].is_null());

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_rejects_conflicting_manifest_identity_before_outputs() {
    let project = project_dir("conflicting-evidence-identity");
    fs::write(
        project.join("manifest.json"),
        r#"{
          "schema_version": 1,
          "nodes": [],
          "evidence": [],
          "edges": [],
          "review_evidence_identity": {
            "subject_digest": "sha256:graph",
            "source_commit": "commit-1",
            "artifact_digest": "sha256:artifact",
            "trust_store_digest": null,
            "lifecycle_digest": null,
            "now": "2026-08-15T00:00:00Z"
          }
        }"#,
    )
    .expect("manifest should be writable");

    let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
    command.current_dir(&project).args([
        "validate",
        "manifest.json",
        "--format",
        "json",
        "--review-subject-digest",
        "sha256:graph",
        "--review-source-commit",
        "commit-2",
        "--review-artifact-digest",
        "sha256:artifact",
        "--review-now",
        "2026-08-15T00:00:00Z",
        "--emit-manifest",
        "conflicting-output.json",
    ]);
    let output = command.output().expect("lsharp validate should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "unexpected exit: {stderr}");
    assert!(output.stdout.is_empty(), "conflict must not emit a report");
    assert!(stderr.contains("既存 manifest と一致しません"));
    assert!(!project.join("conflicting-output.json").exists());

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_projects_expiry_and_identity_context_to_state() {
    let project = project_dir("expiry-context");
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
    let (trust_wire, lifecycle_wire) = signed_review_wire();
    fs::write(project.join("trust.json"), trust_wire).expect("trust wire should be writable");
    fs::write(project.join("lifecycle.json"), lifecycle_wire)
        .expect("lifecycle wire should be writable");

    for (label, subject_digest, now, expected_state) in [
        (
            "verified",
            "sha256:graph",
            "2026-07-30T00:00:00Z",
            "verified",
        ),
        ("expired", "sha256:graph", "2026-08-01T00:00:00Z", "stale"),
        ("mismatch", "sha256:other", "2026-07-30T00:00:00Z", "stale"),
    ] {
        let emitted_manifest = project.join(format!("{label}-manifest.json"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
        command.current_dir(&project).args([
            "validate",
            "manifest.json",
            "--format",
            "json",
            "--trust-store",
            "trust.json",
            "--review-lifecycle",
            "lifecycle.json",
            "--review-subject-digest",
            subject_digest,
            "--review-source-commit",
            "commit-1",
            "--review-now",
            now,
            "--emit-manifest",
        ]);
        command.arg(emitted_manifest.file_name().expect("manifest file name"));
        let output = command.output().expect("lsharp validate should run");

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "unexpected exit: {stderr}");
        assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
        let report: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("report should be JSON");
        assert_eq!(report["review_verifications"][0]["state"], expected_state);
        let manifest: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(emitted_manifest).expect("projected manifest should be written"),
        )
        .expect("projected manifest should be JSON");
        assert_eq!(manifest["reviews"][0]["verification_state"], expected_state);
    }

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_does_not_apply_future_lifecycle_transition_before_review_now() {
    let project = project_dir("future-lifecycle");
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
    let (trust_wire, lifecycle_wire) = signed_delayed_review_wire();
    fs::write(project.join("trust.json"), trust_wire).expect("trust wire should be writable");
    fs::write(project.join("lifecycle.json"), lifecycle_wire)
        .expect("lifecycle wire should be writable");

    for (now, expected_state) in [
        ("2026-08-01T12:00:00Z", "unverified"),
        ("2026-08-02T00:00:00Z", "verified"),
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
        command.current_dir(&project).args([
            "validate",
            "manifest.json",
            "--format",
            "json",
            "--trust-store",
            "trust.json",
            "--review-lifecycle",
            "lifecycle.json",
            "--review-subject-digest",
            "sha256:graph",
            "--review-source-commit",
            "commit-1",
            "--review-now",
            now,
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
                "state": expected_state
            }]),
            "lifecycle state must be evaluated at now={now}"
        );
    }

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_projects_out_of_order_lifecycle_as_revoked_with_stable_identity() {
    let project = project_dir("lifecycle-ordering");
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
    let (trust_wire, _) = signed_review_wire();
    fs::write(project.join("trust.json"), trust_wire).expect("trust wire should be writable");

    let active = r#"{
            "review_id": "review:checkout/reviewer-001",
            "sequence": 1,
            "state": "active",
            "effective_at": "2026-07-29T00:00:00Z"
          }"#;
    let revoked = r#"{
            "review_id": "review:checkout/reviewer-001",
            "sequence": 2,
            "state": "revoked",
            "effective_at": "2026-07-30T00:00:00Z",
            "reason_digest": "sha256:revocation"
          }"#;
    fs::write(
        project.join("lifecycle-ordered.json"),
        lifecycle_wire_with_events(&format!("{active},{revoked}")),
    )
    .expect("ordered lifecycle wire should be writable");
    fs::write(
        project.join("lifecycle-reversed.json"),
        lifecycle_wire_with_events(&format!("{revoked},{active}")),
    )
    .expect("reversed lifecycle wire should be writable");

    let mut identities = Vec::new();
    for lifecycle in ["lifecycle-ordered.json", "lifecycle-reversed.json"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
        command.current_dir(&project).args([
            "validate",
            "manifest.json",
            "--format",
            "json",
            "--trust-store",
            "trust.json",
            "--review-lifecycle",
            lifecycle,
            "--review-subject-digest",
            "sha256:graph",
            "--review-source-commit",
            "commit-1",
            "--review-artifact-digest",
            "sha256:artifact",
            "--review-now",
            "2026-07-30T00:00:00Z",
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
                "state": "revoked"
            }])
        );
        identities.push(
            report["review_evidence_identity"]["lifecycle_digest"]
                .as_str()
                .expect("lifecycle digest should be projected")
                .to_string(),
        );
    }
    assert_eq!(identities[0], identities[1]);

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_rejects_malformed_review_clock_without_report_or_manifest() {
    let project = project_dir("malformed-review-clock");
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
    let (trust_wire, lifecycle_wire) = signed_review_wire();
    fs::write(project.join("trust.json"), trust_wire).expect("trust wire should be writable");
    fs::write(project.join("lifecycle.json"), lifecycle_wire)
        .expect("lifecycle wire should be writable");
    let emitted_manifest = project.join("malformed-clock-manifest.json");
    let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
    command.current_dir(&project).args([
        "validate",
        "manifest.json",
        "--format",
        "json",
        "--trust-store",
        "trust.json",
        "--review-lifecycle",
        "lifecycle.json",
        "--review-subject-digest",
        "sha256:graph",
        "--review-source-commit",
        "commit-1",
        "--review-now",
        "not-a-canonical-timestamp",
        "--emit-manifest",
        "malformed-clock-manifest.json",
    ]);
    let output = command.output().expect("lsharp validate should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "unexpected exit: {stderr}");
    assert!(
        output.stdout.is_empty(),
        "clock errors must not emit report"
    );
    assert!(
        stderr.contains("明示 clock") || stderr.contains("timestamp"),
        "diagnostic missing: {stderr}"
    );
    assert!(
        !emitted_manifest.exists(),
        "clock errors must not emit manifest"
    );

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_rejects_malformed_review_clock_without_verification_inputs() {
    let project = project_dir("malformed-review-clock-without-inputs");
    let emitted_manifest = project.join("malformed-clock-without-inputs.json");
    let mut command = Command::new(env!("CARGO_BIN_EXE_lsharp"));
    command.current_dir(&project).args([
        "validate",
        "manifest.json",
        "--format",
        "json",
        "--review-subject-digest",
        "sha256:graph",
        "--review-source-commit",
        "commit-1",
        "--review-artifact-digest",
        "sha256:artifact",
        "--review-now",
        "not-a-canonical-timestamp",
        "--emit-manifest",
        "malformed-clock-without-inputs.json",
    ]);
    let output = command.output().expect("lsharp validate should run");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "unexpected exit: {stderr}");
    assert!(
        output.stdout.is_empty(),
        "clock errors must not emit report"
    );
    assert!(
        stderr.contains("明示 clock") || stderr.contains("timestamp"),
        "diagnostic missing: {stderr}"
    );
    assert!(
        !emitted_manifest.exists(),
        "clock errors must not emit manifest"
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
