use lsharp_types::evidence::{
    Edge, EvidenceValidationError, GraphError, ReviewRecord, ReviewVisibility,
};
use lsharp_types::intent::{ChangeId, ReviewId};
use lsharp_types::validation::IntentGraph;
use lsharp_types::validation_input::{ValidationInputError, parse_intent_graph_json};

fn review(id: &str, digest: &str, visibility: ReviewVisibility) -> ReviewRecord {
    ReviewRecord::new(
        ReviewId::new("checkout", id).expect("review ID should be valid"),
        digest,
        visibility,
    )
}

#[test]
fn review_registry_round_trips_opaque_provenance_and_redaction_policy() {
    let mut graph = IntentGraph::default();
    graph
        .add_review(review(
            "reviewer-001",
            "sha256:review-provenance-001",
            ReviewVisibility::Redacted,
        ))
        .expect("review registry should accept an opaque provenance digest");
    graph
        .add_edge(Edge::Invalidates {
            change: ChangeId::new("checkout", "api-v2").unwrap(),
            subject: lsharp_types::evidence::InvalidationSubject::Review(
                ReviewId::new("checkout", "reviewer-001").unwrap(),
            ),
        })
        .expect("registered review should be referenceable by invalidation");

    let value = graph.to_manifest_json_value();
    assert_eq!(value["reviews"][0]["namespace"], "checkout");
    assert_eq!(value["reviews"][0]["key"], "reviewer-001");
    assert_eq!(
        value["reviews"][0]["provenance_digest"],
        "sha256:review-provenance-001"
    );
    assert_eq!(value["reviews"][0]["visibility"], "redacted");
    assert!(value["reviews"][0].get("author").is_none());
    assert!(value["reviews"][0].get("email").is_none());
    assert!(value["reviews"][0].get("body").is_none());

    let decoded = parse_intent_graph_json(&graph.to_manifest_json_string().unwrap())
        .expect("review registry manifest should round-trip");
    assert_eq!(decoded, graph);
}

#[test]
fn review_registry_rejects_edges_to_unregistered_review() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [],
      "reviews": [
        {
          "namespace": "checkout",
          "key": "reviewer-001",
          "provenance_digest": "sha256:review-provenance-001",
          "visibility": "public"
        }
      ],
      "evidence": [],
      "edges": [
        {
          "relation": "invalidates",
          "change": {"namespace": "checkout", "key": "api-v2"},
          "subject": {"kind": "review", "namespace": "checkout", "key": "missing-review"}
        }
      ]
    }
    "#;

    assert!(matches!(
        parse_intent_graph_json(manifest),
        Err(lsharp_types::validation_input::ValidationInputError::Graph(
            GraphError::MissingReview { id }
        )) if id.as_str() == "review:checkout/missing-review"
    ));
}

#[test]
fn explicit_empty_review_registry_rejects_unregistered_review_edges() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [
        {"kind": "intent", "namespace": "checkout", "key": "safe-cancel", "text": "Users can cancel"}
      ],
      "reviews": [],
      "evidence": [],
      "edges": [
        {
          "relation": "evaluates",
          "review": {"namespace": "checkout", "key": "missing-review"},
          "subject": {"kind": "intent", "namespace": "checkout", "key": "safe-cancel"}
        }
      ]
    }
    "#;

    assert!(matches!(
        parse_intent_graph_json(manifest),
        Err(ValidationInputError::Graph(GraphError::MissingReview { id }))
            if id.as_str() == "review:checkout/missing-review"
    ));
}

#[test]
fn explicit_empty_review_registry_round_trips_as_an_empty_array() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [],
      "reviews": [],
      "evidence": [],
      "edges": []
    }
    "#;

    let graph = parse_intent_graph_json(manifest).expect("explicit empty registry should parse");
    let output = graph.to_manifest_json_value();
    assert_eq!(output["reviews"], serde_json::json!([]));

    let decoded = parse_intent_graph_json(&graph.to_manifest_json_string().unwrap())
        .expect("explicit empty registry output should parse");
    assert_eq!(decoded, graph);
}

#[test]
fn manifest_rejects_null_review_registry_instead_of_treating_it_as_absent() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [],
      "reviews": null,
      "evidence": [],
      "edges": []
    }
    "#;

    assert!(matches!(
        parse_intent_graph_json(manifest),
        Err(ValidationInputError::Json(_))
    ));
}

#[test]
fn review_registry_rejects_empty_provenance_digest() {
    let mut graph = IntentGraph::default();
    assert!(matches!(
        graph.add_review(review("reviewer-001", "  ", ReviewVisibility::Public)),
        Err(GraphError::InvalidReview { .. })
    ));
}

#[test]
fn review_registry_rejects_unicode_whitespace_only_provenance_digest_in_manifest_input() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [],
      "reviews": [
        {
          "namespace": "checkout",
          "key": "reviewer-001",
          "provenance_digest": "sha256:review-provenance-001",
          "visibility": "public"
        }
      ],
      "evidence": [],
      "edges": []
    }
    "#
    .replace("sha256:review-provenance-001", "\u{00A0}");

    assert!(matches!(
        parse_intent_graph_json(&manifest),
        Err(ValidationInputError::Graph(GraphError::InvalidReview {
            source: EvidenceValidationError::EmptyField {
                field: "review_provenance_digest"
            }
        }))
    ));
}

#[test]
fn review_registry_rejects_duplicate_review_ids_in_manifest_input() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [],
      "reviews": [
        {
          "namespace": "checkout",
          "key": "reviewer-001",
          "provenance_digest": "sha256:review-provenance-001",
          "visibility": "public"
        },
        {
          "namespace": "checkout",
          "key": "reviewer-001",
          "provenance_digest": "sha256:review-provenance-002",
          "visibility": "redacted"
        }
      ],
      "evidence": [],
      "edges": []
    }
    "#;

    assert!(matches!(
        parse_intent_graph_json(manifest),
        Err(ValidationInputError::Graph(GraphError::DuplicateReview { id }))
            if id.as_str() == "review:checkout/reviewer-001"
    ));
}

#[test]
fn review_registry_rejects_private_author_fields_in_manifest_input() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [],
      "reviews": [
        {
          "namespace": "checkout",
          "key": "reviewer-001",
          "provenance_digest": "sha256:review-provenance-001",
          "visibility": "redacted",
          "author": "alice@example.com"
        }
      ],
      "evidence": [],
      "edges": []
    }
    "#;

    assert!(matches!(
        parse_intent_graph_json(manifest),
        Err(lsharp_types::validation_input::ValidationInputError::Json(
            _
        ))
    ));
}
