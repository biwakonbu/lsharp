//! selfhost review evidence identity の manifest projection tests。

use super::harness::{run_evidence_registry_runtime, run_manifest_input_runtime};
use serde_json::Value;

/// EC-M3-05 RED: caller が渡した review evidence identity は、Rust manifest wire と同じ
/// field 順・nullable 表現で投影され、同一 identity の再 attach だけを許可する。
#[test]
fn test_e2e_selfhost_evidence_registry_projects_review_identity_and_rejects_conflict() {
    let harness = r#"
(defn main []
  (let [identity-result (source-review-evidence-identity-result
                          "sha256:graph"
                          "commit-1"
                          "sha256:artifact"
                          "sha256:trust"
                          ""
                          "2026-08-15T00:00:00Z")
        identity (source-result-value identity-result)
        graph (source-evidence-graph-with-reviews-and-attestations
                (vector-new 0)
                (vector-new 0)
                (vector-new 0)
                (vector-new 0)
                (vector-new 0))
        attached (source-evidence-graph-attach-review-identity graph identity)
        same (source-evidence-graph-attach-review-identity
              (source-result-value attached)
              identity)
        conflict-result (source-review-evidence-identity-result
                          "sha256:graph"
                          "commit-2"
                          "sha256:artifact"
                          "sha256:trust"
                          ""
                          "2026-08-15T00:00:00Z")
        conflict (source-evidence-graph-attach-review-identity
                  (source-result-value same)
                  (source-result-value conflict-result))
        invalid (source-review-evidence-identity-result
                  "sha256:graph"
                  "commit-1"
                  "sha256:artifact"
                  ""
                  ""
                  "not-a-timestamp")
        invalid-attach (source-evidence-graph-attach-review-identity
                        (source-result-value attached)
                        (source-result-error invalid))]
    (do
      (print (source-result-status identity-result))
      (print (source-result-status attached))
      (print (source-result-status same))
      (print (source-result-status conflict))
      (print (source-evidence-error-code (source-result-error conflict)))
      (print (source-result-status invalid))
      (print (source-evidence-error-code (source-result-error invalid)))
      (print (source-result-status invalid-attach))
      (print (source-evidence-error-code (source-result-error invalid-attach)))
      (print-string (validation-source-manifest-json (source-result-value attached)))
      (print-string "\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let mut lines = output.trim().lines();
    assert_eq!(lines.next(), Some("1"));
    assert_eq!(lines.next(), Some("1"));
    assert_eq!(lines.next(), Some("1"));
    assert_eq!(lines.next(), Some("0"));
    assert_eq!(lines.next(), Some("14"));
    assert_eq!(lines.next(), Some("0"));
    assert_eq!(lines.next(), Some("14"));
    assert_eq!(lines.next(), Some("0"));
    assert_eq!(lines.next(), Some("14"));
    let raw_manifest = lines.next().expect("manifest JSON が必要");
    assert_eq!(
        raw_manifest,
        r#"{"schema_version":1,"nodes":[],"evidence":[],"review_evidence_identity":{"subject_digest":"sha256:graph","source_commit":"commit-1","artifact_digest":"sha256:artifact","trust_store_digest":"sha256:trust","lifecycle_digest":null,"now":"2026-08-15T00:00:00Z"},"edges":[]}"#
    );
    let manifest: Value =
        serde_json::from_str(raw_manifest).expect("manifest JSON は parse 可能であるべき");
    assert_eq!(
        manifest["review_evidence_identity"],
        serde_json::json!({
            "subject_digest": "sha256:graph",
            "source_commit": "commit-1",
            "artifact_digest": "sha256:artifact",
            "trust_store_digest": "sha256:trust",
            "lifecycle_digest": null,
            "now": "2026-08-15T00:00:00Z"
        })
    );
}

#[test]
fn test_e2e_selfhost_evidence_registry_projects_non_null_identity_in_rust_manifest_order() {
    let harness = r#"
(defn main []
  (let [identity-result (source-review-evidence-identity-result
                          "sha256:graph"
                          "commit-1"
                          "sha256:artifact"
                          "sha256:trust"
                          "sha256:lifecycle"
                          "2026-08-15T00:00:00Z")
        identity (source-result-value identity-result)
        graph (source-evidence-graph-with-reviews-and-attestations
                (vector-new 0)
                (vector-new 0)
                (vector-new 0)
                (vector-new 0)
                (vector-new 0))
        attached (source-evidence-graph-attach-review-identity graph identity)
        same (source-evidence-graph-attach-review-identity
              (source-result-value attached)
              identity)]
    (do
      (print (source-result-status identity-result))
      (print (source-result-status attached))
      (print (source-result-status same))
      (print-string (validation-source-manifest-json (source-result-value same)))
      (print-string "\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let mut lines = output.trim().lines();
    assert_eq!(lines.next(), Some("1"));
    assert_eq!(lines.next(), Some("1"));
    assert_eq!(lines.next(), Some("1"));
    let raw_manifest = lines.next().expect("manifest JSON が必要");
    assert_eq!(
        raw_manifest,
        r#"{"schema_version":1,"nodes":[],"evidence":[],"review_evidence_identity":{"subject_digest":"sha256:graph","source_commit":"commit-1","artifact_digest":"sha256:artifact","trust_store_digest":"sha256:trust","lifecycle_digest":"sha256:lifecycle","now":"2026-08-15T00:00:00Z"},"edges":[]}"#
    );
    let manifest: Value =
        serde_json::from_str(raw_manifest).expect("manifest JSON は parse 可能であるべき");
    assert_eq!(
        manifest["review_evidence_identity"]["lifecycle_digest"],
        "sha256:lifecycle"
    );
    assert!(lines.next().is_none(), "unexpected extra selfhost output");
}

#[test]
fn test_e2e_selfhost_manifest_input_retrieves_existing_review_identity() {
    let harness = r#"
(defn main []
  (let [parsed (validation-manifest-review-identity-result
      "{\"schema_version\":1,\"nodes\":[],\"evidence\":[],\"review_evidence_identity\":{\"subject_digest\":\"sha256:graph\",\"source_commit\":\"commit-1\",\"artifact_digest\":\"sha256:artifact\",\"trust_store_digest\":null,\"lifecycle_digest\":null,\"now\":\"2026-08-15T00:00:00Z\"},\"edges\":[]}")
    identity (source-result-value parsed)]
    (do
      (print (source-result-status parsed))
      (print (vector-length identity))
      (print-string (source-review-evidence-identity-json identity))
      (print-string "\n")
      0)))
"#;

    let output = run_manifest_input_runtime(harness);
    let mut lines = output.trim().lines();
    assert_eq!(lines.next(), Some("1"));
    assert_eq!(lines.next(), Some("6"));
    assert_eq!(
        lines.next(),
        Some(
            r#"{"subject_digest":"sha256:graph","source_commit":"commit-1","artifact_digest":"sha256:artifact","trust_store_digest":null,"lifecycle_digest":null,"now":"2026-08-15T00:00:00Z"}"#
        )
    );
    assert!(lines.next().is_none(), "unexpected extra selfhost output");
}
