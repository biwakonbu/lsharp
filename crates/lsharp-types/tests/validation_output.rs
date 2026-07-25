use lsharp_syntax::span::Span;
use lsharp_types::evidence::{
    Edge, Evidence, EvidenceMethod, EvidenceOutcome, EvidenceSubject, ExecutionContext,
    ExecutionIdentity, Independence, InvalidationSubject, Provenance, ReviewSubject, SamplingPlan,
};
use lsharp_types::intent::{
    Assumption, AssumptionId, ChangeId, Claim, ClaimId, ContractId, EvidenceId, Intent, IntentId,
    IntentNode, OpenQuestion, OpenQuestionId, ReviewId,
};
use lsharp_types::validation::IntentGraph;

fn all_edges_graph() -> IntentGraph {
    let intent_id = IntentId::new("checkout", "safe-cancel").unwrap();
    let claim_id = ClaimId::new("checkout", "cancel-rejects-shipped").unwrap();
    let assumption_id = AssumptionId::new("checkout", "shipment-state-authoritative").unwrap();
    let question_id = OpenQuestionId::new("checkout", "after-label").unwrap();
    let observation_id = EvidenceId::new("checkout", "production-001").unwrap();
    let review_evidence_id = EvidenceId::new("checkout", "review-001").unwrap();

    let mut graph = IntentGraph::default();
    graph
        .add_node(IntentNode::Intent(
            Intent::new(
                intent_id.clone(),
                "Users can cancel before shipment",
                Span::new(1, 4),
            )
            .unwrap(),
        ))
        .unwrap();
    graph
        .add_node(IntentNode::Claim(
            Claim::new(
                claim_id.clone(),
                "The API rejects shipped orders",
                Span::new(5, 9),
            )
            .unwrap(),
        ))
        .unwrap();
    graph
        .add_node(IntentNode::Assumption(
            Assumption::new(
                assumption_id.clone(),
                "Shipment state is authoritative",
                Span::new(10, 14),
            )
            .unwrap(),
        ))
        .unwrap();
    graph
        .add_node(IntentNode::OpenQuestion(
            OpenQuestion::new(
                question_id,
                "Can cancellation happen after a label is printed?",
                Span::new(15, 20),
            )
            .unwrap(),
        ))
        .unwrap();

    let execution = ExecutionContext::new(
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
    );
    let provenance = Provenance::new("lsharp-test", "0.2.0-dev", "2026-07-23T00:00:00Z");
    graph
        .add_evidence(Evidence::new(
            observation_id.clone(),
            EvidenceMethod::Production,
            EvidenceSubject::Claim(claim_id.clone()),
            EvidenceOutcome::Pass,
            execution.clone(),
            provenance.clone(),
            Independence::ExternalObservation,
        ))
        .unwrap();
    graph
        .add_evidence(Evidence::new(
            review_evidence_id.clone(),
            EvidenceMethod::Review,
            EvidenceSubject::Claim(claim_id.clone()),
            EvidenceOutcome::Pass,
            execution,
            provenance,
            Independence::IndependentReview,
        ))
        .unwrap();

    graph
        .add_edge(Edge::Motivates {
            intent: intent_id,
            claim: claim_id.clone(),
        })
        .unwrap();
    graph
        .add_edge(Edge::ConstrainedBy {
            claim: claim_id.clone(),
            assumption: assumption_id,
        })
        .unwrap();
    graph
        .add_edge(Edge::TestedBy {
            claim: claim_id.clone(),
            contract: ContractId::new("checkout", "cancel-case").unwrap(),
        })
        .unwrap();
    graph
        .add_edge(Edge::Supports {
            observation: observation_id.clone(),
            claim: claim_id.clone(),
        })
        .unwrap();
    graph
        .add_edge(Edge::Contradicts {
            observation: observation_id,
            claim: claim_id.clone(),
        })
        .unwrap();
    graph
        .add_edge(Edge::Evaluates {
            review: ReviewId::new("checkout", "reviewer-001").unwrap(),
            subject: ReviewSubject::Evidence(review_evidence_id.clone()),
        })
        .unwrap();
    graph
        .add_edge(Edge::Evaluates {
            review: ReviewId::new("checkout", "reviewer-002").unwrap(),
            subject: ReviewSubject::Claim(claim_id),
        })
        .unwrap();
    graph
        .add_edge(Edge::Invalidates {
            change: ChangeId::new("checkout", "api-v2").unwrap(),
            subject: InvalidationSubject::Evidence(review_evidence_id),
        })
        .unwrap();

    graph
}

#[test]
fn manifest_output_preserves_every_graph_edge_and_validation_facts() {
    let graph = all_edges_graph();
    let value = graph.to_manifest_json_value();

    assert_eq!(
        value["nodes"].as_array().unwrap().len(),
        graph.nodes().len()
    );
    assert_eq!(
        value["evidence"].as_array().unwrap().len(),
        graph.evidence().len()
    );
    assert_eq!(
        value["edges"].as_array().unwrap().len(),
        graph.edges().len()
    );
    assert_eq!(graph.validate().status().as_str(), "fail");
    assert_eq!(graph.validate().contradicting_observations(), 1);
}

#[test]
fn manifest_output_is_deterministic_and_has_only_schema_fields() {
    let graph = all_edges_graph();
    let first = graph
        .to_manifest_json_string()
        .expect("graph is serializable");
    let second = graph
        .to_manifest_json_string()
        .expect("graph is serializable");
    assert_eq!(first, second);

    let value = graph.to_manifest_json_value();
    let object = value.as_object().expect("manifest is an object");
    assert_eq!(
        object
            .get("schema_version")
            .and_then(|value| value.as_u64()),
        Some(1)
    );
    assert!(object.get("nodes").is_some_and(serde_json::Value::is_array));
    assert!(
        object
            .get("evidence")
            .is_some_and(serde_json::Value::is_array)
    );
    assert!(object.get("edges").is_some_and(serde_json::Value::is_array));
    assert!(!object.contains_key("verified"));

    let edge_relations: Vec<_> = object["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| edge["relation"].as_str().unwrap())
        .collect();
    assert_eq!(
        edge_relations,
        vec![
            "motivates",
            "constrained-by",
            "tested-by",
            "supports",
            "contradicts",
            "evaluates",
            "evaluates",
            "invalidates",
        ]
    );
}
