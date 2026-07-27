use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_path(name: &str, extension: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("lsharp-validate-{name}-{nonce}.{extension}"))
}

#[test]
fn validate_source_projects_review_and_change_edges_into_manifest() {
    let source = temp_path("review-invalidation", "ls");
    let output_dir = temp_path("review-invalidation-output", "dir");
    let manifest = output_dir.join("intent-graph.json");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    fs::write(
        &source,
        r#"
        (defn review []
          :intent "intent:checkout/safe-cancel" "Users can cancel an order"
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :motivates "intent:checkout/safe-cancel" "claim:checkout/cancel-rejects-shipped"
          :tested-by "claim:checkout/cancel-rejects-shipped" "contract:checkout/cancel-case"
          :evidence "evidence:checkout/review-001"
            :subject "claim:checkout/cancel-rejects-shipped"
            :method "review"
            :outcome "pass"
            :runner "review-tool"
            :target "aarch64-apple-darwin"
            :source-commit "commit-review-1"
            :artifact-digest "sha256:review-1"
            :cases 1
            :seed 42
            :generator "review-fixture"
            :producer "review-tool"
            :tool-version "0.2.0"
            :timestamp "2026-07-27T00:00:00Z"
            :independence "independent-review"
          :review "review:checkout/reviewer-001" "sha256:review-provenance-001" "redacted"
          :evaluates "review:checkout/reviewer-001" "claim:checkout/cancel-rejects-shipped"
          :invalidates "change:checkout/api-v2" "evidence:checkout/review-001"
          true)
        "#,
    )
    .expect("source should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            "--source",
            source.to_str().expect("source path should be UTF-8"),
            "--format",
            "json",
            "--emit-manifest",
            manifest.to_str().expect("manifest path should be UTF-8"),
        ])
        .output()
        .expect("lsharp validate --source should run");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("report should be JSON");
    let manifest_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).expect("manifest should be emitted"))
            .expect("manifest should be JSON");
    fs::remove_file(&source).ok();
    fs::remove_dir_all(&output_dir).ok();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(report["status"], "pass");
    assert_eq!(manifest_value["reviews"].as_array().unwrap().len(), 1);
    assert_eq!(manifest_value["reviews"][0]["namespace"], "checkout");
    assert_eq!(manifest_value["reviews"][0]["key"], "reviewer-001");
    assert_eq!(
        manifest_value["reviews"][0]["provenance_digest"],
        "sha256:review-provenance-001"
    );
    assert_eq!(manifest_value["reviews"][0]["visibility"], "redacted");
    assert_eq!(manifest_value["edges"].as_array().unwrap().len(), 4);
    assert_eq!(manifest_value["edges"][2]["relation"], "evaluates");
    assert_eq!(
        manifest_value["edges"][2]["review"]["namespace"],
        "checkout"
    );
    assert_eq!(manifest_value["edges"][2]["subject"]["kind"], "claim");
    assert_eq!(manifest_value["edges"][3]["relation"], "invalidates");
    assert_eq!(manifest_value["edges"][3]["change"]["key"], "api-v2");
    assert_eq!(manifest_value["edges"][3]["subject"]["kind"], "evidence");
}

#[test]
fn validate_source_rejects_invalid_review_subject_without_manifest() {
    let source = temp_path("review-kind-mismatch", "ls");
    let output_dir = temp_path("review-kind-mismatch-output", "dir");
    let manifest = output_dir.join("intent-graph.json");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    fs::write(
        &source,
        r#"(defn review []
          :evaluates "review:checkout/reviewer-001" "contract:checkout/not-a-review-subject"
          true)"#,
    )
    .expect("source should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            "--source",
            source.to_str().expect("source path should be UTF-8"),
            "--emit-manifest",
            manifest.to_str().expect("manifest path should be UTF-8"),
        ])
        .output()
        .expect("lsharp validate --source should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let manifest_exists = manifest.exists();
    fs::remove_file(&source).ok();
    fs::remove_dir_all(&output_dir).ok();

    assert!(!output.status.success());
    assert!(stderr.contains("subject kind"));
    assert!(!manifest_exists);
}
