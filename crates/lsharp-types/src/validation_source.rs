//! L# source metadata を intent graph の node registry へ投影する adapter。
//!
//! source syntax では node の wire ID、本文、span、evidence record と typed edge endpoint を
//! 明示的に受け取る。ID の省略や kind の推測は行わず、同じ ID の重複と typed kind mismatch
//! を既存の canonical model のエラーとして返す。

use crate::intent::{EvidenceId, IntentNode, IntentNodeError, NodeKind, StableIdError};
use crate::validation::IntentGraph;
use lsharp_syntax::ast::{Decl, Metadata, Program};
use lsharp_syntax::metadata::MetadataFormKind;
use lsharp_syntax::span::Span;

mod source_edges;
mod source_evidence;

/// source node adapter が入力を graph へ投影できない理由。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceGraphError {
    #[error("source intent node の生成に失敗しました: {0}")]
    Node(#[from] IntentNodeError),
    #[error(
        "source intent node の stable ID が重複しています (id={id}, first_span={first_span}, duplicate_span={duplicate_span})"
    )]
    DuplicateNode {
        id: String,
        first_span: Span,
        duplicate_span: Span,
    },
    #[error(
        "source evidence の stable ID が重複しています (id={id}, first_span={first_span}, duplicate_span={duplicate_span})"
    )]
    DuplicateEvidence {
        id: String,
        first_span: Span,
        duplicate_span: Span,
    },
    #[error("source intent graph の登録に失敗しました: {0}")]
    Graph(#[from] crate::evidence::GraphError),
    #[error("source intent edge の ID 解析に失敗しました: {0}")]
    EdgeId(#[from] StableIdError),
    #[error(
        "source intent edge の ID 解析に失敗しました (relation={relation}, span={span}): {source}"
    )]
    EdgeIdAt {
        relation: &'static str,
        span: Span,
        #[source]
        source: StableIdError,
    },
    #[error(
        "source intent edge が参照する node がありません (relation={relation}, id={id}, span={span})"
    )]
    MissingNodeReference {
        relation: &'static str,
        id: String,
        span: Span,
    },
    #[error(
        "source evidence edge は evidence registry の登録を要求します (relation={relation}, evidence_id={evidence_id})"
    )]
    EvidenceRegistryRequired {
        relation: &'static str,
        evidence_id: String,
        span: Span,
    },
    #[error("source evidence の {field} が不正です (span={span}): {value}")]
    InvalidEvidenceField {
        field: &'static str,
        value: String,
        span: Span,
    },
    #[error(
        "source metadata の node kind と stable ID が不一致です (expected={expected:?}, actual={actual:?}, id={wire_id})"
    )]
    KindMismatch {
        expected: NodeKind,
        actual: NodeKind,
        wire_id: String,
    },
}

impl SourceGraphError {
    /// source adapter が保持している primary span を返す。
    ///
    /// graph-only の内部エラーは source directive に対応する span をまだ持たないため、
    /// CLI は従来どおりメッセージだけを表示する。重複エラーは後続の declaration を
    /// primary span とし、最初の declaration はエラーメッセージ内の補助情報として残す。
    pub fn source_span(&self) -> Option<Span> {
        match self {
            Self::DuplicateNode { duplicate_span, .. }
            | Self::DuplicateEvidence { duplicate_span, .. } => Some(*duplicate_span),
            Self::EdgeIdAt { span, .. }
            | Self::MissingNodeReference { span, .. }
            | Self::EvidenceRegistryRequired { span, .. }
            | Self::InvalidEvidenceField { span, .. } => Some(*span),
            Self::Node(_) | Self::Graph(_) | Self::EdgeId(_) | Self::KindMismatch { .. } => None,
        }
    }
}

/// source の明示 node metadata と typed edge を version 1 graph へ投影する。
///
/// `:intent` / `:claim` / `:assumption` / `:open-question` 以外の metadata は
/// presentation または executable contract として別の adapter が扱うため、node registry
/// では無視する。source の `:evidence` record は required provenance/sampling fields を
/// canonical `Evidence` へ投影し、source edge は endpoint を registry へ解決できる
/// `:motivates`、`:constrained-by`、`:tested-by`、`:supports`、`:contradicts` を生成する。
/// evidence record がない supports/contradicts は明示的な registry-required error とする。
pub fn source_program_to_intent_graph(program: &Program) -> Result<IntentGraph, SourceGraphError> {
    let mut graph = IntentGraph::default();
    for decl in &program.decls {
        add_decl_nodes(decl, &mut graph)?;
    }
    let mut evidence_spans = Vec::new();
    for decl in &program.decls {
        source_evidence::add_decl_evidence(decl, &mut graph, &mut evidence_spans)?;
    }
    for decl in &program.decls {
        source_edges::add_decl_edges(decl, &mut graph)?;
    }
    Ok(graph)
}

fn add_decl_nodes(decl: &Decl, graph: &mut IntentGraph) -> Result<(), SourceGraphError> {
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

fn require_node(
    graph: &IntentGraph,
    relation: &'static str,
    id: &crate::intent::StableId,
    span: Span,
) -> Result<(), SourceGraphError> {
    if graph.nodes().iter().any(|node| node.stable_id() == id) {
        Ok(())
    } else {
        Err(SourceGraphError::MissingNodeReference {
            relation,
            id: id.as_str().to_string(),
            span,
        })
    }
}

fn require_evidence(
    graph: &IntentGraph,
    relation: &'static str,
    id: &EvidenceId,
    span: Span,
) -> Result<(), SourceGraphError> {
    if graph.evidence().iter().any(|evidence| evidence.id() == id) {
        Ok(())
    } else {
        Err(SourceGraphError::EvidenceRegistryRequired {
            relation,
            evidence_id: id.as_str().to_string(),
            span,
        })
    }
}
