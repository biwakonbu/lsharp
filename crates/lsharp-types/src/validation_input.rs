//! M2-03 の versioned intent graph manifest 入力境界。
//!
//! source syntax がまだ graph に接続されていない間も、Rust/selfhost の両方が同じ
//! typed graph を受け取れるよう、JSON manifest の wire shape と referential closure
//! をここで固定する。未知の field、未知の node 参照、未対応 schema version は黙って
//! 無視せず診断する。

use crate::evidence::{
    Edge, Evidence, EvidenceSubject, ExecutionContext, ExecutionIdentity, InvalidationSubject,
    Provenance, ReviewSubject, SamplingPlan,
};
use crate::intent::{
    Assumption, AssumptionId, Claim, ClaimId, ContractId, EvidenceId, Intent, IntentId, IntentNode,
    OpenQuestion, OpenQuestionId, ReviewId, StableId, StableIdError,
};
use crate::validation::IntentGraph;
use lsharp_syntax::span::Span;

mod manifest;

use manifest::{
    EdgeInput, EvidenceInput, IdInput, Manifest, NodeInput, NodeKindInput,
    SUPPORTED_SCHEMA_VERSION, SpanInput, SubjectInput, SubjectKindInput,
};

/// JSON manifest を graph に変換できない理由。
#[derive(Debug, thiserror::Error)]
pub enum ValidationInputError {
    #[error("intent graph manifest の JSON 解析に失敗しました: {0}")]
    Json(#[from] serde_json::Error),
    #[error("intent graph manifest の schema_version {version} は未対応です (対応: 1)")]
    UnsupportedSchemaVersion { version: u32 },
    #[error("stable ID の生成に失敗しました: {0}")]
    StableId(#[from] StableIdError),
    #[error("intent graph node の生成に失敗しました: {0}")]
    Node(#[from] crate::intent::NodeTextError),
    #[error("intent graph の登録に失敗しました: {0}")]
    Graph(#[from] crate::evidence::GraphError),
    #[error("{relation} が存在しない node を参照しています: {id}")]
    MissingNodeReference { relation: &'static str, id: String },
    #[error("span の範囲が不正です: {start}..{end}")]
    InvalidSpan { start: usize, end: usize },
}

/// version 1 の JSON manifest を parse して typed intent graph を返す。
pub fn parse_intent_graph_json(source: &str) -> Result<IntentGraph, ValidationInputError> {
    let document: Manifest = serde_json::from_str(source)?;
    if document.schema_version != SUPPORTED_SCHEMA_VERSION {
        return Err(ValidationInputError::UnsupportedSchemaVersion {
            version: document.schema_version,
        });
    }

    let mut graph = IntentGraph::default();
    for node in document.nodes {
        graph.add_node(build_node(node)?)?;
    }
    for evidence in document.evidence {
        let evidence = build_evidence(evidence)?;
        validate_evidence_subject(&graph, evidence.subject())?;
        graph.add_evidence(evidence)?;
    }
    for edge in document.edges {
        let edge = build_edge(edge)?;
        validate_edge_references(&graph, &edge)?;
        graph.add_edge(edge)?;
    }

    Ok(graph)
}

fn build_node(input: NodeInput) -> Result<IntentNode, ValidationInputError> {
    let span = span(input.span)?;
    Ok(match input.kind {
        NodeKindInput::Intent => IntentNode::Intent(Intent::new(
            IntentId::new(input.namespace, input.key)?,
            input.text,
            span,
        )?),
        NodeKindInput::Claim => IntentNode::Claim(Claim::new(
            ClaimId::new(input.namespace, input.key)?,
            input.text,
            span,
        )?),
        NodeKindInput::Assumption => IntentNode::Assumption(Assumption::new(
            AssumptionId::new(input.namespace, input.key)?,
            input.text,
            span,
        )?),
        NodeKindInput::OpenQuestion => IntentNode::OpenQuestion(OpenQuestion::new(
            OpenQuestionId::new(input.namespace, input.key)?,
            input.text,
            span,
        )?),
    })
}

fn span(input: SpanInput) -> Result<Span, ValidationInputError> {
    if input.start > input.end {
        return Err(ValidationInputError::InvalidSpan {
            start: input.start,
            end: input.end,
        });
    }
    Ok(Span::new(input.start, input.end))
}

fn build_evidence(input: EvidenceInput) -> Result<Evidence, ValidationInputError> {
    let subject = match input.subject.kind {
        SubjectKindInput::Intent => {
            EvidenceSubject::Intent(IntentId::new(input.subject.namespace, input.subject.key)?)
        }
        SubjectKindInput::Claim => {
            EvidenceSubject::Claim(ClaimId::new(input.subject.namespace, input.subject.key)?)
        }
        SubjectKindInput::Contract => {
            EvidenceSubject::Contract(ContractId::new(input.subject.namespace, input.subject.key)?)
        }
        SubjectKindInput::Evidence | SubjectKindInput::Review => {
            return Err(ValidationInputError::MissingNodeReference {
                relation: "evidence.subject",
                id: format!(
                    "{}:{}/{}",
                    input.subject.kind.as_str(),
                    input.subject.namespace,
                    input.subject.key
                ),
            });
        }
    };
    let execution = ExecutionContext::new(
        ExecutionIdentity::new(
            input.execution.runner,
            input.execution.target,
            input.execution.source_commit,
            input.execution.artifact_digest,
        ),
        SamplingPlan::new(
            input.execution.sampling.cases,
            input.execution.sampling.seed,
            input.execution.sampling.generator,
            input.execution.sampling.shrinks,
            input.execution.sampling.coverage,
        ),
    );
    Ok(Evidence::new(
        EvidenceId::new(input.namespace, input.key)?,
        input.method.into(),
        subject,
        input.outcome.into(),
        execution,
        Provenance::new(
            input.provenance.producer,
            input.provenance.tool_version,
            input.provenance.timestamp,
        ),
        input.independence.into(),
    ))
}

fn build_edge(input: EdgeInput) -> Result<Edge, ValidationInputError> {
    Ok(match input {
        EdgeInput::Motivates { intent, claim } => Edge::Motivates {
            intent: intent_id(intent)?,
            claim: claim_id(claim)?,
        },
        EdgeInput::ConstrainedBy { claim, assumption } => Edge::ConstrainedBy {
            claim: claim_id(claim)?,
            assumption: assumption_id(assumption)?,
        },
        EdgeInput::TestedBy { claim, contract } => Edge::TestedBy {
            claim: claim_id(claim)?,
            contract: contract_id(contract)?,
        },
        EdgeInput::Supports { observation, claim } => Edge::Supports {
            observation: evidence_id(observation)?,
            claim: claim_id(claim)?,
        },
        EdgeInput::Contradicts { observation, claim } => Edge::Contradicts {
            observation: evidence_id(observation)?,
            claim: claim_id(claim)?,
        },
        EdgeInput::Evaluates { review, subject } => Edge::Evaluates {
            review: review_id(review)?,
            subject: review_subject(subject)?,
        },
        EdgeInput::Invalidates { change, subject } => Edge::Invalidates {
            change: crate::intent::ChangeId::new(change.namespace, change.key)?,
            subject: invalidation_subject(subject)?,
        },
    })
}

fn intent_id(input: IdInput) -> Result<IntentId, ValidationInputError> {
    Ok(IntentId::new(input.namespace, input.key)?)
}

fn claim_id(input: IdInput) -> Result<ClaimId, ValidationInputError> {
    Ok(ClaimId::new(input.namespace, input.key)?)
}

fn assumption_id(input: IdInput) -> Result<AssumptionId, ValidationInputError> {
    Ok(AssumptionId::new(input.namespace, input.key)?)
}

fn contract_id(input: IdInput) -> Result<ContractId, ValidationInputError> {
    Ok(ContractId::new(input.namespace, input.key)?)
}

fn evidence_id(input: IdInput) -> Result<EvidenceId, ValidationInputError> {
    Ok(EvidenceId::new(input.namespace, input.key)?)
}

fn review_id(input: IdInput) -> Result<ReviewId, ValidationInputError> {
    Ok(ReviewId::new(input.namespace, input.key)?)
}

fn review_subject(input: SubjectInput) -> Result<ReviewSubject, ValidationInputError> {
    Ok(match input.kind {
        SubjectKindInput::Intent => {
            ReviewSubject::Intent(IntentId::new(input.namespace, input.key)?)
        }
        SubjectKindInput::Claim => ReviewSubject::Claim(ClaimId::new(input.namespace, input.key)?),
        SubjectKindInput::Evidence => {
            ReviewSubject::Evidence(EvidenceId::new(input.namespace, input.key)?)
        }
        SubjectKindInput::Contract | SubjectKindInput::Review => {
            return Err(ValidationInputError::MissingNodeReference {
                relation: "evaluates.subject",
                id: format!("{}:{}/{}", input.kind.as_str(), input.namespace, input.key),
            });
        }
    })
}

fn invalidation_subject(input: SubjectInput) -> Result<InvalidationSubject, ValidationInputError> {
    Ok(match input.kind {
        SubjectKindInput::Evidence => {
            InvalidationSubject::Evidence(EvidenceId::new(input.namespace, input.key)?)
        }
        SubjectKindInput::Review => {
            InvalidationSubject::Review(ReviewId::new(input.namespace, input.key)?)
        }
        SubjectKindInput::Intent | SubjectKindInput::Claim | SubjectKindInput::Contract => {
            return Err(ValidationInputError::MissingNodeReference {
                relation: "invalidates.subject",
                id: format!("{}:{}/{}", input.kind.as_str(), input.namespace, input.key),
            });
        }
    })
}

fn validate_evidence_subject(
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

fn validate_edge_references(graph: &IntentGraph, edge: &Edge) -> Result<(), ValidationInputError> {
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
