use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(name: &str, extension: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("lsharp-review-registry-{name}-{nonce}.{extension}"))
}

fn manifest_with_review(edge: &str) -> String {
    format!(
        r#"{{
          "schema_version": 1,
          "nodes": [],
          "reviews": [
            {{
              "namespace": "checkout",
              "key": "reviewer-001",
              "provenance_digest": "sha256:review-provenance-001",
              "visibility": "redacted"
            }}
          ],
          "evidence": [],
          "edges": {edge}
        }}"#
    )
}

#[test]
fn validate_manifest_round_trips_redacted_review_registry_without_private_fields() {
    let input = temp_path("input", "json");
    let output = temp_path("output", "json");
    fs::write(&input, manifest_with_review("[]")).expect("manifest should be writable");

    let result = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            input.to_str().expect("input path should be UTF-8"),
            "--format",
            "json",
            "--emit-manifest",
            output.to_str().expect("output path should be UTF-8"),
        ])
        .output()
        .expect("validate manifest should run");
    let report: serde_json::Value =
        serde_json::from_slice(&result.stdout).expect("report should be JSON");
    let emitted: serde_json::Value =
        serde_json::from_slice(&fs::read(&output).expect("normalized manifest should exist"))
            .expect("normalized manifest should be JSON");
    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();

    assert_eq!(result.status.code(), Some(2));
    assert_eq!(report["status"], "unknown");
    assert_eq!(emitted["reviews"][0]["namespace"], "checkout");
    assert_eq!(emitted["reviews"][0]["visibility"], "redacted");
    assert_eq!(
        emitted["reviews"][0]["provenance_digest"],
        "sha256:review-provenance-001"
    );
    assert!(emitted["reviews"][0].get("author").is_none());
    assert!(emitted["reviews"][0].get("email").is_none());
    assert!(emitted["reviews"][0].get("body").is_none());
}

#[test]
fn validate_manifest_rejects_unregistered_review_edge_without_emitting_output() {
    let input = temp_path("missing", "json");
    let output = temp_path("missing-output", "json");
    let edge = r#"[
      {
        "relation": "invalidates",
        "change": {"namespace": "checkout", "key": "api-v2"},
        "subject": {"kind": "review", "namespace": "checkout", "key": "missing-review"}
      }
    ]"#;
    fs::write(&input, manifest_with_review(edge)).expect("manifest should be writable");

    let result = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            input.to_str().expect("input path should be UTF-8"),
            "--emit-manifest",
            output.to_str().expect("output path should be UTF-8"),
        ])
        .output()
        .expect("validate manifest should run");
    let stderr = String::from_utf8_lossy(&result.stderr);
    let output_exists = output.exists();
    fs::remove_file(&input).ok();
    fs::remove_file(&output).ok();

    assert!(!result.status.success());
    assert!(stderr.contains("review ID"), "unexpected stderr: {stderr}");
    assert!(
        stderr.contains("missing-review"),
        "unexpected stderr: {stderr}"
    );
    assert!(
        !output_exists,
        "invalid review edge must not emit a manifest"
    );
}
