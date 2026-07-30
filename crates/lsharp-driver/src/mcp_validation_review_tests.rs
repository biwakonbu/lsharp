use ed25519_dalek::{Signer, SigningKey};
use lsharp_types::intent::review_attestation::{AttestationAlgorithm, ReviewAttestation};
use std::path::{Path, PathBuf};

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

const SOURCE_REVIEW_ATTESTATION: &str = r#"
(defn review []
  :review "review:checkout/reviewer-001" "sha256:review" "redacted"
  :review-attestation
    :review-id "review:checkout/reviewer-001"
    :subject-digest "sha256:graph"
    :source-commit "commit-1"
    :provenance-digest "sha256:review"
    :provider "github"
    :key-id "org/reviews-2026"
    :algorithm "ed25519"
    :signature "AAECAw"
    :issued-at "2026-07-29T00:00:00Z"
    :expires-at "2026-08-01T00:00:00Z"
    :sequence 1
  true)
"#;

fn signed_review_fields() -> (String, String) {
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
    (
        base64url_no_padding(&signing_key.sign(&attestation.canonical_bytes()).to_bytes()),
        base64url_no_padding(&signing_key.verifying_key().to_bytes()),
    )
}

fn mcp_review_project(label: &str, signature: &str, public_key: &str) -> (PathBuf, PathBuf) {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let project = std::env::temp_dir().join(format!("lsharp-mcp-review-{label}-{nonce}"));
    std::fs::create_dir_all(&project).expect("project directory should be writable");
    std::fs::write(
        project.join("lsharp.toml"),
        "[project]\nname = \"mcp-review-context\"\n",
    )
    .expect("project config should be writable");
    let manifest = project.join("manifest.json");
    std::fs::write(
        &manifest,
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
    std::fs::write(
        project.join("trust.json"),
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
        .replace("__SIGNATURE__", signature)
        .replace("__PUBLIC_KEY__", public_key),
    )
    .expect("trust wire should be writable");
    std::fs::write(
        project.join("lifecycle.json"),
        r#"{
          "schema_version": 1,
          "attestations": [],
          "lifecycle": [{
            "review_id": "review:checkout/reviewer-001",
            "sequence": 1,
            "state": "active",
            "effective_at": "2026-07-29T00:00:00Z"
          }]
        }"#,
    )
    .expect("lifecycle wire should be writable");
    (project, manifest)
}

fn mcp_review_arguments(
    manifest: &Path,
    subject_digest: &str,
    source_commit: &str,
    now: &str,
) -> serde_json::Value {
    mcp_review_arguments_with_artifact(manifest, subject_digest, source_commit, None, now)
}

fn mcp_review_arguments_with_artifact(
    manifest: &Path,
    subject_digest: &str,
    source_commit: &str,
    artifact_digest: Option<&str>,
    now: &str,
) -> serde_json::Value {
    let mut arguments = json!({
        "manifest_file": manifest.display().to_string(),
        "trust_store": "trust.json",
        "review_lifecycle": "lifecycle.json",
        "review_subject_digest": subject_digest,
        "review_source_commit": source_commit,
        "review_now": now,
        "include_manifest": true
    });
    if let Some(artifact_digest) = artifact_digest {
        arguments["review_artifact_digest"] = json!(artifact_digest);
    }
    arguments
}

#[test]
fn test_validate_tool_projects_source_attestation_as_unverified() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let project = std::env::temp_dir().join(format!("lsharp-mcp-source-attestation-{nonce}"));
    std::fs::create_dir_all(&project).expect("project directory should be writable");
    std::fs::write(
        project.join("lsharp.toml"),
        "[project]\nname = \"mcp-source-attestation\"\n",
    )
    .expect("project config should be writable");
    let source = project.join("review.ls");
    std::fs::write(&source, SOURCE_REVIEW_ATTESTATION).expect("source should be writable");

    let result = call_tool(
        "lsharp_validate",
        &json!({
            "file": source.display().to_string(),
            "include_manifest": true
        }),
    )
    .expect("source attestation should project through MCP");

    assert_eq!(result["status"], "unknown");
    assert_eq!(
        result["review_verifications"],
        json!([{
            "review_id": "review:checkout/reviewer-001",
            "state": "unverified"
        }])
    );
    assert_eq!(
        result["manifest"]["reviews"][0]["verification_state"],
        "unverified"
    );

    std::fs::remove_dir_all(project).ok();
}

#[test]
fn test_validate_tool_external_verification_overrides_source_unverified() {
    let (signature, public_key) = signed_review_fields();
    let (project, _manifest) = mcp_review_project("source-verified", &signature, &public_key);
    let source = project.join("review.ls");
    std::fs::write(&source, SOURCE_REVIEW_ATTESTATION).expect("source should be writable");

    let result = call_tool(
        "lsharp_validate",
        &json!({
            "file": source.display().to_string(),
            "trust_store": "trust.json",
            "review_lifecycle": "lifecycle.json",
            "review_subject_digest": "sha256:graph",
            "review_source_commit": "commit-1",
            "review_now": "2026-07-30T00:00:00Z",
            "include_manifest": true
        }),
    )
    .expect("explicit source verification should project through MCP");

    assert_eq!(result["status"], "unknown");
    assert_eq!(
        result["review_verifications"],
        json!([{
            "review_id": "review:checkout/reviewer-001",
            "state": "verified"
        }])
    );
    assert_eq!(
        result["manifest"]["reviews"][0]["verification_state"],
        "verified"
    );

    std::fs::remove_dir_all(project).ok();
}

#[test]
fn test_validate_tool_rejects_invalid_source_attestation_with_stable_error_code() {
    let (signature, public_key) = signed_review_fields();
    let (project, _manifest) =
        mcp_review_project("source-invalid-attestation", &signature, &public_key);
    let source = project.join("review.ls");
    std::fs::write(
        &source,
        SOURCE_REVIEW_ATTESTATION.replace(":algorithm \"ed25519\"", ":algorithm \"rsa-sha256\""),
    )
    .expect("source should be writable");

    let error = call_tool(
        "lsharp_validate",
        &json!({
            "file": source.display().to_string(),
            "include_manifest": true
        }),
    )
    .expect_err("invalid source attestation must fail closed");
    assert!(
        error.contains("source validation error:8"),
        "stable source attestation error code is missing: {error}"
    );

    std::fs::remove_dir_all(project).ok();
}

#[test]
fn test_validate_tool_projects_valid_attestation_context_to_report_and_manifest() {
    let (signature, public_key) = signed_review_fields();
    let (project, manifest) = mcp_review_project("valid-context", &signature, &public_key);
    let result = call_tool(
        "lsharp_validate",
        &mcp_review_arguments(
            &manifest,
            "sha256:graph",
            "commit-1",
            "2026-07-30T00:00:00Z",
        ),
    )
    .expect("valid MCP review context should project through report and manifest");

    assert_eq!(
        result["review_verifications"],
        json!([{
            "review_id": "review:checkout/reviewer-001",
            "state": "verified"
        }])
    );
    assert_eq!(
        result["manifest"]["reviews"][0]["verification_state"],
        "verified"
    );

    std::fs::remove_dir_all(project).ok();
}

#[test]
fn test_validate_tool_projects_manifest_only_review_as_unverified() {
    let (signature, public_key) = signed_review_fields();
    let (project, manifest) = mcp_review_project("manifest-only-review", &signature, &public_key);
    std::fs::write(
        project.join("trust.json"),
        r#"{
          "schema_version": 1,
          "attestations": [],
          "lifecycle": [],
          "trust_store": []
        }"#,
    )
    .expect("empty trust wire should be writable");
    std::fs::write(
        project.join("lifecycle.json"),
        r#"{
          "schema_version": 1,
          "attestations": [],
          "lifecycle": []
        }"#,
    )
    .expect("empty lifecycle wire should be writable");

    let result = call_tool(
        "lsharp_validate",
        &mcp_review_arguments(
            &manifest,
            "sha256:graph",
            "commit-1",
            "2026-07-30T00:00:00Z",
        ),
    )
    .expect("manifest-only review should remain explicitly unverified");

    assert_eq!(
        result["review_verifications"],
        json!([{
            "review_id": "review:checkout/reviewer-001",
            "state": "unverified"
        }])
    );
    assert_eq!(
        result["manifest"]["reviews"][0]["verification_state"],
        "unverified"
    );

    std::fs::remove_dir_all(project).ok();
}

#[test]
fn test_validate_tool_projects_review_evidence_identity_with_explicit_artifact() {
    let (signature, public_key) = signed_review_fields();
    let (project, manifest) = mcp_review_project("evidence-identity", &signature, &public_key);
    let result = call_tool(
        "lsharp_validate",
        &mcp_review_arguments_with_artifact(
            &manifest,
            "sha256:graph",
            "commit-1",
            Some("sha256:artifact"),
            "2026-07-30T00:00:00Z",
        ),
    )
    .expect("MCP evidence identity should project through report");

    assert_eq!(
        result["review_evidence_identity"]["subject_digest"],
        "sha256:graph"
    );
    assert_eq!(
        result["review_evidence_identity"]["source_commit"],
        "commit-1"
    );
    assert_eq!(
        result["review_evidence_identity"]["artifact_digest"],
        "sha256:artifact"
    );
    assert!(
        result["review_evidence_identity"]["trust_store_digest"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:"))
    );
    assert!(
        result["review_evidence_identity"]["lifecycle_digest"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:"))
    );
    assert_eq!(
        result["manifest"]["review_evidence_identity"]["artifact_digest"],
        "sha256:artifact"
    );

    std::fs::remove_dir_all(project).ok();
}

#[test]
fn test_validate_tool_projects_expiry_and_binding_mismatches_as_stale() {
    for (label, subject_digest, source_commit, now) in [
        (
            "expired",
            "sha256:graph",
            "commit-1",
            "2026-08-01T00:00:00Z",
        ),
        (
            "subject-mismatch",
            "sha256:other",
            "commit-1",
            "2026-07-30T00:00:00Z",
        ),
        (
            "source-mismatch",
            "sha256:graph",
            "commit-other",
            "2026-07-30T00:00:00Z",
        ),
    ] {
        let (signature, public_key) = signed_review_fields();
        let (project, manifest) = mcp_review_project(label, &signature, &public_key);
        let result = call_tool(
            "lsharp_validate",
            &mcp_review_arguments(&manifest, subject_digest, source_commit, now),
        )
        .expect("stale MCP review context should still project report and manifest");

        assert_eq!(result["review_verifications"][0]["state"], "stale");
        assert_eq!(
            result["manifest"]["reviews"][0]["verification_state"],
            "stale"
        );

        std::fs::remove_dir_all(project).ok();
    }
}

#[test]
fn test_validate_tool_rejects_malformed_review_clock_without_report_or_manifest() {
    let (signature, public_key) = signed_review_fields();
    let (project, manifest) = mcp_review_project("malformed-clock", &signature, &public_key);
    let error = call_tool(
        "lsharp_validate",
        &mcp_review_arguments(
            &manifest,
            "sha256:graph",
            "commit-1",
            "not-a-canonical-timestamp",
        ),
    )
    .expect_err("malformed MCP review clock should be rejected");

    assert!(
        error.contains("明示 clock") || error.contains("timestamp"),
        "unexpected error: {error}"
    );

    std::fs::remove_dir_all(project).ok();
}

#[test]
fn test_validate_tool_rejects_malformed_review_clock_without_verification_inputs() {
    let (signature, public_key) = signed_review_fields();
    let (project, manifest) =
        mcp_review_project("malformed-clock-without-inputs", &signature, &public_key);
    let error = call_tool(
        "lsharp_validate",
        &json!({
            "manifest_file": manifest.display().to_string(),
            "review_subject_digest": "sha256:graph",
            "review_source_commit": "commit-1",
            "review_artifact_digest": "sha256:artifact",
            "review_now": "not-a-canonical-timestamp",
            "include_manifest": true
        }),
    )
    .expect_err("malformed MCP review clock must fail before report projection");

    assert!(
        error.contains("明示 clock") || error.contains("timestamp"),
        "unexpected error: {error}"
    );

    std::fs::remove_dir_all(project).ok();
}
