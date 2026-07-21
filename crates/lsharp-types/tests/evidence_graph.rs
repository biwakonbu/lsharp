use lsharp_syntax::span::Span;
use lsharp_types::evidence::{
    Edge, Evidence, EvidenceGraph, EvidenceMethod, EvidenceOutcome, EvidenceSubject,
    ExecutionContext, ExecutionIdentity, GraphError, Independence, InvalidationSubject, Provenance,
    ReviewSubject, SamplingPlan,
};
use lsharp_types::intent::{
    AssumptionId, ChangeId, ClaimId, ContractId, EvidenceId, IntentId, OpenQuestionId, ReviewId,
};

fn execution() -> ExecutionContext {
    ExecutionContext::new(
        ExecutionIdentity::new(
            "native-stage0",
            "aarch64-apple-darwin",
            "source-commit-1",
            "sha256:artifact-1",
        ),
        SamplingPlan::new(
            32,
            81042,
            "type-directed-splitmix64-v1",
            vec![1, 3],
            [("negative", 10), ("positive", 22)],
        ),
    )
}

fn provenance() -> Provenance {
    Provenance::new("lsharp-test", "0.2.0-dev", "2026-07-21T00:00:00Z")
}

#[test]
fn evidence_keeps_method_subject_outcome_and_execution_provenance() {
    let claim = ClaimId::new("checkout", "cancel-rejects-shipped").expect("valid claim id");
    let evidence = Evidence::new(
        EvidenceId::new("checkout", "cancel-case-001").expect("valid evidence id"),
        EvidenceMethod::Case,
        EvidenceSubject::Claim(claim.clone()),
        EvidenceOutcome::Pass,
        execution(),
        provenance(),
        Independence::SameAuthor,
    );

    assert_eq!(evidence.id().as_str(), "evidence:checkout/cancel-case-001");
    assert_eq!(evidence.method(), EvidenceMethod::Case);
    assert_eq!(evidence.subject(), &EvidenceSubject::Claim(claim));
    assert_eq!(evidence.outcome(), EvidenceOutcome::Pass);
    assert_eq!(evidence.execution().cases(), 32);
    assert_eq!(evidence.execution().coverage()["negative"], 10);
    assert_eq!(evidence.provenance().producer(), "lsharp-test");
    assert_eq!(evidence.independence(), Independence::SameAuthor);
}

#[test]
fn typed_edges_preserve_the_allowed_graph_relationships() {
    let intent = IntentId::new("checkout", "safe-cancel").expect("valid intent id");
    let claim = ClaimId::new("checkout", "cancel-rejects-shipped").expect("valid claim id");
    let assumption =
        AssumptionId::new("checkout", "shipment-state-authoritative").expect("valid assumption id");
    let contract = ContractId::new("checkout", "cancel-case").expect("valid contract id");
    let observation = EvidenceId::new("checkout", "production-001").expect("valid evidence id");
    let review = ReviewId::new("checkout", "independent-001").expect("valid review id");
    let change = ChangeId::new("checkout", "api-v2").expect("valid change id");
    let evidence = EvidenceId::new("checkout", "case-001").expect("valid evidence id");

    let edges = [
        Edge::Motivates {
            intent,
            claim: claim.clone(),
        },
        Edge::ConstrainedBy {
            claim: claim.clone(),
            assumption,
        },
        Edge::TestedBy {
            claim: claim.clone(),
            contract,
        },
        Edge::Supports {
            observation,
            claim: claim.clone(),
        },
        Edge::Contradicts {
            observation: EvidenceId::new("checkout", "production-001").unwrap(),
            claim,
        },
        Edge::Evaluates {
            review,
            subject: ReviewSubject::Evidence(evidence.clone()),
        },
        Edge::Invalidates {
            change,
            subject: InvalidationSubject::Evidence(evidence),
        },
    ];

    assert_eq!(edges[0].relation(), "motivates");
    assert_eq!(edges[1].relation(), "constrained-by");
    assert_eq!(edges[2].relation(), "tested-by");
    assert_eq!(edges[3].relation(), "supports");
    assert_eq!(edges[4].relation(), "contradicts");
    assert_eq!(edges[5].relation(), "evaluates");
    assert_eq!(edges[6].relation(), "invalidates");
}

#[test]
fn graph_rejects_duplicate_evidence_ids_but_keeps_edge_order() {
    let id = EvidenceId::new("checkout", "case-001").expect("valid evidence id");
    let first = Evidence::new(
        id.clone(),
        EvidenceMethod::Case,
        EvidenceSubject::Contract(ContractId::new("checkout", "cancel-case").unwrap()),
        EvidenceOutcome::Pass,
        execution(),
        provenance(),
        Independence::SameAuthor,
    );
    let second = Evidence::new(
        id.clone(),
        EvidenceMethod::Review,
        EvidenceSubject::Intent(IntentId::new("checkout", "safe-cancel").unwrap()),
        EvidenceOutcome::Unknown,
        execution(),
        provenance(),
        Independence::IndependentReview,
    );
    let mut graph = EvidenceGraph::default();

    graph.add_evidence(first).expect("first evidence is unique");
    assert!(matches!(
        graph.add_evidence(second),
        Err(GraphError::DuplicateEvidence { id: duplicate }) if duplicate == id
    ));

    graph
        .add_edge(Edge::Evaluates {
            review: ReviewId::new("checkout", "independent-001").unwrap(),
            subject: ReviewSubject::Evidence(id),
        })
        .expect("edge is retained as an ordered append");
    assert_eq!(graph.edges().len(), 1);
}

#[test]
fn graph_rejects_edges_that_reference_missing_evidence() {
    let missing = EvidenceId::new("checkout", "missing").expect("valid evidence id");
    let mut graph = EvidenceGraph::default();

    assert!(matches!(
        graph.add_edge(Edge::Evaluates {
            review: ReviewId::new("checkout", "independent-001").unwrap(),
            subject: ReviewSubject::Evidence(missing.clone()),
        }),
        Err(GraphError::MissingEvidence { id }) if id == missing
    ));
    assert!(graph.edges().is_empty());
}

#[test]
fn unrelated_ast_types_are_not_silently_coerced_into_graph_subjects() {
    let question = OpenQuestionId::new("checkout", "cancel-after-label").expect("valid id");
    let node = lsharp_types::intent::OpenQuestion::new(
        question,
        "Can cancellation happen after a label is printed?",
        Span::new(1, 4),
    )
    .expect("valid question");

    assert_eq!(node.kind().as_str(), "open-question");
}
