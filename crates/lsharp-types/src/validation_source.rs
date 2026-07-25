//! L# source metadata を intent graph の node registry へ投影する adapter。
//!
//! source syntax では node の wire ID、本文、span と typed edge endpoint を明示的に受け取り、
//! evidence record 自体は別の入力境界で扱う。ID の省略や kind の推測は行わず、同じ ID の
//! 重複と typed kind mismatch を既存の canonical model のエラーとして返す。

use crate::evidence::Edge;
use crate::intent::{
    AssumptionId, ClaimId, ContractId, EvidenceId, IntentId, IntentNode, IntentNodeError, NodeKind,
    StableIdError,
};
use crate::validation::IntentGraph;
use lsharp_syntax::ast::{Decl, Metadata, Program};
use lsharp_syntax::metadata::MetadataFormKind;

/// source node adapter が入力を graph へ投影できない理由。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceGraphError {
    #[error("source intent node の生成に失敗しました: {0}")]
    Node(#[from] IntentNodeError),
    #[error("source intent graph の登録に失敗しました: {0}")]
    Graph(#[from] crate::evidence::GraphError),
    #[error("source intent edge の ID 解析に失敗しました: {0}")]
    EdgeId(#[from] StableIdError),
    #[error("source intent edge が参照する node がありません (relation={relation}, id={id})")]
    MissingNodeReference { relation: &'static str, id: String },
    #[error(
        "source evidence edge は evidence registry の登録を要求します (relation={relation}, evidence_id={evidence_id})"
    )]
    EvidenceRegistryRequired {
        relation: &'static str,
        evidence_id: String,
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

/// source の明示 node metadata と typed edge を version 1 graph へ投影する。
///
/// `:intent` / `:claim` / `:assumption` / `:open-question` 以外の metadata は
/// presentation または executable contract として別の adapter が扱うため、node/edge
/// registry では無視する。source edge は endpoint を node registry へ解決できる
/// `:motivates`、`:constrained-by`、`:tested-by` を生成する。`:supports` と
/// `:contradicts` は evidence registry が未接続のため、ID と claim endpoint を検証した
/// うえで明示的なエラーを返す。contract/evidence の実体定義と evidence record 投入は
/// 別の adapter の責務として残す。
pub fn source_program_to_intent_graph(program: &Program) -> Result<IntentGraph, SourceGraphError> {
    let mut graph = IntentGraph::default();
    for decl in &program.decls {
        add_decl_nodes(decl, &mut graph)?;
    }
    for decl in &program.decls {
        add_decl_edges(decl, &mut graph)?;
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
        graph.add_node(node)?;
    }
    Ok(())
}

fn add_decl_edges(decl: &Decl, graph: &mut IntentGraph) -> Result<(), SourceGraphError> {
    match decl {
        Decl::Defn {
            metadata: Some(metadata),
            ..
        }
        | Decl::TypeDef {
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
                let intent = IntentId::parse(intent.clone())?;
                let claim = ClaimId::parse(claim.clone())?;
                require_node(graph, "motivates.intent", intent.stable_id())?;
                require_node(graph, "motivates.claim", claim.stable_id())?;
                graph.add_edge(Edge::Motivates { intent, claim })?;
            }
            MetadataFormKind::ConstrainedBy { claim, assumption } => {
                let claim = ClaimId::parse(claim.clone())?;
                let assumption = AssumptionId::parse(assumption.clone())?;
                require_node(graph, "constrained-by.claim", claim.stable_id())?;
                require_node(graph, "constrained-by.assumption", assumption.stable_id())?;
                graph.add_edge(Edge::ConstrainedBy { claim, assumption })?;
            }
            MetadataFormKind::TestedBy { claim, contract } => {
                let claim = ClaimId::parse(claim.clone())?;
                let contract = ContractId::parse(contract.clone())?;
                require_node(graph, "tested-by.claim", claim.stable_id())?;
                graph.add_edge(Edge::TestedBy { claim, contract })?;
            }
            MetadataFormKind::Supports { observation, claim } => {
                let observation = EvidenceId::parse(observation.clone())?;
                let claim = ClaimId::parse(claim.clone())?;
                require_node(graph, "supports.claim", claim.stable_id())?;
                return Err(SourceGraphError::EvidenceRegistryRequired {
                    relation: "supports",
                    evidence_id: observation.as_str().to_string(),
                });
            }
            MetadataFormKind::Contradicts { observation, claim } => {
                let observation = EvidenceId::parse(observation.clone())?;
                let claim = ClaimId::parse(claim.clone())?;
                require_node(graph, "contradicts.claim", claim.stable_id())?;
                return Err(SourceGraphError::EvidenceRegistryRequired {
                    relation: "contradicts",
                    evidence_id: observation.as_str().to_string(),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

fn require_node(
    graph: &IntentGraph,
    relation: &'static str,
    id: &crate::intent::StableId,
) -> Result<(), SourceGraphError> {
    if graph.nodes().iter().any(|node| node.stable_id() == id) {
        Ok(())
    } else {
        Err(SourceGraphError::MissingNodeReference {
            relation,
            id: id.as_str().to_string(),
        })
    }
}
