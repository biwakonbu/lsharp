use lsharp_syntax::span::Span;
use lsharp_types::evidence::{
    Edge, Evidence, EvidenceMethod, EvidenceOutcome, EvidenceSubject, ExecutionContext,
    ExecutionIdentity, Independence, InvalidationSubject, Provenance, ReviewSubject, SamplingPlan,
};
use lsharp_types::intent::{ChangeId, Claim, ClaimId, EvidenceId, IntentNode, ReviewId};
use lsharp_types::validation::IntentGraph;

fn execution() -> ExecutionContext {
    ExecutionContext::new(
        ExecutionIdentity::new("stale-test", "aarch64-apple-darwin", "commit-1", "sha256:1"),
        SamplingPlan::new(1, 0, "fixture", Vec::<u64>::new(), [("all", 1)]),
    )
}

fn evidence(id: &str, outcome: EvidenceOutcome) -> Evidence {
    Evidence::new(
        EvidenceId::new("checkout", id).expect("evidence ID should be valid"),
        EvidenceMethod::Review,
        EvidenceSubject::Claim(ClaimId::new("checkout", "cancel").unwrap()),
        outcome,
        execution(),
        Provenance::new("stale-test", "0.2", "2026-07-27T00:00:00Z"),
        Independence::IndependentReview,
    )
}

fn review(id: &str) -> lsharp_types::evidence::ReviewRecord {
    lsharp_types::evidence::ReviewRecord::new(
        ReviewId::new("checkout", id).expect("review ID should be valid"),
        format!("sha256:{id}"),
        lsharp_types::evidence::ReviewVisibility::Redacted,
    )
}

#[test]
fn invalidated_review_propagates_stale_to_evidence_it_evaluates() {
    let review_id = ReviewId::new("checkout", "reviewer-001").unwrap();
    let first = EvidenceId::new("checkout", "review-001").unwrap();
    let second = EvidenceId::new("checkout", "review-002").unwrap();
    let mut graph = IntentGraph::default();
    graph.add_review(review("reviewer-001")).unwrap();
    graph
        .add_evidence(evidence("review-001", EvidenceOutcome::Pass))
        .unwrap();
    graph
        .add_evidence(evidence("review-002", EvidenceOutcome::Pass))
        .unwrap();
    graph
        .add_edge(Edge::Evaluates {
            review: review_id.clone(),
            subject: ReviewSubject::Evidence(first.clone()),
        })
        .unwrap();
    graph
        .add_edge(Edge::Evaluates {
            review: review_id.clone(),
            subject: ReviewSubject::Evidence(second.clone()),
        })
        .unwrap();
    graph
        .add_edge(Edge::Invalidates {
            change: ChangeId::new("checkout", "api-v2").unwrap(),
            subject: InvalidationSubject::Review(review_id.clone()),
        })
        .unwrap();

    let stale = graph.stale_subjects();

    assert_eq!(stale.reviews(), &[review_id]);
    assert_eq!(stale.evidence(), &[first, second]);
}

#[test]
fn direct_and_declared_stale_evidence_are_deduplicated_in_edge_order() {
    let first = EvidenceId::new("checkout", "case-001").unwrap();
    let second = EvidenceId::new("checkout", "case-002").unwrap();
    let mut graph = IntentGraph::default();
    graph
        .add_evidence(evidence("case-001", EvidenceOutcome::Stale))
        .unwrap();
    graph
        .add_evidence(evidence("case-002", EvidenceOutcome::Pass))
        .unwrap();
    graph
        .add_edge(Edge::Invalidates {
            change: ChangeId::new("checkout", "api-v2").unwrap(),
            subject: InvalidationSubject::Evidence(first.clone()),
        })
        .unwrap();
    graph
        .add_edge(Edge::Invalidates {
            change: ChangeId::new("checkout", "api-v3").unwrap(),
            subject: InvalidationSubject::Evidence(second.clone()),
        })
        .unwrap();
    graph
        .add_edge(Edge::Invalidates {
            change: ChangeId::new("checkout", "api-v4").unwrap(),
            subject: InvalidationSubject::Evidence(first.clone()),
        })
        .unwrap();

    let stale = graph.stale_subjects();

    assert!(stale.reviews().is_empty());
    assert_eq!(stale.evidence(), &[first, second]);
}

#[test]
fn invalidated_review_does_not_stale_node_subjects() {
    let review_id = ReviewId::new("checkout", "reviewer-001").unwrap();
    let claim_id = ClaimId::new("checkout", "cancel").unwrap();
    let mut graph = IntentGraph::default();
    graph.add_review(review("reviewer-001")).unwrap();
    graph
        .add_node(IntentNode::Claim(
            Claim::new(claim_id.clone(), "cancel checkout", Span::dummy()).unwrap(),
        ))
        .unwrap();
    graph
        .add_edge(Edge::Evaluates {
            review: review_id.clone(),
            subject: ReviewSubject::Claim(claim_id),
        })
        .unwrap();
    graph
        .add_edge(Edge::Invalidates {
            change: ChangeId::new("checkout", "api-v2").unwrap(),
            subject: InvalidationSubject::Review(review_id),
        })
        .unwrap();

    let stale = graph.stale_subjects();

    assert!(stale.evidence().is_empty());
}
