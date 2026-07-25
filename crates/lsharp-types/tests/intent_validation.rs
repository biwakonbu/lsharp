use lsharp_syntax::span::Span;
use lsharp_types::evidence::{
    Edge, Evidence, EvidenceMethod, EvidenceOutcome, EvidenceSubject, ExecutionContext,
    ExecutionIdentity, GraphError, Independence, Provenance, SamplingPlan,
};
use lsharp_types::intent::{
    AssumptionId, Claim, ClaimId, ContractId, EvidenceId, Intent, IntentId, IntentNode,
    OpenQuestion, OpenQuestionId, ReviewId,
};
use lsharp_types::validation::{IntentGraph, TraceGap, ValidationStatus};

fn execution() -> ExecutionContext {
    ExecutionContext::new(
        ExecutionIdentity::new("validator-test", "host", "commit-1", "sha256:artifact"),
        SamplingPlan::new(1, 0, "fixture", Vec::<u64>::new(), [("all", 1)]),
    )
}

fn provenance() -> Provenance {
    Provenance::new("validator-test", "0.2", "2026-07-21T00:00:00Z")
}

fn intent_graph() -> (IntentGraph, IntentId, ClaimId) {
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
    (graph, intent_id, claim_id)
}

fn evidence(
    id: &str,
    method: EvidenceMethod,
    subject: EvidenceSubject,
    outcome: EvidenceOutcome,
    independence: Independence,
) -> Evidence {
    Evidence::new(
        EvidenceId::new("checkout", id).unwrap(),
        method,
        subject,
        outcome,
        execution(),
        provenance(),
        independence,
    )
}

#[test]
fn validation_reports_trace_gaps_open_questions_and_missing_independent_review() {
    let (mut graph, intent_id, claim_id) = intent_graph();
    let question_id = OpenQuestionId::new("checkout", "cancel-after-label").unwrap();
    graph
        .add_node(IntentNode::OpenQuestion(
            OpenQuestion::new(
                question_id,
                "Can cancellation happen after a label is printed?",
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

    let report = graph.validate();

    assert_eq!(report.status(), ValidationStatus::Unknown);
    assert_eq!(report.status().as_str(), "unknown");
    assert_eq!(report.open_questions(), 1);
    assert_eq!(report.independent_reviews(), 0);
    assert_eq!(report.contradicting_observations(), 0);
    assert_eq!(
        report.trace_gaps(),
        &[TraceGap::ClaimWithoutTest { claim: claim_id }]
    );
    assert_eq!(
        report.trace_gaps()[0].code(),
        "trace-gap.claim-without-test"
    );
}

#[test]
fn validation_counts_independent_reviews_and_deduplicates_contradicting_observations() {
    let (mut graph, intent_id, claim_id) = intent_graph();
    let contract = ContractId::new("checkout", "cancel-case").unwrap();
    graph
        .add_edge(Edge::Motivates {
            intent: intent_id,
            claim: claim_id.clone(),
        })
        .unwrap();
    graph
        .add_edge(Edge::TestedBy {
            claim: claim_id.clone(),
            contract,
        })
        .unwrap();

    let observation = evidence(
        "production-001",
        EvidenceMethod::Production,
        EvidenceSubject::Claim(claim_id.clone()),
        EvidenceOutcome::Contradicted,
        Independence::ExternalObservation,
    );
    let observation_id = observation.id().clone();
    graph.add_evidence(observation).unwrap();
    graph
        .add_edge(Edge::Contradicts {
            observation: observation_id.clone(),
            claim: claim_id.clone(),
        })
        .unwrap();
    let review = evidence(
        "review-001",
        EvidenceMethod::Review,
        EvidenceSubject::Claim(claim_id.clone()),
        EvidenceOutcome::Pass,
        Independence::IndependentReview,
    );
    let review_id = review.id().clone();
    graph.add_evidence(review).unwrap();
    graph
        .add_edge(Edge::Evaluates {
            review: ReviewId::new("checkout", "reviewer-001").unwrap(),
            subject: lsharp_types::evidence::ReviewSubject::Evidence(review_id),
        })
        .unwrap();
    graph
        .add_edge(Edge::Contradicts {
            observation: observation_id,
            claim: claim_id,
        })
        .unwrap();

    let report = graph.validate();

    assert_eq!(report.status(), ValidationStatus::Fail);
    assert_eq!(report.status().as_str(), "fail");
    assert!(report.trace_gaps().is_empty());
    assert_eq!(report.independent_reviews(), 1);
    assert_eq!(report.contradicting_observations(), 1);
}

#[test]
fn complete_graph_passes_without_open_questions_or_contradictions() {
    let (mut graph, intent_id, claim_id) = intent_graph();
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
    let review = evidence(
        "review-001",
        EvidenceMethod::Review,
        EvidenceSubject::Claim(claim_id),
        EvidenceOutcome::Pass,
        Independence::IndependentReview,
    );
    let review_id = review.id().clone();
    graph.add_evidence(review).unwrap();
    graph
        .add_edge(Edge::Evaluates {
            review: ReviewId::new("checkout", "reviewer-001").unwrap(),
            subject: lsharp_types::evidence::ReviewSubject::Evidence(review_id),
        })
        .unwrap();

    let report = graph.validate();

    assert_eq!(report.status(), ValidationStatus::Pass);
    assert_eq!(report.status().as_str(), "pass");
    assert!(report.trace_gaps().is_empty());
    assert_eq!(report.open_questions(), 0);
    assert_eq!(report.independent_reviews(), 1);
    assert_eq!(report.contradicting_observations(), 0);
}

#[test]
fn graph_rejects_duplicate_intent_node_ids() {
    let id = IntentId::new("checkout", "safe-cancel").unwrap();
    let mut graph = IntentGraph::default();
    let first = IntentNode::Intent(Intent::new(id.clone(), "first", Span::dummy()).unwrap());
    let second = IntentNode::Intent(Intent::new(id.clone(), "second", Span::dummy()).unwrap());

    graph.add_node(first).unwrap();
    assert!(matches!(
        graph.add_node(second),
        Err(lsharp_types::evidence::GraphError::DuplicateNode { duplicate })
            if duplicate == *id.stable_id()
    ));
}

#[test]
fn graph_rejects_typed_edges_with_missing_node_endpoints() {
    let (mut graph, intent_id, claim_id) = intent_graph();
    let missing_claim = ClaimId::new("checkout", "missing-claim").unwrap();
    assert!(matches!(
        graph.clone().add_edge(Edge::Motivates {
            intent: intent_id.clone(),
            claim: missing_claim.clone(),
        }),
        Err(GraphError::MissingNode { id }) if id == *missing_claim.stable_id()
    ));
    let missing_assumption = AssumptionId::new("checkout", "missing-assumption").unwrap();
    assert!(matches!(
        graph.clone().add_edge(Edge::ConstrainedBy {
            claim: claim_id.clone(),
            assumption: missing_assumption.clone(),
        }),
        Err(GraphError::MissingNode { id }) if id == *missing_assumption.stable_id()
    ));
    assert!(matches!(
        graph.add_edge(Edge::Evaluates {
            review: ReviewId::new("checkout", "reviewer-001").unwrap(),
            subject: lsharp_types::evidence::ReviewSubject::Claim(missing_claim.clone()),
        }),
        Err(GraphError::MissingNode { id }) if id == *missing_claim.stable_id()
    ));
}

#[test]
fn graph_rejects_evidence_edges_with_missing_claim_endpoints() {
    let (mut graph, _intent_id, _claim_id) = intent_graph();
    let observation = evidence(
        "observation-001",
        EvidenceMethod::Case,
        EvidenceSubject::Claim(ClaimId::new("checkout", "cancel-rejects-shipped").unwrap()),
        EvidenceOutcome::Pass,
        Independence::SameAuthor,
    );
    let observation_id = observation.id().clone();
    graph.add_evidence(observation).unwrap();
    let missing_claim = ClaimId::new("checkout", "missing-claim").unwrap();

    assert!(matches!(
        graph.clone().add_edge(Edge::Supports {
            observation: observation_id.clone(),
            claim: missing_claim.clone(),
        }),
        Err(GraphError::MissingNode { id }) if id == *missing_claim.stable_id()
    ));
    assert!(matches!(
        graph.add_edge(Edge::Contradicts {
            observation: observation_id,
            claim: missing_claim.clone(),
        }),
        Err(GraphError::MissingNode { id }) if id == *missing_claim.stable_id()
    ));
}
