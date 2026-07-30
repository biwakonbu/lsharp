use lsharp_types::validation::{IntentGraph, ReviewEvidenceIdentity};
use lsharp_types::validation_input::parse_intent_graph_json;

#[test]
fn manifest_projects_and_roundtrips_review_evidence_identity_in_stable_order() {
    let identity = ReviewEvidenceIdentity::new(
        "sha256:graph",
        "commit-1",
        "sha256:artifact",
        "2026-08-15T00:00:00Z",
        Some("sha256:trust".to_string()),
        Some("sha256:lifecycle".to_string()),
    )
    .expect("review evidence identity should accept complete values");
    let mut graph = IntentGraph::default();
    graph
        .attach_review_evidence_identity(identity)
        .expect("manifest identity should attach once");

    assert_eq!(
        graph.to_manifest_json_value()["review_evidence_identity"],
        serde_json::json!({
            "subject_digest": "sha256:graph",
            "source_commit": "commit-1",
            "artifact_digest": "sha256:artifact",
            "trust_store_digest": "sha256:trust",
            "lifecycle_digest": "sha256:lifecycle",
            "now": "2026-08-15T00:00:00Z"
        })
    );
    assert_eq!(
        graph
            .to_manifest_json_string()
            .expect("manifest identity should serialize"),
        r#"{"schema_version":1,"nodes":[],"evidence":[],"review_evidence_identity":{"subject_digest":"sha256:graph","source_commit":"commit-1","artifact_digest":"sha256:artifact","trust_store_digest":"sha256:trust","lifecycle_digest":"sha256:lifecycle","now":"2026-08-15T00:00:00Z"},"edges":[]}"#
    );

    let parsed = parse_intent_graph_json(
        &graph
            .to_manifest_json_string()
            .expect("manifest identity should serialize"),
    )
    .expect("manifest identity should roundtrip");
    let parsed_identity = parsed
        .review_evidence_identity()
        .expect("parsed graph should retain manifest identity");
    assert_eq!(parsed_identity.subject_digest(), "sha256:graph");
    assert_eq!(parsed_identity.source_commit(), "commit-1");
    assert_eq!(parsed_identity.artifact_digest(), "sha256:artifact");
    assert_eq!(parsed_identity.trust_store_digest(), Some("sha256:trust"));
    assert_eq!(parsed_identity.lifecycle_digest(), Some("sha256:lifecycle"));
    assert_eq!(
        parsed
            .validate()
            .review_evidence_identity()
            .map(|value| value.source_commit()),
        Some("commit-1")
    );
}

#[test]
fn manifest_identity_conflict_is_rejected_instead_of_overwritten() {
    let first = ReviewEvidenceIdentity::new(
        "sha256:graph",
        "commit-1",
        "sha256:artifact",
        "2026-08-15T00:00:00Z",
        None,
        None,
    )
    .expect("first identity should be valid");
    let second = ReviewEvidenceIdentity::new(
        "sha256:graph",
        "commit-2",
        "sha256:artifact",
        "2026-08-15T00:00:00Z",
        None,
        None,
    )
    .expect("second identity should be valid");
    let mut graph = IntentGraph::default();
    graph
        .attach_review_evidence_identity(first)
        .expect("first identity should attach");
    let error = graph
        .attach_review_evidence_identity(second)
        .expect_err("conflicting identity must fail closed");
    assert!(error.to_string().contains("一致しません"));
}

#[test]
fn manifest_identity_requires_explicit_nullable_fields() {
    let source = serde_json::json!({
        "schema_version": 1,
        "nodes": [],
        "evidence": [],
        "edges": [],
        "review_evidence_identity": {
            "subject_digest": "sha256:graph",
            "source_commit": "commit-1",
            "artifact_digest": "sha256:artifact",
            "trust_store_digest": null,
            "lifecycle_digest": null
        }
    });
    let error = parse_intent_graph_json(&source.to_string())
        .expect_err("identity now must be required even when digest fields are null");
    assert!(error.to_string().contains("now"));
}

#[test]
fn manifest_identity_rejects_malformed_now() {
    let source = serde_json::json!({
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
            "now": "not-a-canonical-timestamp"
        }
    });
    let error = parse_intent_graph_json(&source.to_string())
        .expect_err("manifest identity now must use strict UTC timestamp");
    assert!(error.to_string().contains("timestamp"));
}

#[test]
fn manifest_identity_rejects_explicit_null() {
    let source = serde_json::json!({
        "schema_version": 1,
        "nodes": [],
        "evidence": [],
        "edges": [],
        "review_evidence_identity": null
    });

    let error = parse_intent_graph_json(&source.to_string())
        .expect_err("identity must be an object when present, not explicit null");
    assert!(error.to_string().contains("review_evidence_identity"));
}
