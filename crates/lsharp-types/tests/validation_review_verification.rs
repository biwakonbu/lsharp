use lsharp_syntax::span::Span;
use lsharp_types::evidence::{
    Edge, Evidence, EvidenceMethod, EvidenceOutcome, EvidenceSubject, ExecutionContext,
    ExecutionIdentity, Independence, Provenance, ReviewSubject, SamplingPlan,
};
use lsharp_types::intent::review_attestation::ReviewVerificationState;
use lsharp_types::intent::{Claim, ClaimId, Intent, IntentId, IntentNode, ReviewId};
use lsharp_types::validation::{
    IntentGraph, ReviewVerificationFact, ReviewVerificationProjectionError, ValidationStatus,
};

fn execution() -> ExecutionContext {
    ExecutionContext::new(
        ExecutionIdentity::new("validator-test", "host", "commit-1", "sha256:artifact"),
        SamplingPlan::new(1, 0, "fixture", Vec::<u64>::new(), [("all", 1)]),
    )
}

fn complete_graph() -> IntentGraph {
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
                "The API rejects shipped orders",
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
    graph
        .add_edge(Edge::TestedBy {
            claim: claim_id.clone(),
            contract: lsharp_types::intent::ContractId::new("checkout", "cancel-case").unwrap(),
        })
        .unwrap();
    let review = Evidence::new(
        lsharp_types::intent::EvidenceId::new("checkout", "review-001").unwrap(),
        EvidenceMethod::Review,
        EvidenceSubject::Claim(claim_id),
        EvidenceOutcome::Pass,
        execution(),
        Provenance::new("validator-test", "0.3", "2026-08-01T00:00:00Z"),
        Independence::IndependentReview,
    );
    let evidence_id = review.id().clone();
    graph.add_evidence(review).unwrap();
    graph
        .add_edge(Edge::Evaluates {
            review: ReviewId::new("checkout", "reviewer-001").unwrap(),
            subject: ReviewSubject::Evidence(evidence_id),
        })
        .unwrap();
    graph
}

fn fact(id: &str, state: ReviewVerificationState) -> ReviewVerificationFact {
    ReviewVerificationFact::new(ReviewId::parse(id).unwrap(), state).unwrap()
}

#[test]
fn verified_review_fact_is_projected_and_satisfies_independent_gate() {
    let graph = complete_graph();
    let report = graph
        .validate_with_review_verifications(&[fact(
            "review:checkout/reviewer-001",
            ReviewVerificationState::Verified,
        )])
        .expect("verified review fact should be accepted");

    assert_eq!(report.status(), ValidationStatus::Pass);
    assert_eq!(report.independent_reviews(), 1);
    assert_eq!(
        report.review_verifications().unwrap()[0].state(),
        ReviewVerificationState::Verified
    );
    assert_eq!(
        report.to_json_value()["review_verifications"],
        serde_json::json!([
            {"review_id": "review:checkout/reviewer-001", "state": "verified"}
        ])
    );
    assert!(
        report
            .to_text()
            .contains("review-verification: review:checkout/reviewer-001=verified")
    );
}

#[test]
fn non_verified_review_fact_is_reported_and_never_satisfies_gate() {
    for state in [
        ReviewVerificationState::Unverified,
        ReviewVerificationState::Stale,
        ReviewVerificationState::Revoked,
    ] {
        let report = complete_graph()
            .validate_with_review_verifications(&[fact("review:checkout/reviewer-001", state)])
            .expect("non-invalid review fact should be accepted");
        assert_eq!(
            report.status(),
            ValidationStatus::Unknown,
            "state={state:?}"
        );
        assert_eq!(report.independent_reviews(), 0, "state={state:?}");
        assert_eq!(report.review_verifications().unwrap()[0].state(), state);
    }
}

#[test]
fn review_facts_are_sorted_and_duplicate_or_invalid_facts_fail_closed() {
    let first = fact("review:z/reviewer-001", ReviewVerificationState::Stale);
    let second = fact("review:a/reviewer-001", ReviewVerificationState::Verified);
    let report = IntentGraph::default()
        .validate_with_review_verifications(&[first, second])
        .expect("distinct review facts should be accepted");
    let ids = report
        .review_verifications()
        .unwrap()
        .iter()
        .map(|entry| entry.review_id().as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["review:a/reviewer-001", "review:z/reviewer-001"]);

    let duplicate = fact("review:a/reviewer-001", ReviewVerificationState::Stale);
    assert!(matches!(
        IntentGraph::default().validate_with_review_verifications(&[duplicate.clone(), duplicate]),
        Err(ReviewVerificationProjectionError::DuplicateReview { .. })
    ));
    assert!(matches!(
        ReviewVerificationFact::new(
            ReviewId::parse("review:a/reviewer-001").unwrap(),
            ReviewVerificationState::Invalid,
        ),
        Err(ReviewVerificationProjectionError::InvalidState { .. })
    ));
}
