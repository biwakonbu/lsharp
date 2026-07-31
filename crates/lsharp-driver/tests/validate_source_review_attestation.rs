use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::{Signer, SigningKey};
use lsharp_types::intent::review_attestation::{AttestationAlgorithm, ReviewAttestation};

fn project_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let project = std::env::temp_dir().join(format!("lsharp-source-review-{name}-{nonce}"));
    fs::create_dir_all(&project).expect("project directory should be writable");
    fs::write(
        project.join("lsharp.toml"),
        "[project]\nname = \"source-review\"\n",
    )
    .expect("project config should be writable");
    project
}

const SOURCE: &str = r#"
(defn review []
  :review "review:checkout/reviewer-001" "sha256:review-001" "redacted"
  :review-attestation
    :review-id "review:checkout/reviewer-001"
    :subject-digest "sha256:subject-001"
    :source-commit "0123456789abcdef"
    :provenance-digest "sha256:review-001"
    :provider "github"
    :key-id "org/reviews-2026"
    :algorithm "ed25519"
    :signature "AAECAw"
    :issued-at "2026-08-01T00:00:00Z"
    :expires-at "2026-09-01T00:00:00Z"
    :sequence 3
  true)
"#;

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

fn write_signed_inputs(project: &PathBuf) {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation = ReviewAttestation::new(
        "review:checkout/reviewer-001",
        "sha256:subject-001",
        "0123456789abcdef",
        "sha256:review-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z",
        Some("2026-09-01T00:00:00Z".to_string()),
        3,
        vec![0; 64],
    )
    .expect("source attestation should be valid");
    let signature = signing_key.sign(&attestation.canonical_bytes());
    let signature = base64url_no_padding(&signature.to_bytes());
    let public_key = base64url_no_padding(&signing_key.verifying_key().to_bytes());
    let json_attestation = format!(
        r#"{{
          "review_id": "review:checkout/reviewer-001",
          "subject_digest": "sha256:subject-001",
          "source_commit": "0123456789abcdef",
          "provenance_digest": "sha256:review-001",
          "provider": "github",
          "key_id": "org/reviews-2026",
          "algorithm": "ed25519",
          "signature": "{signature}",
          "issued_at": "2026-08-01T00:00:00Z",
          "expires_at": "2026-09-01T00:00:00Z",
          "sequence": 3
        }}"#
    );
    fs::write(
        project.join("trust.json"),
        r#"{
              "schema_version": 1,
              "attestations": [__ATTESTATION__],
              "lifecycle": [],
              "trust_store": [{
                "provider": "github",
                "key_id": "org/reviews-2026",
                "algorithm": "ed25519",
                "public_key": "__PUBLIC_KEY__"
              }]
            }"#
        .replace("__ATTESTATION__", &json_attestation)
        .replace("__PUBLIC_KEY__", &public_key),
    )
    .expect("trust input should be writable");
    fs::write(
        project.join("lifecycle.json"),
        r#"{
          "schema_version": 1,
          "attestations": [],
          "lifecycle": [{
            "review_id": "review:checkout/reviewer-001",
            "sequence": 3,
            "state": "active",
            "effective_at": "2026-08-01T00:00:00Z"
          }]
        }"#,
    )
    .expect("lifecycle input should be writable");
}

#[test]
fn validate_source_projects_attestation_as_unverified_fact_and_manifest_state() {
    let project = project_dir("unverified");
    fs::write(project.join("review.ls"), SOURCE).expect("source should be writable");
    let output = project.join("manifest.json");

    let result = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(&project)
        .args([
            "validate",
            "--source",
            "review.ls",
            "--format",
            "json",
            "--emit-manifest",
            "manifest.json",
        ])
        .output()
        .expect("validate source should run");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert_eq!(result.status.code(), Some(2), "unexpected exit: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    let report: serde_json::Value =
        serde_json::from_slice(&result.stdout).expect("report should be JSON");
    assert_eq!(
        report["review_verifications"],
        serde_json::json!([{
            "review_id": "review:checkout/reviewer-001",
            "state": "unverified"
        }])
    );
    let expected_attestation = ReviewAttestation::new(
        "review:checkout/reviewer-001",
        "sha256:subject-001",
        "0123456789abcdef",
        "sha256:review-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z",
        Some("2026-09-01T00:00:00Z".to_string()),
        3,
        vec![0, 1, 2],
    )
    .expect("source attestation projection fixture should be valid");
    let attestation_start = SOURCE
        .find(":review-attestation")
        .expect("attestation directive span should exist");
    let attestation_end = SOURCE
        .rfind("\n  true")
        .expect("attestation directive end should exist");
    assert_eq!(
        report["review_attestations"],
        serde_json::json!([{
            "review_id": "review:checkout/reviewer-001",
            "subject_digest": "sha256:subject-001",
            "source_commit": "0123456789abcdef",
            "provenance_digest": "sha256:review-001",
            "provider": "github",
            "key_id": "org/reviews-2026",
            "algorithm": "ed25519",
            "signature": "AAECAw",
            "issued_at": "2026-08-01T00:00:00Z",
            "expires_at": "2026-09-01T00:00:00Z",
            "sequence": 3,
            "state": "unverified",
            "canonical_bytes": expected_attestation.canonical_bytes(),
            "span": {"start": attestation_start, "end": attestation_end}
        }])
    );
    let report_text = String::from_utf8_lossy(&result.stdout);
    let expected_attestation_fields = [
        "\"review_id\"",
        "\"subject_digest\"",
        "\"source_commit\"",
        "\"provenance_digest\"",
        "\"provider\"",
        "\"key_id\"",
        "\"algorithm\"",
        "\"signature\"",
        "\"issued_at\"",
        "\"expires_at\"",
        "\"sequence\"",
        "\"state\"",
        "\"canonical_bytes\"",
        "\"span\"",
    ];
    let mut previous = 0;
    for field in expected_attestation_fields {
        let relative = report_text[previous..]
            .find(field)
            .expect("review attestation fields should be present in deterministic order");
        previous += relative + field.len();
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&output).expect("source manifest should be emitted"))
            .expect("manifest should be JSON");
    assert_eq!(manifest["reviews"][0]["verification_state"], "unverified");

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_source_external_verification_overrides_source_unverified_fact() {
    let project = project_dir("verified");
    fs::write(project.join("review.ls"), SOURCE).expect("source should be writable");
    write_signed_inputs(&project);

    let output = project.join("manifest.json");
    let result = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(&project)
        .args([
            "validate",
            "--source",
            "review.ls",
            "--format",
            "json",
            "--emit-manifest",
            "manifest.json",
            "--trust-store",
            "trust.json",
            "--review-lifecycle",
            "lifecycle.json",
            "--review-subject-digest",
            "sha256:subject-001",
            "--review-source-commit",
            "0123456789abcdef",
            "--review-now",
            "2026-08-15T00:00:00Z",
        ])
        .output()
        .expect("validate source should run");

    let stderr = String::from_utf8_lossy(&result.stderr);
    assert_eq!(result.status.code(), Some(2), "unexpected exit: {stderr}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
    let report: serde_json::Value =
        serde_json::from_slice(&result.stdout).expect("report should be JSON");
    assert_eq!(
        report["review_verifications"],
        serde_json::json!([{
            "review_id": "review:checkout/reviewer-001",
            "state": "verified"
        }])
    );
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&output).expect("source manifest should be emitted"))
            .expect("manifest should be JSON");
    assert_eq!(manifest["reviews"][0]["verification_state"], "verified");

    fs::remove_dir_all(project).ok();
}

#[test]
fn validate_source_rejects_invalid_attestation_fields_with_stable_error_code() {
    for (name, source) in [
        (
            "algorithm",
            SOURCE.replace(":algorithm \"ed25519\"", ":algorithm \"rsa-sha256\""),
        ),
        (
            "signature",
            SOURCE.replace(":signature \"AAECAw\"", ":signature \"A===\""),
        ),
        (
            "timestamp",
            SOURCE.replace(
                ":issued-at \"2026-08-01T00:00:00Z\"",
                ":issued-at \"2026-02-30T00:00:00Z\"",
            ),
        ),
        (
            "time-window",
            SOURCE.replace(
                ":expires-at \"2026-09-01T00:00:00Z\"",
                ":expires-at \"2026-07-01T00:00:00Z\"",
            ),
        ),
    ] {
        let project = project_dir(&format!("invalid-attestation-code-{name}"));
        fs::write(project.join("review.ls"), source).expect("source should be writable");
        let output = project.join("manifest.json");

        let result = Command::new(env!("CARGO_BIN_EXE_lsharp"))
            .current_dir(&project)
            .args([
                "validate",
                "--source",
                "review.ls",
                "--format",
                "json",
                "--emit-manifest",
                "manifest.json",
            ])
            .output()
            .expect("validate source should run");

        let stderr = String::from_utf8_lossy(&result.stderr);
        assert_eq!(
            result.status.code(),
            Some(1),
            "unexpected exit for {name}: {stderr}"
        );
        assert!(
            result.stdout.is_empty(),
            "invalid source must not emit a report for {name}"
        );
        assert!(
            stderr.contains("source validation error:8"),
            "stable source attestation error code is missing for {name}: {stderr}"
        );
        assert!(
            !output.exists(),
            "invalid source must not emit a manifest for {name}"
        );

        fs::remove_dir_all(project).ok();
    }
}
