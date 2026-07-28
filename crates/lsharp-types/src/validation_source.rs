//! L# source metadata を intent graph の node registry へ投影する adapter。
//!
//! source syntax では node の wire ID、本文、span、evidence record と typed edge endpoint を
//! 明示的に受け取る。ID の省略や kind の推測は行わず、同じ ID の重複と typed kind mismatch
//! を既存の canonical model のエラーとして返す。

use crate::intent::{EvidenceId, IntentNodeError, NodeKind, ReviewId, StableIdError};
use crate::validation::IntentGraph;
use lsharp_syntax::ast::Program;
use lsharp_syntax::span::Span;

mod source_edges;
mod source_evidence;
mod source_nodes;

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
    #[error("source node の ID 解析に失敗しました (span={span}): {source}")]
    NodeIdAt {
        span: Span,
        #[source]
        source: StableIdError,
    },
    #[error(
        "source evidence の stable ID が重複しています (id={id}, first_span={first_span}, duplicate_span={duplicate_span})"
    )]
    DuplicateEvidence {
        id: String,
        first_span: Span,
        duplicate_span: Span,
    },
    #[error(
        "source review の stable ID が重複しています (id={id}, first_span={first_span}, duplicate_span={duplicate_span})"
    )]
    DuplicateReview {
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
    #[error("source evidence の ID 解析に失敗しました (span={span}): {source}")]
    EvidenceIdAt {
        span: Span,
        #[source]
        source: StableIdError,
    },
    #[error("source review の ID 解析に失敗しました (span={span}): {source}")]
    ReviewIdAt {
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
        "source intent edge が参照する review が registry にありません (relation={relation}, id={id}, span={span})"
    )]
    MissingReviewReference {
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
    #[error("source evidence の required field {field} が空または空白のみです (span={span})")]
    InvalidEvidenceRequiredField {
        field: &'static str,
        value: String,
        span: Span,
    },
    #[error("source node の {field} が空または空白のみです (span={span})")]
    InvalidNodeField {
        field: &'static str,
        value: String,
        span: Span,
    },
    #[error("source review の {field} が不正です (span={span}): {value}")]
    InvalidReviewField {
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
    #[error(
        "source intent edge の subject kind が relation に対して不正です (relation={relation}, actual={actual:?}, id={wire_id}, span={span})"
    )]
    EdgeSubjectKindMismatch {
        relation: &'static str,
        actual: NodeKind,
        wire_id: String,
        span: Span,
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
            | Self::DuplicateEvidence { duplicate_span, .. }
            | Self::DuplicateReview { duplicate_span, .. } => Some(*duplicate_span),
            Self::EdgeIdAt { span, .. }
            | Self::EvidenceIdAt { span, .. }
            | Self::NodeIdAt { span, .. }
            | Self::ReviewIdAt { span, .. }
            | Self::MissingNodeReference { span, .. }
            | Self::MissingReviewReference { span, .. }
            | Self::EvidenceRegistryRequired { span, .. }
            | Self::InvalidEvidenceField { span, .. }
            | Self::InvalidEvidenceRequiredField { span, .. }
            | Self::InvalidNodeField { span, .. }
            | Self::InvalidReviewField { span, .. } => Some(*span),
            Self::Node(_) | Self::Graph(_) | Self::EdgeId(_) | Self::KindMismatch { .. } => None,
            Self::EdgeSubjectKindMismatch { span, .. } => Some(*span),
        }
    }
}

/// source の明示 node metadata と typed edge を version 1 graph へ投影する。
///
/// `:intent` / `:claim` / `:assumption` / `:open-question` と opaque `:review` registry を
/// canonical graph へ投影する。その他の metadata は presentation または executable
/// contract として別の adapter が扱うため、node registry では無視する。source の
/// `:evidence` record は required provenance/sampling fields を canonical `Evidence` へ
/// 投影し、source edge は endpoint を registry へ解決できる `:motivates`、
/// `:constrained-by`、`:tested-by`、`:supports`、`:contradicts` を生成する。evidence
/// record がない supports/contradicts は明示的な registry-required error とする。
pub fn source_program_to_intent_graph(program: &Program) -> Result<IntentGraph, SourceGraphError> {
    let mut graph = IntentGraph::default();
    let mut review_spans = Vec::new();
    for decl in &program.decls {
        source_nodes::add_decl_nodes(decl, &mut graph, &mut review_spans)?;
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

fn require_review(
    graph: &IntentGraph,
    relation: &'static str,
    id: &ReviewId,
    span: Span,
) -> Result<(), SourceGraphError> {
    if graph.reviews().is_empty() || graph.reviews().iter().any(|review| review.id() == id) {
        Ok(())
    } else {
        Err(SourceGraphError::MissingReviewReference {
            relation,
            id: id.as_str().to_string(),
            span,
        })
    }
}
