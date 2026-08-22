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
    assert_eq!(value["stale_reviews"], 0);
    assert_eq!(value["stale_evidence"], 0);
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
fn validate_rejects_invalid_subject_kind_without_report_or_manifest_output() {
    let path = manifest_path(
        "invalid-subject-kind",
        r#"
        {
          "schema_version": 1,
          "nodes": [],
          "evidence": [],
          "edges": [
            {
              "relation": "evaluates",
              "review": {"namespace": "checkout", "key": "reviewer-001"},
              "subject": {"kind": "contract", "namespace": "checkout", "key": "cancel-case"}
            }
          ]
        }
        "#,
    );

    let output_dir = project_dir("invalid-subject-kind-output");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    let output_manifest = output_dir.join("intent-graph.json");

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
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&output_dir).ok();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "input errors must not be serialized as a validation report or manifest: {:?}",
        output.stdout
    );
    assert!(!manifest_exists, "input error must not emit a manifest");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("evaluates.subject"),
        "diagnostic missing relation: {stderr}"
    );
    assert!(
        stderr.contains("subject kind"),
        "diagnostic missing kind label: {stderr}"
    );
    assert!(
        stderr.contains("contract"),
        "diagnostic missing kind value: {stderr}"
    );
    assert!(
        stderr.contains("contract:checkout/cancel-case"),
        "diagnostic missing stable ID: {stderr}"
    );
}

#[test]
fn validate_rejects_invalid_invalidates_subject_kind_without_report_or_manifest_output() {
    let path = manifest_path(
        "invalid-invalidates-subject-kind",
        r#"
        {
          "schema_version": 1,
          "nodes": [],
          "evidence": [],
          "edges": [
            {
              "relation": "invalidates",
              "change": {"namespace": "checkout", "key": "api-v2"},
              "subject": {"kind": "claim", "namespace": "checkout", "key": "cancel-rejects-shipped"}
            }
          ]
        }
        "#,
    );
    let output_dir = project_dir("invalid-invalidates-subject-kind-output");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    let output_manifest = output_dir.join("intent-graph.json");

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
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&output_dir).ok();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "input errors must not be serialized as a validation report or manifest: {:?}",
        output.stdout
    );
    assert!(!manifest_exists, "input error must not emit a manifest");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalidates.subject"),
        "diagnostic missing relation: {stderr}"
    );
    assert!(
        stderr.contains("subject kind"),
        "diagnostic missing kind label: {stderr}"
    );
    assert!(
        stderr.contains("claim"),
        "diagnostic missing kind value: {stderr}"
    );
    assert!(
        stderr.contains("claim:checkout/cancel-rejects-shipped"),
        "diagnostic missing stable ID: {stderr}"
    );
}

#[test]
fn validate_rejects_null_review_registry_without_report_or_manifest_output() {
    let path = manifest_path(
        "null-review-registry",
        r#"
        {
          "schema_version": 1,
          "nodes": [],
          "evidence": [],
          "edges": [],
          "reviews": null
        }
        "#,
    );
    let output_dir = project_dir("null-review-registry-output");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    let output_manifest = output_dir.join("intent-graph.json");

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
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&output_dir).ok();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "input errors must not be serialized as a validation report or manifest: {:?}",
        output.stdout
    );
    assert!(!manifest_exists, "input error must not emit a manifest");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("reviews"),
        "diagnostic missing review registry field: {stderr}"
    );
    assert!(
        stderr.contains("null"),
        "diagnostic missing invalid type: {stderr}"
    );
}

#[test]
fn validate_rejects_duplicate_coverage_bucket_without_report_or_manifest_output() {
    let path = manifest_path(
        "duplicate-coverage-bucket",
        r#"
        {
          "schema_version": 1,
          "nodes": [
            {"kind": "claim", "namespace": "checkout", "key": "cancel", "text": "The API rejects shipped orders"}
          ],
          "evidence": [
            {
              "namespace": "checkout",
              "key": "review-001",
              "method": "review",
              "subject": {"kind": "claim", "namespace": "checkout", "key": "cancel"},
              "outcome": "pass",
              "execution": {
                "runner": "validator-test",
                "target": "host",
                "source_commit": "commit-1",
                "artifact_digest": "sha256:artifact",
                "sampling": {
                  "cases": 1,
                  "seed": 0,
                  "generator": "fixture",
                  "shrinks": [],
                  "coverage": {"all": 1, "all": 2}
                }
              },
              "provenance": {
                "producer": "validator-test",
                "tool_version": "0.2",
                "timestamp": "2026-07-23T00:00:00Z"
              },
              "independence": "independent-review"
            }
          ],
          "edges": []
        }
        "#,
    );
    let output_dir = project_dir("duplicate-coverage-bucket-output");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    let output_manifest = output_dir.join("intent-graph.json");

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
    fs::remove_file(&path).ok();
    fs::remove_dir_all(&output_dir).ok();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "input errors must not be serialized as a validation report or manifest: {:?}",
        output.stdout
    );
    assert!(!manifest_exists, "input error must not emit a manifest");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("coverage"),
        "diagnostic missing coverage field: {stderr}"
    );
    assert!(
        stderr.contains("duplicate") || stderr.contains("重複"),
        "diagnostic missing duplicate-key classification: {stderr}"
    );
}

#[test]
fn validate_rejects_manifest_missing_required_field_without_report_stdout() {
    let path = manifest_path(
        "missing-required-field",
        r#"{"schema_version":1,"nodes":[],"evidence":[]}"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args(["validate", path.to_str().unwrap(), "--format", "json"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_file(&path).ok();

    assert_ne!(output.status.code(), Some(0));
    assert!(
        output.stdout.is_empty(),
        "input errors must not be serialized as a validation report: {:?}",
        output.stdout
    );
    assert!(!output.stderr.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("missing field"));
}

#[test]
fn validate_manifest_read_failure_preserves_driver_io_error_boundary() {
    let path = manifest_path("missing-input", "{}");
    fs::remove_file(&path).expect("missing manifest fixture should be removed");
    let output_dir = project_dir("missing-input-output");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    let output_manifest = output_dir.join("intent-graph.json");

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
    fs::remove_dir_all(&output_dir).ok();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "read failure must not emit a report"
    );
    assert!(!manifest_exists, "read failure must not emit a manifest");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[LS5001]"),
        "manifest read failure should preserve driver I/O code: {stderr}"
    );
}

#[test]
fn validate_source_read_failure_preserves_driver_io_error_boundary() {
    let path = source_path("missing-source-input", "(defn main [] true)");
    fs::remove_file(&path).expect("missing source fixture should be removed");
    let output_dir = project_dir("missing-source-input-output");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    let output_manifest = output_dir.join("intent-graph.json");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args([
            "validate",
            "--source",
            path.to_str().unwrap(),
            "--format",
            "json",
            "--emit-manifest",
            output_manifest.to_str().unwrap(),
        ])
        .output()
        .expect("lsharp validate --source should run");
    let manifest_exists = output_manifest.exists();
    fs::remove_dir_all(&output_dir).ok();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "read failure must not emit a report"
    );
    assert!(!manifest_exists, "read failure must not emit a manifest");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[LS5001]"),
        "source read failure should preserve driver I/O code: {stderr}"
    );
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
    assert!(text.contains("stale-reviews: 0"));
    assert!(text.contains("stale-evidence: 0"));
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
fn validate_rejects_project_duplicate_across_source_files() {
    let project = project_dir("project-duplicate-across-source-files");
    let source_dir = project.join("src");
    fs::create_dir_all(&source_dir).expect("project source directory should be writable");
    let first = source_dir.join("first.ls");
    let second = source_dir.join("second.ls");
    fs::write(
        &first,
        r#"(defn first [] :intent "intent:checkout/same" "first declaration" true)
"#,
    )
    .expect("first source should be writable");
    fs::write(
        &second,
        r#"(defn second [] :intent "intent:checkout/same" "second declaration" true)
"#,
    )
    .expect("second source should be writable");
    let output_dir = project.join("out");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    let output_manifest = output_dir.join("intent-graph.json");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(&project)
        .args([
            "validate",
            "--source",
            source_dir.to_str().unwrap(),
            "--format",
            "json",
            "--emit-manifest",
            output_manifest.to_str().unwrap(),
        ])
        .output()
        .expect("project source validation should run");
    let manifest_exists = output_manifest.exists();
    fs::remove_dir_all(&project).ok();

    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "project input errors must not be serialized as a report: {:?}",
        output.stdout
    );
    assert!(
        !manifest_exists,
        "project input error must not emit a manifest"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("source validation error:2"),
        "duplicate-node stable code missing: {stderr}"
    );
    assert!(
        stderr.contains("intent:checkout/same"),
        "duplicate stable ID missing: {stderr}"
    );
    assert!(
        stderr.contains("first.ls"),
        "first source path missing: {stderr}"
    );
    assert!(
        stderr.contains("second.ls"),
        "duplicate source path missing: {stderr}"
    );
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
fn validate_source_forwards_parser_code_and_does_not_emit_manifest() {
    let source = source_path(
        "source-parser-error",
        r#"
        (type Point
          (record (: x Int))
          :claim "claim:geometry/point-typed" true)
        "#,
    );
    let output_dir = project_dir("source-parser-error-output");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    let manifest = output_dir.join("intent-graph.json");

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
        .expect("malformed source validation should run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let manifest_exists = manifest.exists();
    fs::remove_file(&source).ok();
    fs::remove_dir_all(&output_dir).ok();

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        stderr.contains("[LS0101]"),
        "stable parser code missing: {stderr}"
    );
    assert!(!manifest_exists, "parse error 時に manifest を作らないべき");
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
fn validate_source_projects_record_definition_metadata_into_report_and_manifest() {
    let source = source_path(
        "source-record-definition",
        r#"
        (type Point
          (record (: x Int))
          :claim "claim:geometry/point-typed" "The point coordinate is an integer"
          :evidence "evidence:geometry/point-proof"
            :subject "claim:geometry/point-typed"
            :method "case"
            :outcome "pass"
            :runner "source-record-test"
            :target "aarch64-apple-darwin"
            :source-commit "source-record-commit"
            :artifact-digest "sha256:source-record"
            :cases 1
            :seed 0
            :generator "source-record-generator"
            :producer "source-record-producer"
            :tool-version "0.2.0-dev"
            :timestamp "2026-07-26T00:00:00Z"
            :independence "same-author"
          :supports "evidence:geometry/point-proof" "claim:geometry/point-typed")
        "#,
    );
    let output_dir = project_dir("source-record-definition-output");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    let manifest = output_dir.join("intent-graph.json");

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
        .expect("record definition source validation should run");
    let report: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("source report should be JSON");
    let emitted: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).expect("record manifest should be emitted"))
            .expect("record manifest should be JSON");
    fs::remove_file(&source).ok();
    fs::remove_dir_all(&output_dir).ok();

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    assert_eq!(report["status"], "unknown");
    assert_eq!(emitted["schema_version"], 1);
    assert_eq!(emitted["nodes"][0]["kind"], "claim");
    assert_eq!(emitted["nodes"][0]["namespace"], "geometry");
    assert_eq!(emitted["nodes"][0]["key"], "point-typed");
    assert_eq!(emitted["evidence"][0]["namespace"], "geometry");
    assert_eq!(emitted["evidence"][0]["key"], "point-proof");
    assert_eq!(emitted["edges"][0]["relation"], "supports");
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
fn validate_source_and_emitted_manifest_have_same_report_and_exit_code() {
    let source = source_path(
        "source-manifest-report-parity",
        r#"
        (defn cancel []
          :intent "intent:checkout/safe-cancel" "Users can cancel an order"
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :motivates "intent:checkout/safe-cancel" "claim:checkout/cancel-rejects-shipped"
          :tested-by "claim:checkout/cancel-rejects-shipped" "contract:checkout/cancel-case"
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
    let output_dir = project_dir("source-manifest-report-parity-output");
    fs::create_dir_all(&output_dir).expect("manifest output directory should be writable");
    let manifest = output_dir.join("intent-graph.json");

    let source_output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
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
        .expect("source validation should run");
    let source_report: serde_json::Value =
        serde_json::from_slice(&source_output.stdout).expect("source report should be JSON");

    let manifest_output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .args(["validate", manifest.to_str().unwrap(), "--format", "json"])
        .output()
        .expect("manifest validation should run");
    let manifest_report: serde_json::Value =
        serde_json::from_slice(&manifest_output.stdout).expect("manifest report should be JSON");

    fs::remove_file(&source).ok();
    fs::remove_dir_all(&output_dir).ok();

    assert_eq!(source_output.status.code(), Some(2));
    assert_eq!(manifest_output.status.code(), source_output.status.code());
    assert_eq!(manifest_report, source_report);
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
    let manifest_exists = manifest.exists();
    fs::remove_file(&source).ok();
    fs::remove_dir_all(manifest.parent().unwrap()).ok();

    assert!(!output.status.success());
    assert!(stderr.contains("evidence registry"));
    assert!(
        stderr.contains(":supports \"evidence:checkout/missing\" \"claim:checkout/cancel\""),
        "adapter diagnostic should include the source directive: {stderr}"
    );
    assert!(
        !manifest_exists,
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

#[test]
fn validate_rejects_project_config_absolute_manifest_path() {
    let project = project_dir("config-absolute");
    fs::create_dir_all(&project).expect("project should be writable");
    let manifest = project.join("intent-graph.json");
    fs::write(&manifest, include_str!("fixtures/intent-graph-pass.json"))
        .expect("manifest should be writable");
    fs::write(
        project.join("lsharp.toml"),
        format!("[validation]\nmanifest = \"{}\"\n", manifest.display()),
    )
    .expect("project config should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(&project)
        .args(["validate"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_dir_all(&project).ok();

    assert_ne!(output.status.code(), Some(0));
    assert!(
        output.stdout.is_empty(),
        "path error must not emit a report"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("project-relative") || stderr.contains("absolute"),
        "unexpected absolute path diagnostic: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn validate_rejects_project_config_manifest_symlink_outside_root() {
    use std::os::unix::fs::symlink;

    let project = project_dir("config-symlink");
    let outside = project_dir("config-symlink-outside");
    fs::create_dir_all(project.join("docs")).expect("project docs should be writable");
    fs::create_dir_all(&outside).expect("outside directory should be writable");
    let target = outside.join("intent-graph.json");
    let link = project.join("docs/intent-graph.json");
    fs::write(&target, include_str!("fixtures/intent-graph-pass.json"))
        .expect("outside manifest should be writable");
    symlink(&target, &link).expect("manifest symlink should be writable");
    fs::write(
        project.join("lsharp.toml"),
        "[validation]\nmanifest = \"docs/intent-graph.json\"\n",
    )
    .expect("project config should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(&project)
        .args(["validate"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_dir_all(&project).ok();
    fs::remove_dir_all(&outside).ok();

    assert_ne!(output.status.code(), Some(0));
    assert!(
        output.stdout.is_empty(),
        "path error must not emit a report"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("project root") || stderr.contains("root 外"),
        "unexpected symlink diagnostic: {stderr}"
    );
}

#[test]
fn validate_rejects_project_config_empty_manifest_path() {
    let project = project_dir("config-empty");
    fs::create_dir_all(&project).expect("project should be writable");
    fs::write(
        project.join("lsharp.toml"),
        "[validation]\nmanifest = \"\"\n",
    )
    .expect("project config should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(&project)
        .args(["validate"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_dir_all(&project).ok();

    assert_ne!(output.status.code(), Some(0));
    assert!(
        output.stdout.is_empty(),
        "path error must not emit a report"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("空") || stderr.contains("empty"),
        "unexpected empty path diagnostic: {stderr}"
    );
}

#[test]
fn validate_rejects_project_config_missing_manifest_file() {
    let project = project_dir("config-missing-file");
    fs::create_dir_all(&project).expect("project should be writable");
    fs::write(
        project.join("lsharp.toml"),
        "[validation]\nmanifest = \"docs/missing.json\"\n",
    )
    .expect("project config should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(&project)
        .args(["validate"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_dir_all(&project).ok();

    assert_ne!(output.status.code(), Some(0));
    assert!(
        output.stdout.is_empty(),
        "path error must not emit a report"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        (stderr.contains("見つかり") && stderr.contains("ません")) || stderr.contains("not found"),
        "unexpected missing path diagnostic: {stderr}"
    );
}

#[test]
fn validate_rejects_project_config_directory_manifest_target() {
    let project = project_dir("config-directory");
    let target = project.join("docs/manifest-dir");
    fs::create_dir_all(&target).expect("manifest directory should be writable");
    fs::write(
        project.join("lsharp.toml"),
        "[validation]\nmanifest = \"docs/manifest-dir\"\n",
    )
    .expect("project config should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .current_dir(&project)
        .args(["validate"])
        .output()
        .expect("lsharp validate should run");
    fs::remove_dir_all(&project).ok();

    assert_ne!(output.status.code(), Some(0));
    assert!(
        output.stdout.is_empty(),
        "path error must not emit a report"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        (stderr.contains("通常の") && stderr.contains("ファイル"))
            || stderr.contains("regular file"),
        "unexpected directory target diagnostic: {stderr}"
    );
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
