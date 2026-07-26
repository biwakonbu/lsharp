use super::{SourceGraphError, require_evidence, require_node};
use crate::evidence::Edge;
use crate::intent::{AssumptionId, ClaimId, ContractId, EvidenceId, IntentId, StableIdError};
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
            _ => {}
        }
    }
    Ok(())
}

fn parse_edge_id<T>(
    wire: &str,
    relation: &'static str,
    span: Span,
    parse: fn(String) -> Result<T, StableIdError>,
) -> Result<T, SourceGraphError> {
    parse(wire.to_string()).map_err(|source| SourceGraphError::EdgeIdAt {
        relation,
        span,
        source,
    })
}
