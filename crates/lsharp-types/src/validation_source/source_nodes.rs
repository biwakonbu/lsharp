use super::SourceGraphError;
use crate::evidence::{ReviewRecord, ReviewVisibility};
use crate::intent::{IntentNode, NodeKind, ReviewId};
use crate::validation::IntentGraph;
use lsharp_syntax::ast::{Decl, Metadata};
use lsharp_syntax::metadata::MetadataFormKind;
use lsharp_syntax::span::Span;

pub(super) fn add_decl_nodes(
    decl: &Decl,
    graph: &mut IntentGraph,
    review_spans: &mut Vec<(String, Span)>,
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
        } => add_metadata_nodes(metadata, graph, review_spans),
        Decl::ModuleDecl { body, .. } | Decl::ImplDef { methods: body, .. } => {
            for nested in body {
                add_decl_nodes(nested, graph, review_spans)?;
            }
            Ok(())
        }
        Decl::Private { inner, .. } => add_decl_nodes(inner, graph, review_spans),
        _ => Ok(()),
    }
}

fn add_metadata_nodes(
    metadata: &Metadata,
    graph: &mut IntentGraph,
    review_spans: &mut Vec<(String, Span)>,
) -> Result<(), SourceGraphError> {
    for form in &metadata.forms {
        if let MetadataFormKind::Review {
            id,
            provenance_digest,
            visibility,
        } = &form.kind
        {
            if id.is_empty() {
                return Err(SourceGraphError::InvalidReviewField {
                    field: "id",
                    value: id.clone(),
                    span: form.span(),
                });
            }
            if provenance_digest.trim().is_empty() {
                return Err(SourceGraphError::InvalidReviewField {
                    field: "provenance_digest",
                    value: provenance_digest.clone(),
                    span: form.span(),
                });
            }
            let review_id =
                ReviewId::parse(id.clone()).map_err(|source| SourceGraphError::ReviewIdAt {
                    span: form.span(),
                    source,
                })?;
            let visibility = ReviewVisibility::parse(visibility).ok_or_else(|| {
                SourceGraphError::InvalidReviewField {
                    field: "visibility",
                    value: visibility.clone(),
                    span: form.span(),
                }
            })?;
            let review = ReviewRecord::new(review_id, provenance_digest.clone(), visibility);
            let id = review.id().as_str().to_string();
            if let Some((_, first_span)) = review_spans.iter().find(|(existing, _)| existing == &id)
            {
                return Err(SourceGraphError::DuplicateReview {
                    id,
                    first_span: *first_span,
                    duplicate_span: form.span(),
                });
            }
            graph.add_review(review)?;
            review_spans.push((id, form.span()));
            continue;
        }
        let (expected_kind, wire_id, text) = match &form.kind {
            MetadataFormKind::Intent { id, text } => (NodeKind::Intent, id, text),
            MetadataFormKind::Claim { id, text } => (NodeKind::Claim, id, text),
            MetadataFormKind::Assumption { id, text } => (NodeKind::Assumption, id, text),
            MetadataFormKind::OpenQuestion { id, text } => (NodeKind::OpenQuestion, id, text),
            _ => continue,
        };
        let node = IntentNode::from_wire_parts(wire_id.clone(), text.clone(), form.span())?;
        if node.kind() != expected_kind {
            return Err(SourceGraphError::KindMismatch {
                expected: expected_kind,
                actual: node.kind(),
                wire_id: wire_id.clone(),
            });
        }
        if let Some(existing) = graph
            .nodes()
            .iter()
            .find(|existing| existing.stable_id() == node.stable_id())
        {
            return Err(SourceGraphError::DuplicateNode {
                id: node.stable_id().as_str().to_string(),
                first_span: existing.source_span(),
                duplicate_span: form.span(),
            });
        }
        graph.add_node(node)?;
    }
    Ok(())
}
