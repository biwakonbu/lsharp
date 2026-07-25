use lsharp_syntax::span::Span;
use lsharp_types::evidence::Edge;
use lsharp_types::intent::{Claim, ClaimId, Intent, IntentId, IntentNode};
use lsharp_types::validation::IntentGraph;

#[test]
fn validation_report_text_matches_json_facts_without_verified_shortcut() {
    let intent_id = IntentId::new("checkout", "safe-cancel").unwrap();
    let claim_id = ClaimId::new("checkout", "cancel-rejects-shipped").unwrap();
    let mut graph = IntentGraph::default();
    graph
        .add_node(IntentNode::Intent(
            Intent::new(intent_id.clone(), "Users can cancel", Span::dummy()).unwrap(),
        ))
        .unwrap();
    graph
        .add_node(IntentNode::Claim(
            Claim::new(claim_id.clone(), "API rejects shipped", Span::dummy()).unwrap(),
        ))
        .unwrap();
    graph
        .add_edge(Edge::Motivates {
            intent: intent_id,
            claim: claim_id,
        })
        .unwrap();

    let text = graph.validate().to_text();

    assert_eq!(
        text,
        "status: unknown\n\
trace-gap.claim-without-test: claim:checkout/cancel-rejects-shipped\n\
open-questions: 0\n\
independent-reviews: 0\n\
contradicting-observations: 0\n"
    );
    assert!(!text.contains("verified"));
}
