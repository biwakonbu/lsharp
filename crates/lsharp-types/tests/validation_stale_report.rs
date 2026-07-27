use lsharp_syntax::span::Span;
use lsharp_types::evidence::{
    Edge, Evidence, EvidenceMethod, EvidenceOutcome, EvidenceSubject, ExecutionContext,
    ExecutionIdentity, Independence, InvalidationSubject, Provenance, ReviewSubject,
    ReviewVisibility, SamplingPlan,
};
use lsharp_types::intent::{
    ChangeId, Claim, ClaimId, ContractId, EvidenceId, Intent, IntentId, IntentNode, ReviewId,
};
use lsharp_types::validation::{IntentGraph, ValidationStatus};

fn complete_graph_with_invalidated_review() -> IntentGraph {
    let intent_id = IntentId::new("checkout", "safe-cancel").unwrap();
    let claim_id = ClaimId::new("checkout", "cancel-rejects-shipped").unwrap();
    let review_id = ReviewId::new("checkout", "reviewer-001").unwrap();
    let evidence_id = EvidenceId::new("checkout", "review-001").unwrap();
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
            claim: claim_id.clone(),
        })
        .unwrap();
    graph
        .add_edge(Edge::TestedBy {
            claim: claim_id.clone(),
            contract: ContractId::new("checkout", "cancel-case").unwrap(),
        })
        .unwrap();
    graph
        .add_review(lsharp_types::evidence::ReviewRecord::new(
            review_id.clone(),
            "sha256:review-001",
            ReviewVisibility::Redacted,
        ))
        .unwrap();
    graph
        .add_evidence(Evidence::new(
            evidence_id.clone(),
            EvidenceMethod::Review,
            EvidenceSubject::Claim(claim_id),
            EvidenceOutcome::Pass,
            ExecutionContext::new(
                ExecutionIdentity::new("validator-test", "host", "commit-1", "sha256:artifact"),
                SamplingPlan::new(1, 0, "fixture", Vec::<u64>::new(), [("all", 1)]),
            ),
            Provenance::new("validator-test", "0.2", "2026-07-27T00:00:00Z"),
            Independence::IndependentReview,
        ))
        .unwrap();
    graph
        .add_edge(Edge::Evaluates {
            review: review_id.clone(),
            subject: ReviewSubject::Evidence(evidence_id),
        })
        .unwrap();
    graph
        .add_edge(Edge::Invalidates {
            change: ChangeId::new("checkout", "api-v2").unwrap(),
            subject: InvalidationSubject::Review(review_id),
        })
        .unwrap();
    graph
}

#[test]
fn stale_review_is_reported_and_prevents_a_pass() {
    let report = complete_graph_with_invalidated_review().validate();

    assert_eq!(report.status(), ValidationStatus::Unknown);
    assert_eq!(report.stale_reviews(), 1);
    assert_eq!(report.stale_evidence(), 1);
    assert_eq!(report.to_json_value()["stale_reviews"], 1);
    assert_eq!(report.to_json_value()["stale_evidence"], 1);
    assert!(report.to_text().contains("stale-reviews: 1\n"));
    assert!(report.to_text().contains("stale-evidence: 1\n"));
}
