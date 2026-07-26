use crate::evidence::{Edge, EvidenceSubject, ReviewSubject};
use crate::intent::StableId;
use crate::validation::IntentGraph;

use super::ValidationInputError;

/// evidence subject と既存 node の参照閉包を検証する。
pub(super) fn validate_evidence_subject(
    graph: &IntentGraph,
    subject: &EvidenceSubject,
) -> Result<(), ValidationInputError> {
    match subject {
        EvidenceSubject::Intent(id) if !has_node(graph, id.stable_id()) => {
            missing_node("evidence.subject", id.stable_id())
        }
        EvidenceSubject::Claim(id) if !has_node(graph, id.stable_id()) => {
            missing_node("evidence.subject", id.stable_id())
        }
        EvidenceSubject::Intent(_) | EvidenceSubject::Claim(_) | EvidenceSubject::Contract(_) => {
            Ok(())
        }
    }
}

/// edge の graph-owned endpoint を既存 node に閉じ込める。
pub(super) fn validate_edge_references(
    graph: &IntentGraph,
    edge: &Edge,
) -> Result<(), ValidationInputError> {
    match edge {
        Edge::Motivates { intent, claim } => {
            require_node(graph, "motivates.intent", intent.stable_id())?;
            require_node(graph, "motivates.claim", claim.stable_id())?;
        }
        Edge::ConstrainedBy { claim, assumption } => {
            require_node(graph, "constrained-by.claim", claim.stable_id())?;
            require_node(graph, "constrained-by.assumption", assumption.stable_id())?;
        }
        Edge::TestedBy { claim, .. } => {
            require_node(graph, "tested-by.claim", claim.stable_id())?;
        }
        Edge::Supports { claim, .. } | Edge::Contradicts { claim, .. } => {
            require_node(graph, edge.relation(), claim.stable_id())?;
        }
        Edge::Evaluates { subject, .. } => match subject {
            ReviewSubject::Intent(id) => {
                require_node(graph, "evaluates.subject", id.stable_id())?;
            }
            ReviewSubject::Claim(id) => {
                require_node(graph, "evaluates.subject", id.stable_id())?;
            }
            ReviewSubject::Evidence(_) => {}
        },
        Edge::Invalidates { .. } => {}
    }
    Ok(())
}

fn require_node(
    graph: &IntentGraph,
    relation: &'static str,
    id: &StableId,
) -> Result<(), ValidationInputError> {
    if has_node(graph, id) {
        Ok(())
    } else {
        missing_node(relation, id)
    }
}

fn has_node(graph: &IntentGraph, id: &StableId) -> bool {
    graph.nodes().iter().any(|node| node.stable_id() == id)
}

fn missing_node(relation: &'static str, id: &StableId) -> Result<(), ValidationInputError> {
    Err(ValidationInputError::MissingNodeReference {
        relation,
        id: id.as_str().to_string(),
    })
}
