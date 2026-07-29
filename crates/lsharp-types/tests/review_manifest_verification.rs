use lsharp_types::evidence::{ReviewRecord, ReviewVisibility};
use lsharp_types::intent::review_attestation::ReviewVerificationState;
use lsharp_types::validation::IntentGraph;

fn review_with_state(state: ReviewVerificationState) -> ReviewRecord {
    ReviewRecord::new(
        lsharp_types::intent::ReviewId::new("checkout", "reviewer-001").unwrap(),
        "sha256:review-provenance-001",
        ReviewVisibility::Redacted,
    )
    .with_verification_state(state)
    .expect("valid verification state should be accepted")
}

#[test]
fn manifest_projects_review_verification_state_and_round_trips_it() {
    let mut graph = IntentGraph::default();
    graph
        .add_review(review_with_state(ReviewVerificationState::Verified))
        .unwrap();

    let value = graph.to_manifest_json_value();
    assert_eq!(value["reviews"][0]["verification_state"], "verified");

    let decoded = lsharp_types::validation_input::parse_intent_graph_json(
        &graph.to_manifest_json_string().unwrap(),
    )
    .expect("review verification state should round-trip through manifest");
    assert_eq!(decoded, graph);
}

#[test]
fn legacy_review_manifest_omits_optional_verification_state() {
    let mut graph = IntentGraph::default();
    graph
        .add_review(ReviewRecord::new(
            lsharp_types::intent::ReviewId::new("checkout", "reviewer-001").unwrap(),
            "sha256:review-provenance-001",
            ReviewVisibility::Public,
        ))
        .unwrap();

    assert!(
        graph.to_manifest_json_value()["reviews"][0]
            .get("verification_state")
            .is_none()
    );
}

#[test]
fn manifest_rejects_invalid_review_verification_state() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [],
      "reviews": [{
        "namespace": "checkout",
        "key": "reviewer-001",
        "provenance_digest": "sha256:review-provenance-001",
        "visibility": "public",
        "verification_state": "invalid"
      }],
      "evidence": [],
      "edges": []
    }
    "#;

    assert!(matches!(
        lsharp_types::validation_input::parse_intent_graph_json(manifest),
        Err(lsharp_types::validation_input::ValidationInputError::Json(
            _
        ))
    ));
}

#[test]
fn manifest_rejects_null_review_verification_state() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [],
      "reviews": [{
        "namespace": "checkout",
        "key": "reviewer-001",
        "provenance_digest": "sha256:review-provenance-001",
        "visibility": "public",
        "verification_state": null
      }],
      "evidence": [],
      "edges": []
    }
    "#;

    assert!(matches!(
        lsharp_types::validation_input::parse_intent_graph_json(manifest),
        Err(lsharp_types::validation_input::ValidationInputError::Json(
            _
        ))
    ));
}

#[test]
fn review_record_rejects_invalid_verification_state() {
    let result = ReviewRecord::new(
        lsharp_types::intent::ReviewId::new("checkout", "reviewer-001").unwrap(),
        "sha256:review-provenance-001",
        ReviewVisibility::Public,
    )
    .with_verification_state(ReviewVerificationState::Invalid);

    assert!(matches!(
        result,
        Err(lsharp_types::evidence::ReviewVerificationStateError::InvalidState)
    ));
}
