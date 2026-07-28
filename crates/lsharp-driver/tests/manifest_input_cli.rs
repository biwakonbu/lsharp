use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn manifest_path(name: &str, body: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("lsharp-manifest-cli-{name}-{nonce}.json"));
    fs::write(&path, body).expect("manifest should be writable");
    path
}

fn output_dir(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("lsharp-manifest-cli-{name}-{nonce}"))
}

fn assert_manifest_input_error(name: &str, body: &str, expected_fragments: &[&str]) {
    let path = manifest_path(name, body);
    let dir = output_dir(name);
    fs::create_dir_all(&dir).expect("manifest output directory should be writable");
    let output_manifest = dir.join("intent-graph.json");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            path.to_str().unwrap(),
            "--format",
            "json",
            "--emit-manifest",
            output_manifest.to_str().unwrap(),
        ])
        .output()
        .expect("lsharp validate should run");
    let manifest_exists = output_manifest.exists();
    let stderr = String::from_utf8_lossy(&output.stderr);
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&dir).ok();

    assert_eq!(output.status.code(), Some(1), "unexpected exit: {stderr}");
    assert!(
        output.stdout.is_empty(),
        "input errors must not serialize a report or manifest: {:?}",
        output.stdout
    );
    assert!(!manifest_exists, "input error must not emit a manifest");
    for fragment in expected_fragments {
        assert!(
            stderr.contains(fragment),
            "diagnostic missing {fragment:?}: {stderr}"
        );
    }
}

fn assert_manifest_unknown_without_explicit_review_registry(name: &str, body: &str) {
    let path = manifest_path(name, body);
    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args(["validate", path.to_str().unwrap(), "--format", "json"])
        .output()
        .expect("lsharp validate should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
        panic!("unknown report should be valid JSON: {err}; stderr={stderr}")
    });
    fs::remove_file(&path).ok();

    assert_eq!(output.status.code(), Some(2), "unexpected exit: {stderr}");
    assert!(
        stderr.is_empty(),
        "unknown report should not write stderr: {stderr}"
    );
    assert_eq!(value["status"], "unknown");
}

#[test]
fn validate_rejects_duplicate_review_identity_without_report_or_manifest_output() {
    assert_manifest_input_error(
        "duplicate-review",
        r#"
        {
          "schema_version": 1,
          "nodes": [],
          "evidence": [],
          "edges": [],
          "reviews": [
            {
              "namespace": "checkout",
              "key": "reviewer-001",
              "provenance_digest": "sha256:first",
              "visibility": "public"
            },
            {
              "namespace": "checkout",
              "key": "reviewer-001",
              "provenance_digest": "sha256:second",
              "visibility": "redacted"
            }
          ]
        }
        "#,
        &["review ID", "重複", "checkout", "reviewer-001"],
    );
}

#[test]
fn validate_rejects_unknown_edge_field_without_report_or_manifest_output() {
    assert_manifest_input_error(
        "unknown-edge-field",
        r#"
        {
          "schema_version": 1,
          "nodes": [
            {"kind": "intent", "namespace": "checkout", "key": "safe-cancel", "text": "Users can cancel"},
            {"kind": "claim", "namespace": "checkout", "key": "cancel", "text": "The API rejects shipped orders"}
          ],
          "evidence": [],
          "edges": [
            {
              "relation": "motivates",
              "intent": {"namespace": "checkout", "key": "safe-cancel"},
              "claim": {"namespace": "checkout", "key": "cancel"},
              "unexpected": true
            }
          ]
        }
        "#,
        &["unexpected", "edge"],
    );
}

#[test]
fn validate_rejects_unregistered_review_edge_with_explicit_empty_registry() {
    assert_manifest_input_error(
        "missing-review",
        r#"
        {
          "schema_version": 1,
          "nodes": [
            {"kind": "intent", "namespace": "checkout", "key": "safe-cancel", "text": "Users can cancel"}
          ],
          "evidence": [],
          "reviews": [],
          "edges": [
            {
              "relation": "evaluates",
              "review": {"namespace": "checkout", "key": "reviewer-001"},
              "subject": {"kind": "intent", "namespace": "checkout", "key": "safe-cancel"}
            }
          ]
        }
        "#,
        &["review", "missing", "checkout", "reviewer-001"],
    );
}

#[test]
fn validate_allows_opaque_review_edge_when_registry_is_omitted() {
    assert_manifest_unknown_without_explicit_review_registry(
        "omitted-review-registry",
        r#"
        {
          "schema_version": 1,
          "nodes": [
            {"kind": "intent", "namespace": "checkout", "key": "safe-cancel", "text": "Users can cancel"}
          ],
          "evidence": [],
          "edges": [
            {
              "relation": "evaluates",
              "review": {"namespace": "checkout", "key": "reviewer-001"},
              "subject": {"kind": "intent", "namespace": "checkout", "key": "safe-cancel"}
            }
          ]
        }
        "#,
    );
}
