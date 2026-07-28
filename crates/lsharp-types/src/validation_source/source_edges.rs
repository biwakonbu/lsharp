use super::{SourceGraphError, require_evidence, require_node};
use crate::evidence::{Edge, InvalidationSubject, ReviewSubject};
use crate::intent::{
    AssumptionId, ChangeId, ClaimId, ContractId, EvidenceId, IntentId, NodeKind, ReviewId,
    StableId, StableIdError,
};
use crate::validation::IntentGraph;
use lsharp_syntax::ast::{Decl, Metadata};
use lsharp_syntax::metadata::MetadataFormKind;
use lsharp_syntax::span::Span;

pub(super) fn add_decl_edges(decl: &Decl, graph: &mut IntentGraph) -> Result<(), SourceGraphError> {
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
        } => add_metadata_edges(metadata, graph),
        Decl::ModuleDecl { body, .. } | Decl::ImplDef { methods: body, .. } => {
            for nested in body {
                add_decl_edges(nested, graph)?;
            }
            Ok(())
        }
        Decl::Private { inner, .. } => add_decl_edges(inner, graph),
        _ => Ok(()),
    }
}

fn add_metadata_edges(
    metadata: &Metadata,
    graph: &mut IntentGraph,
) -> Result<(), SourceGraphError> {
    for form in &metadata.forms {
        match &form.kind {
            MetadataFormKind::Motivates { intent, claim } => {
                let intent =
                    parse_edge_id(intent, "motivates.intent", form.span(), IntentId::parse)?;
                let claim = parse_edge_id(claim, "motivates.claim", form.span(), ClaimId::parse)?;
                require_node(graph, "motivates.intent", intent.stable_id(), form.span())?;
                require_node(graph, "motivates.claim", claim.stable_id(), form.span())?;
                graph.add_edge(Edge::Motivates { intent, claim })?;
            }
            MetadataFormKind::ConstrainedBy { claim, assumption } => {
                let claim =
                    parse_edge_id(claim, "constrained-by.claim", form.span(), ClaimId::parse)?;
                let assumption = parse_edge_id(
                    assumption,
                    "constrained-by.assumption",
                    form.span(),
                    AssumptionId::parse,
                )?;
                require_node(
                    graph,
                    "constrained-by.claim",
                    claim.stable_id(),
                    form.span(),
                )?;
                require_node(
                    graph,
                    "constrained-by.assumption",
                    assumption.stable_id(),
                    form.span(),
                )?;
                graph.add_edge(Edge::ConstrainedBy { claim, assumption })?;
            }
            MetadataFormKind::TestedBy { claim, contract } => {
                let claim = parse_edge_id(claim, "tested-by.claim", form.span(), ClaimId::parse)?;
                let contract = parse_edge_id(
                    contract,
                    "tested-by.contract",
                    form.span(),
                    ContractId::parse,
                )?;
                require_node(graph, "tested-by.claim", claim.stable_id(), form.span())?;
                graph.add_edge(Edge::TestedBy { claim, contract })?;
            }
            MetadataFormKind::Supports { observation, claim } => {
                require_evidence_wire(graph, "supports", observation, form.span())?;
                let observation = parse_edge_id(
                    observation,
                    "supports.observation",
                    form.span(),
                    EvidenceId::parse,
                )?;
                let claim = parse_edge_id(claim, "supports.claim", form.span(), ClaimId::parse)?;
                require_node(graph, "supports.claim", claim.stable_id(), form.span())?;
                require_evidence(graph, "supports", &observation, form.span())?;
                graph.add_edge(Edge::Supports { observation, claim })?;
            }
            MetadataFormKind::Contradicts { observation, claim } => {
                require_evidence_wire(graph, "contradicts", observation, form.span())?;
                let observation = parse_edge_id(
                    observation,
                    "contradicts.observation",
                    form.span(),
                    EvidenceId::parse,
                )?;
                let claim = parse_edge_id(claim, "contradicts.claim", form.span(), ClaimId::parse)?;
                require_node(graph, "contradicts.claim", claim.stable_id(), form.span())?;
                require_evidence(graph, "contradicts", &observation, form.span())?;
                graph.add_edge(Edge::Contradicts { observation, claim })?;
            }
            MetadataFormKind::Evaluates { review, subject } => {
                let review =
                    parse_edge_id(review, "evaluates.review", form.span(), ReviewId::parse)?;
                let subject = parse_review_subject(subject, "evaluates.subject", form.span())?;
                match &subject {
                    ReviewSubject::Intent(intent) => {
                        require_node(graph, "evaluates.subject", intent.stable_id(), form.span())?
                    }
                    ReviewSubject::Claim(claim) => {
                        require_node(graph, "evaluates.subject", claim.stable_id(), form.span())?
                    }
                    ReviewSubject::Evidence(evidence) => {
                        require_evidence(graph, "evaluates.subject", evidence, form.span())?
                    }
                }
                graph.add_edge(Edge::Evaluates { review, subject })?;
            }
            MetadataFormKind::Invalidates { change, subject } => {
                let change =
                    parse_edge_id(change, "invalidates.change", form.span(), ChangeId::parse)?;
                let subject =
                    parse_invalidation_subject(subject, "invalidates.subject", form.span())?;
                if let InvalidationSubject::Evidence(evidence) = &subject {
                    require_evidence(graph, "invalidates.subject", evidence, form.span())?;
                }
                graph.add_edge(Edge::Invalidates { change, subject })?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_edge_id<T>(
    wire: &str,
    relation: &'static str,
    span: Span,
    parse: impl Fn(String) -> Result<T, StableIdError>,
) -> Result<T, SourceGraphError> {
    parse(wire.to_string()).map_err(|source| SourceGraphError::EdgeIdAt {
        relation,
        span,
        source,
    })
}

fn require_evidence_wire(
    graph: &IntentGraph,
    relation: &'static str,
    wire: &str,
    span: Span,
) -> Result<(), SourceGraphError> {
    if wire.is_empty() {
        return Ok(());
    }
    if graph
        .evidence()
        .iter()
        .any(|evidence| evidence.id().as_str() == wire)
    {
        Ok(())
    } else {
        Err(SourceGraphError::EvidenceRegistryRequired {
            relation,
            evidence_id: wire.to_string(),
            span,
        })
    }
}

fn parse_review_subject(
    wire: &str,
    relation: &'static str,
    span: Span,
) -> Result<ReviewSubject, SourceGraphError> {
    let stable = parse_edge_id(wire, relation, span, StableId::parse)?;
    match stable.kind() {
        NodeKind::Intent => Ok(ReviewSubject::Intent(parse_edge_id(
            wire,
            relation,
            span,
            IntentId::parse,
        )?)),
        NodeKind::Claim => Ok(ReviewSubject::Claim(parse_edge_id(
            wire,
            relation,
            span,
            ClaimId::parse,
        )?)),
        NodeKind::Evidence => Ok(ReviewSubject::Evidence(parse_edge_id(
            wire,
            relation,
            span,
            EvidenceId::parse,
        )?)),
        actual => Err(SourceGraphError::EdgeSubjectKindMismatch {
            relation,
            actual,
            wire_id: wire.to_string(),
            span,
        }),
    }
}

fn parse_invalidation_subject(
    wire: &str,
    relation: &'static str,
    span: Span,
) -> Result<InvalidationSubject, SourceGraphError> {
    let stable = parse_edge_id(wire, relation, span, StableId::parse)?;
    match stable.kind() {
        NodeKind::Review => Ok(InvalidationSubject::Review(parse_edge_id(
            wire,
            relation,
            span,
            ReviewId::parse,
        )?)),
        NodeKind::Evidence => Ok(InvalidationSubject::Evidence(parse_edge_id(
            wire,
            relation,
            span,
            EvidenceId::parse,
        )?)),
        actual => Err(SourceGraphError::EdgeSubjectKindMismatch {
            relation,
            actual,
            wire_id: wire.to_string(),
            span,
        }),
    }
}
