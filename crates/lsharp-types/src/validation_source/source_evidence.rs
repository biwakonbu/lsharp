use super::{SourceGraphError, require_node};
use crate::evidence::{
    Evidence, EvidenceMethod, EvidenceOutcome, EvidenceSubject, ExecutionContext,
    ExecutionIdentity, Independence, Provenance, SamplingPlan,
};
use crate::intent::{ClaimId, ContractId, EvidenceId, IntentId, NodeKind, StableId};
use crate::validation::IntentGraph;
use lsharp_syntax::ast::{Decl, Metadata};
use lsharp_syntax::metadata::MetadataFormKind;
use lsharp_syntax::span::Span;

pub(super) fn add_decl_evidence(
    decl: &Decl,
    graph: &mut IntentGraph,
    evidence_spans: &mut Vec<(String, Span)>,
) -> Result<(), SourceGraphError> {
    match decl {
        Decl::Defn {
            metadata: Some(metadata),
            ..
        }
        | Decl::TypeDef {
            metadata: Some(metadata),
            ..
        }
        | Decl::RecordDef {
            metadata: Some(metadata),
            ..
        } => add_metadata_evidence(metadata, graph, evidence_spans),
        Decl::ModuleDecl { body, .. } | Decl::ImplDef { methods: body, .. } => {
            for nested in body {
                add_decl_evidence(nested, graph, evidence_spans)?;
            }
            Ok(())
        }
        Decl::Private { inner, .. } => add_decl_evidence(inner, graph, evidence_spans),
        _ => Ok(()),
    }
}

fn add_metadata_evidence(
    metadata: &Metadata,
    graph: &mut IntentGraph,
    evidence_spans: &mut Vec<(String, Span)>,
) -> Result<(), SourceGraphError> {
    for form in &metadata.forms {
        let MetadataFormKind::Evidence { record } = &form.kind else {
            continue;
        };
        let evidence = build_source_evidence(record, graph, form.span())?;
        let id = evidence.id().as_str().to_string();
        if let Some((_, first_span)) = evidence_spans.iter().find(|(existing, _)| existing == &id) {
            return Err(SourceGraphError::DuplicateEvidence {
                id,
                first_span: *first_span,
                duplicate_span: form.span(),
            });
        }
        graph.add_evidence(evidence)?;
        evidence_spans.push((id, form.span()));
    }
    Ok(())
}

fn build_source_evidence(
    record: &lsharp_syntax::metadata::EvidenceForm,
    graph: &IntentGraph,
    span: Span,
) -> Result<Evidence, SourceGraphError> {
    validate_required_source_evidence_fields(record, span)?;
    let id = EvidenceId::parse(record.id().to_string())
        .map_err(|source| SourceGraphError::EvidenceIdAt { span, source })?;
    let subject = parse_evidence_subject(record.subject(), graph, span)?;
    let method = parse_evidence_method(record.method(), span)?;
    let outcome = parse_evidence_outcome(record.outcome(), span)?;
    let independence = parse_independence(record.independence(), span)?;
    validate_sampling_coverage(record, span)?;
    let execution = ExecutionContext::new(
        ExecutionIdentity::new(
            record.runner().to_string(),
            record.target().to_string(),
            record.source_commit().to_string(),
            record.artifact_digest().to_string(),
        ),
        SamplingPlan::new(
            record.cases(),
            record.seed(),
            record.generator().to_string(),
            record.shrinks().to_vec(),
            record.coverage().iter().cloned(),
        ),
    );
    Ok(Evidence::new(
        id,
        method,
        subject,
        outcome,
        execution,
        Provenance::new(
            record.producer().to_string(),
            record.tool_version().to_string(),
            record.timestamp().to_string(),
        ),
        independence,
    ))
}

fn validate_sampling_coverage(
    record: &lsharp_syntax::metadata::EvidenceForm,
    span: Span,
) -> Result<(), SourceGraphError> {
    for (bucket, _) in record.coverage() {
        if bucket.trim().is_empty() {
            return Err(SourceGraphError::InvalidEvidenceField {
                field: "coverage",
                value: bucket.clone(),
                span,
            });
        }
    }
    Ok(())
}

fn validate_required_source_evidence_fields(
    record: &lsharp_syntax::metadata::EvidenceForm,
    span: Span,
) -> Result<(), SourceGraphError> {
    for (field, value) in [
        ("runner", record.runner()),
        ("target", record.target()),
        ("source_commit", record.source_commit()),
        ("artifact_digest", record.artifact_digest()),
        ("generator", record.generator()),
        ("producer", record.producer()),
        ("tool_version", record.tool_version()),
        ("timestamp", record.timestamp()),
    ] {
        if value.trim().is_empty() {
            return Err(SourceGraphError::InvalidEvidenceRequiredField {
                field,
                value: value.to_string(),
                span,
            });
        }
    }
    Ok(())
}

fn parse_evidence_subject(
    wire_id: &str,
    graph: &IntentGraph,
    span: Span,
) -> Result<EvidenceSubject, SourceGraphError> {
    let stable_id = StableId::parse(wire_id.to_string())
        .map_err(|source| SourceGraphError::EvidenceSubjectIdAt { span, source })?;
    match stable_id.kind() {
        NodeKind::Intent => {
            let id = IntentId::parse(wire_id.to_string())
                .map_err(|source| SourceGraphError::EvidenceSubjectIdAt { span, source })?;
            require_node(graph, "evidence.subject", id.stable_id(), span)?;
            Ok(EvidenceSubject::Intent(id))
        }
        NodeKind::Claim => {
            let id = ClaimId::parse(wire_id.to_string())
                .map_err(|source| SourceGraphError::EvidenceSubjectIdAt { span, source })?;
            require_node(graph, "evidence.subject", id.stable_id(), span)?;
            Ok(EvidenceSubject::Claim(id))
        }
        NodeKind::Contract => {
            let id = ContractId::parse(wire_id.to_string())
                .map_err(|source| SourceGraphError::EvidenceSubjectIdAt { span, source })?;
            Ok(EvidenceSubject::Contract(id))
        }
        _ => Err(SourceGraphError::InvalidEvidenceField {
            field: "subject",
            value: wire_id.to_string(),
            span,
        }),
    }
}

fn parse_evidence_method(value: &str, span: Span) -> Result<EvidenceMethod, SourceGraphError> {
    match value {
        "example" => Ok(EvidenceMethod::Example),
        "case" => Ok(EvidenceMethod::Case),
        "assert" => Ok(EvidenceMethod::Assert),
        "property" => Ok(EvidenceMethod::Property),
        "production" => Ok(EvidenceMethod::Production),
        "reference" => Ok(EvidenceMethod::Reference),
        "proof" => Ok(EvidenceMethod::Proof),
        "review" => Ok(EvidenceMethod::Review),
        _ => Err(SourceGraphError::InvalidEvidenceField {
            field: "method",
            value: value.to_string(),
            span,
        }),
    }
}

fn parse_evidence_outcome(value: &str, span: Span) -> Result<EvidenceOutcome, SourceGraphError> {
    match value {
        "pass" => Ok(EvidenceOutcome::Pass),
        "fail" => Ok(EvidenceOutcome::Fail),
        "contradicted" => Ok(EvidenceOutcome::Contradicted),
        "unknown" => Ok(EvidenceOutcome::Unknown),
        "stale" => Ok(EvidenceOutcome::Stale),
        _ => Err(SourceGraphError::InvalidEvidenceField {
            field: "outcome",
            value: value.to_string(),
            span,
        }),
    }
}

fn parse_independence(value: &str, span: Span) -> Result<Independence, SourceGraphError> {
    match value {
        "same-author" => Ok(Independence::SameAuthor),
        "independent-review" => Ok(Independence::IndependentReview),
        "external-observation" => Ok(Independence::ExternalObservation),
        _ => Err(SourceGraphError::InvalidEvidenceField {
            field: "independence",
            value: value.to_string(),
            span,
        }),
    }
}
