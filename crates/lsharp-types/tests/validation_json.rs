use lsharp_syntax::span::Span;
use lsharp_types::evidence::Edge;
use lsharp_types::intent::{Claim, ClaimId, Intent, IntentId, IntentNode};
use lsharp_types::validation::IntentGraph;

#[test]
fn validation_report_json_is_strict_fact_oriented_and_has_no_verified_flag() {
    let intent_id = IntentId::new("checkout", "safe-cancel").unwrap();
    let claim_id = ClaimId::new("checkout", "cancel-rejects-shipped").unwrap();
    let mut graph = IntentGraph::default();
    graph
        .add_node(IntentNode::Intent(
            Intent::new(
                intent_id.clone(),
                "Users can cancel before shipment",
                Span::dummy(),
            )
            .unwrap(),
        ))
        .unwrap();
    graph
        .add_node(IntentNode::Claim(
            Claim::new(
                claim_id.clone(),
                "API rejects shipped orders",
                Span::dummy(),
            )
            .unwrap(),
        ))
        .unwrap();
    graph
        .add_edge(Edge::Motivates {
            intent: intent_id,
            claim: claim_id.clone(),
        })
        .unwrap();

    let json = graph.validate().to_json_string().unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let object = value.as_object().unwrap();
    let mut keys = object.keys().cloned().collect::<Vec<_>>();
    keys.sort();

    assert_eq!(
        keys,
        vec![
            "contradicting_observations",
            "independent_reviews",
            "open_questions",
            "stale_evidence",
            "stale_reviews",
            "status",
            "trace_gaps",
        ]
    );
    assert_eq!(value["status"], "unknown");
    assert!(value.get("verified").is_none());
    assert_eq!(
        value["trace_gaps"][0],
        serde_json::json!({
            "code": "trace-gap.claim-without-test",
            "subject_id": "claim:checkout/cancel-rejects-shipped"
        })
    );
}
