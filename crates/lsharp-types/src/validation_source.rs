//! L# source metadata を intent graph の node registry へ投影する adapter。
//!
//! source syntax では node の wire ID、本文、span、evidence record と typed edge endpoint を
//! 明示的に受け取る。ID の省略や kind の推測は行わず、同じ ID の重複と typed kind mismatch
//! を既存の canonical model のエラーとして返す。

use crate::evidence::{
    Edge, Evidence, EvidenceMethod, EvidenceOutcome, EvidenceSubject, ExecutionContext,
    ExecutionIdentity, Independence, Provenance, SamplingPlan,
};
use crate::intent::{
    AssumptionId, ClaimId, ContractId, EvidenceId, IntentId, IntentNode, IntentNodeError, NodeKind,
    StableId, StableIdError,
};
use crate::validation::IntentGraph;
use lsharp_syntax::ast::{Decl, Metadata, Program};
use lsharp_syntax::metadata::MetadataFormKind;
use lsharp_syntax::span::Span;

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
    #[error("source intent edge が参照する node がありません (relation={relation}, id={id})")]
    MissingNodeReference { relation: &'static str, id: String },
    #[error(
        "source evidence edge は evidence registry の登録を要求します (relation={relation}, evidence_id={evidence_id})"
    )]
    EvidenceRegistryRequired {
        relation: &'static str,
        evidence_id: String,
    },
    #[error("source evidence の {field} が不正です: {value}")]
    InvalidEvidenceField { field: &'static str, value: String },
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
        add_decl_evidence(decl, &mut graph, &mut evidence_spans)?;
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

fn add_decl_evidence(
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
        let evidence = build_source_evidence(record, graph)?;
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
) -> Result<Evidence, SourceGraphError> {
    let id = EvidenceId::parse(record.id().to_string())?;
    let subject = parse_evidence_subject(record.subject(), graph)?;
    let method = parse_evidence_method(record.method())?;
    let outcome = parse_evidence_outcome(record.outcome())?;
    let independence = parse_independence(record.independence())?;
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

fn parse_evidence_subject(
    wire_id: &str,
    graph: &IntentGraph,
) -> Result<EvidenceSubject, SourceGraphError> {
    let stable_id = StableId::parse(wire_id.to_string())?;
    match stable_id.kind() {
        NodeKind::Intent => {
            let id = IntentId::parse(wire_id.to_string())?;
            require_node(graph, "evidence.subject", id.stable_id())?;
            Ok(EvidenceSubject::Intent(id))
        }
        NodeKind::Claim => {
            let id = ClaimId::parse(wire_id.to_string())?;
            require_node(graph, "evidence.subject", id.stable_id())?;
            Ok(EvidenceSubject::Claim(id))
        }
        NodeKind::Contract => Ok(EvidenceSubject::Contract(ContractId::parse(
            wire_id.to_string(),
        )?)),
        _ => Err(SourceGraphError::InvalidEvidenceField {
            field: "subject",
            value: wire_id.to_string(),
        }),
    }
}

fn parse_evidence_method(value: &str) -> Result<EvidenceMethod, SourceGraphError> {
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
        }),
    }
}

fn parse_evidence_outcome(value: &str) -> Result<EvidenceOutcome, SourceGraphError> {
    match value {
        "pass" => Ok(EvidenceOutcome::Pass),
        "fail" => Ok(EvidenceOutcome::Fail),
        "contradicted" => Ok(EvidenceOutcome::Contradicted),
        "unknown" => Ok(EvidenceOutcome::Unknown),
        "stale" => Ok(EvidenceOutcome::Stale),
        _ => Err(SourceGraphError::InvalidEvidenceField {
            field: "outcome",
            value: value.to_string(),
        }),
    }
}

fn parse_independence(value: &str) -> Result<Independence, SourceGraphError> {
    match value {
        "same-author" => Ok(Independence::SameAuthor),
        "independent-review" => Ok(Independence::IndependentReview),
        "external-observation" => Ok(Independence::ExternalObservation),
        _ => Err(SourceGraphError::InvalidEvidenceField {
            field: "independence",
            value: value.to_string(),
        }),
    }
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
                require_evidence(graph, "supports", &observation)?;
                graph.add_edge(Edge::Supports { observation, claim })?;
            }
            MetadataFormKind::Contradicts { observation, claim } => {
                let observation = EvidenceId::parse(observation.clone())?;
                let claim = ClaimId::parse(claim.clone())?;
                require_node(graph, "contradicts.claim", claim.stable_id())?;
                require_evidence(graph, "contradicts", &observation)?;
                graph.add_edge(Edge::Contradicts { observation, claim })?;
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

fn require_evidence(
    graph: &IntentGraph,
    relation: &'static str,
    id: &EvidenceId,
) -> Result<(), SourceGraphError> {
    if graph.evidence().iter().any(|evidence| evidence.id() == id) {
        Ok(())
    } else {
        Err(SourceGraphError::EvidenceRegistryRequired {
            relation,
            evidence_id: id.as_str().to_string(),
        })
    }
}
