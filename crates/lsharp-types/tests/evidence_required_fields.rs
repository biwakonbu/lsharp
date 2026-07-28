use lsharp_types::evidence::{
    Evidence, EvidenceGraph, EvidenceMethod, EvidenceOutcome, EvidenceSubject,
    EvidenceValidationError, ExecutionContext, ExecutionIdentity, GraphError, Independence,
    Provenance, SamplingPlan,
};
use lsharp_types::intent::{ClaimId, EvidenceId};

fn valid_execution() -> ExecutionContext {
    ExecutionContext::new(
        ExecutionIdentity::new(
            "native-stage0",
            "aarch64-apple-darwin",
            "commit-1",
            "sha256:1",
        ),
        SamplingPlan::new(1, 42, "fixed-v1", Vec::new(), [("all", 1)]),
    )
}

fn valid_provenance() -> Provenance {
    Provenance::new("lsharp-test", "0.2.0-dev", "2026-07-23T00:00:00Z")
}

fn valid_evidence() -> Evidence {
    Evidence::new(
        EvidenceId::new("checkout", "case-001").expect("valid evidence id"),
        EvidenceMethod::Case,
        EvidenceSubject::Claim(
            ClaimId::new("checkout", "cancel-rejects-shipped").expect("valid claim id"),
        ),
        EvidenceOutcome::Pass,
        valid_execution(),
        valid_provenance(),
        Independence::SameAuthor,
    )
}

#[test]
fn execution_identity_rejects_each_empty_required_field() {
    let cases = [
        (
            "runner",
            ExecutionIdentity::new("  ", "target", "commit", "digest"),
        ),
        (
            "target",
            ExecutionIdentity::new("runner", "\t", "commit", "digest"),
        ),
        (
            "source_commit",
            ExecutionIdentity::new("runner", "target", "", "digest"),
        ),
        (
            "artifact_digest",
            ExecutionIdentity::new("runner", "target", "commit", "   "),
        ),
    ];

    for (expected_field, identity) in cases {
        assert!(matches!(
            identity.validate_required_fields(),
            Err(EvidenceValidationError::EmptyField { field }) if field == expected_field
        ));
    }
}

#[test]
fn sampling_and_provenance_reject_empty_required_fields() {
    let sampling = SamplingPlan::new(1, 42, "  ", Vec::new(), [("all", 1)]);
    assert!(matches!(
        sampling.validate_required_fields(),
        Err(EvidenceValidationError::EmptyField { field }) if field == "generator"
    ));

    let provenance_cases = [
        ("producer", Provenance::new("", "tool", "timestamp")),
        (
            "tool_version",
            Provenance::new("producer", "  ", "timestamp"),
        ),
        ("timestamp", Provenance::new("producer", "tool", "\n")),
    ];

    for (expected_field, provenance) in provenance_cases {
        assert!(matches!(
            provenance.validate_required_fields(),
            Err(EvidenceValidationError::EmptyField { field }) if field == expected_field
        ));
    }
}

#[test]
fn sampling_rejects_empty_coverage_bucket_before_graph_registration() {
    let sampling = SamplingPlan::new(1, 42, "fixed-v1", Vec::new(), [("", 1)]);

    assert!(matches!(
        sampling.validate_required_fields(),
        Err(EvidenceValidationError::EmptyField { field }) if field == "coverage"
    ));
}

#[test]
fn graph_rejects_empty_coverage_bucket_before_registration() {
    let invalid = Evidence::new(
        EvidenceId::new("checkout", "empty-coverage").expect("valid evidence id"),
        EvidenceMethod::Case,
        EvidenceSubject::Claim(
            ClaimId::new("checkout", "cancel-rejects-shipped").expect("valid claim id"),
        ),
        EvidenceOutcome::Pass,
        ExecutionContext::new(
            ExecutionIdentity::new("runner", "target", "commit", "digest"),
            SamplingPlan::new(1, 42, "fixed-v1", Vec::new(), [("", 1)]),
        ),
        valid_provenance(),
        Independence::SameAuthor,
    );
    let mut graph = EvidenceGraph::default();

    assert!(matches!(
        graph.add_evidence(invalid),
        Err(GraphError::InvalidEvidence {
            source: EvidenceValidationError::EmptyField { field: "coverage" }
        })
    ));
    assert!(graph.evidence().is_empty());
}

#[test]
fn graph_rejects_invalid_evidence_before_registration() {
    let mut invalid = valid_evidence();
    invalid = Evidence::new(
        invalid.id().clone(),
        invalid.method(),
        invalid.subject().clone(),
        invalid.outcome(),
        ExecutionContext::new(
            ExecutionIdentity::new("", "aarch64-apple-darwin", "commit-1", "sha256:1"),
            SamplingPlan::new(1, 42, "fixed-v1", Vec::new(), [("all", 1)]),
        ),
        invalid.provenance().clone(),
        invalid.independence(),
    );

    let mut graph = EvidenceGraph::default();
    assert!(matches!(
        graph.add_evidence(invalid),
        Err(GraphError::InvalidEvidence {
            source: EvidenceValidationError::EmptyField { field: "runner" }
        })
    ));
    assert!(graph.evidence().is_empty());
}

#[test]
fn valid_evidence_passes_required_field_validation() {
    assert!(valid_evidence().validate_required_fields().is_ok());
}
