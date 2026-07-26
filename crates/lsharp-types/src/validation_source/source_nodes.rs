use super::SourceGraphError;
use crate::intent::{IntentNode, NodeKind};
use crate::validation::IntentGraph;
use lsharp_syntax::ast::{Decl, Metadata};
use lsharp_syntax::metadata::MetadataFormKind;

pub(super) fn add_decl_nodes(decl: &Decl, graph: &mut IntentGraph) -> Result<(), SourceGraphError> {
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
        } => add_metadata_nodes(metadata, graph),
        Decl::ModuleDecl { body, .. } | Decl::ImplDef { methods: body, .. } => {
            for nested in body {
                add_decl_nodes(nested, graph)?;
            }
            Ok(())
        }
        Decl::Private { inner, .. } => add_decl_nodes(inner, graph),
        _ => Ok(()),
    }
}

fn add_metadata_nodes(
    metadata: &Metadata,
    graph: &mut IntentGraph,
) -> Result<(), SourceGraphError> {
    for form in &metadata.forms {
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
