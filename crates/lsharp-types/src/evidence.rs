//! v0.2 M2-02 の evidence graph model。
//!
//! edge の向きを typed ID で表し、異なる node kind を文字列で暗黙に結び付けない。
//! graph の referential closure や欠落検査は M2-03 `validate` の責務として残す。

use crate::intent::{
    AssumptionId, ChangeId, ClaimId, ContractId, EvidenceId, IntentId, ReviewId, StableId,
};
use std::collections::BTreeMap;

/// executable / observed evidence の生成方法。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceMethod {
    Example,
    Case,
    Assert,
    Property,
    Production,
    Reference,
    Proof,
    Review,
}

/// evidence が subject に対して示す結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceOutcome {
    Pass,
    Fail,
    Contradicted,
    Unknown,
    Stale,
}

/// evidence がどの程度独立しているか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Independence {
    SameAuthor,
    IndependentReview,
    ExternalObservation,
}

/// evidence manifest の required string field が不正な理由。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EvidenceValidationError {
    #[error("evidence の必須 field が空です: {field}")]
    EmptyField { field: &'static str },
}

fn validate_required_field(
    field: &'static str,
    value: &str,
) -> Result<(), EvidenceValidationError> {
    if value.trim().is_empty() {
        return Err(EvidenceValidationError::EmptyField { field });
    }
    Ok(())
}

/// evidence が対象とする M2 node または executable contract。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceSubject {
    Intent(IntentId),
    Claim(ClaimId),
    Contract(ContractId),
}

/// 実行 evidence の runner/target/source/artifact identity。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionIdentity {
    runner: String,
    target: String,
    source_commit: String,
    artifact_digest: String,
}

impl ExecutionIdentity {
    pub fn new(
        runner: impl Into<String>,
        target: impl Into<String>,
        source_commit: impl Into<String>,
        artifact_digest: impl Into<String>,
    ) -> Self {
        Self {
            runner: runner.into(),
            target: target.into(),
            source_commit: source_commit.into(),
            artifact_digest: artifact_digest.into(),
        }
    }

    pub fn runner(&self) -> &str {
        &self.runner
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn source_commit(&self) -> &str {
        &self.source_commit
    }

    pub fn artifact_digest(&self) -> &str {
        &self.artifact_digest
    }

    /// 実行境界を再現する required identity が欠落していないことを検査する。
    pub fn validate_required_fields(&self) -> Result<(), EvidenceValidationError> {
        validate_required_field("runner", &self.runner)?;
        validate_required_field("target", &self.target)?;
        validate_required_field("source_commit", &self.source_commit)?;
        validate_required_field("artifact_digest", &self.artifact_digest)
    }
}

/// 実行した case 数と sampling/replay の deterministic metadata。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplingPlan {
    cases: usize,
    seed: u64,
    generator: String,
    shrinks: Vec<u64>,
    coverage: BTreeMap<String, usize>,
}

impl SamplingPlan {
    pub fn new<I, K>(
        cases: usize,
        seed: u64,
        generator: impl Into<String>,
        shrinks: Vec<u64>,
        coverage: I,
    ) -> Self
    where
        I: IntoIterator<Item = (K, usize)>,
        K: Into<String>,
    {
        Self {
            cases,
            seed,
            generator: generator.into(),
            shrinks,
            coverage: coverage
                .into_iter()
                .map(|(bucket, count)| (bucket.into(), count))
                .collect(),
        }
    }

    pub fn cases(&self) -> usize {
        self.cases
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn generator(&self) -> &str {
        &self.generator
    }

    pub fn shrinks(&self) -> &[u64] {
        &self.shrinks
    }

    pub fn coverage(&self) -> &BTreeMap<String, usize> {
        &self.coverage
    }

    /// sampling metadata の required generator が欠落していないことを検査する。
    pub fn validate_required_fields(&self) -> Result<(), EvidenceValidationError> {
        validate_required_field("generator", &self.generator)
    }
}

/// 実行境界と sampling plan を一つの再現可能な context に束ねる。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionContext {
    identity: ExecutionIdentity,
    sampling: SamplingPlan,
}

impl ExecutionContext {
    pub fn new(identity: ExecutionIdentity, sampling: SamplingPlan) -> Self {
        Self { identity, sampling }
    }

    pub fn runner(&self) -> &str {
        self.identity.runner()
    }

    pub fn target(&self) -> &str {
        self.identity.target()
    }

    pub fn source_commit(&self) -> &str {
        self.identity.source_commit()
    }

    pub fn artifact_digest(&self) -> &str {
        self.identity.artifact_digest()
    }

    pub fn cases(&self) -> usize {
        self.sampling.cases()
    }

    pub fn seed(&self) -> u64 {
        self.sampling.seed()
    }

    pub fn generator(&self) -> &str {
        self.sampling.generator()
    }

    pub fn shrinks(&self) -> &[u64] {
        self.sampling.shrinks()
    }

    pub fn coverage(&self) -> &BTreeMap<String, usize> {
        self.sampling.coverage()
    }

    /// execution identity と sampling metadata の required field をまとめて検査する。
    pub fn validate_required_fields(&self) -> Result<(), EvidenceValidationError> {
        self.identity.validate_required_fields()?;
        self.sampling.validate_required_fields()
    }
}

/// producer/tool/timestamp を固定する provenance。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    producer: String,
    tool_version: String,
    timestamp: String,
}

impl Provenance {
    pub fn new(
        producer: impl Into<String>,
        tool_version: impl Into<String>,
        timestamp: impl Into<String>,
    ) -> Self {
        Self {
            producer: producer.into(),
            tool_version: tool_version.into(),
            timestamp: timestamp.into(),
        }
    }

    pub fn producer(&self) -> &str {
        &self.producer
    }

    pub fn tool_version(&self) -> &str {
        &self.tool_version
    }

    pub fn timestamp(&self) -> &str {
        &self.timestamp
    }

    /// provenance の required field が欠落していないことを検査する。
    pub fn validate_required_fields(&self) -> Result<(), EvidenceValidationError> {
        validate_required_field("producer", &self.producer)?;
        validate_required_field("tool_version", &self.tool_version)?;
        validate_required_field("timestamp", &self.timestamp)
    }
}

/// 一つの evidence record。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    id: EvidenceId,
    method: EvidenceMethod,
    subject: EvidenceSubject,
    outcome: EvidenceOutcome,
    execution: ExecutionContext,
    provenance: Provenance,
    independence: Independence,
}

impl Evidence {
    pub fn new(
        id: EvidenceId,
        method: EvidenceMethod,
        subject: EvidenceSubject,
        outcome: EvidenceOutcome,
        execution: ExecutionContext,
        provenance: Provenance,
        independence: Independence,
    ) -> Self {
        Self {
            id,
            method,
            subject,
            outcome,
            execution,
            provenance,
            independence,
        }
    }

    pub fn id(&self) -> &EvidenceId {
        &self.id
    }

    pub fn method(&self) -> EvidenceMethod {
        self.method
    }

    pub fn subject(&self) -> &EvidenceSubject {
        &self.subject
    }

    pub fn outcome(&self) -> EvidenceOutcome {
        self.outcome
    }

    pub fn execution(&self) -> &ExecutionContext {
        &self.execution
    }

    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    pub fn independence(&self) -> Independence {
        self.independence
    }

    /// evidence を manifest へ投影する前に required field を fail-closed 検査する。
    pub fn validate_required_fields(&self) -> Result<(), EvidenceValidationError> {
        self.execution.validate_required_fields()?;
        self.provenance.validate_required_fields()
    }
}

/// review が評価する対象。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviewSubject {
    Intent(IntentId),
    Claim(ClaimId),
    Evidence(EvidenceId),
}

/// change が失効させる対象。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidationSubject {
    Review(ReviewId),
    Evidence(EvidenceId),
}

/// M2-02 で固定する typed graph edge。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edge {
    Motivates {
        intent: IntentId,
        claim: ClaimId,
    },
    ConstrainedBy {
        claim: ClaimId,
        assumption: AssumptionId,
    },
    TestedBy {
        claim: ClaimId,
        contract: ContractId,
    },
    Supports {
        observation: EvidenceId,
        claim: ClaimId,
    },
    Contradicts {
        observation: EvidenceId,
        claim: ClaimId,
    },
    Evaluates {
        review: ReviewId,
        subject: ReviewSubject,
    },
    Invalidates {
        change: ChangeId,
        subject: InvalidationSubject,
    },
}

impl Edge {
    pub const fn relation(&self) -> &'static str {
        match self {
            Self::Motivates { .. } => "motivates",
            Self::ConstrainedBy { .. } => "constrained-by",
            Self::TestedBy { .. } => "tested-by",
            Self::Supports { .. } => "supports",
            Self::Contradicts { .. } => "contradicts",
            Self::Evaluates { .. } => "evaluates",
            Self::Invalidates { .. } => "invalidates",
        }
    }
}

/// evidence graph への登録と edge closure に関するエラー。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GraphError {
    #[error("evidence の required field が不正です: {source}")]
    InvalidEvidence {
        #[source]
        source: EvidenceValidationError,
    },
    #[error("graph node ID が重複しています: {duplicate:?}")]
    DuplicateNode { duplicate: StableId },
    #[error("edge が参照する graph node ID がありません: {id:?}")]
    MissingNode { id: StableId },
    #[error("evidence ID が重複しています: {id:?}")]
    DuplicateEvidence { id: EvidenceId },
    #[error("edge が参照する evidence ID が graph にありません: {id:?}")]
    MissingEvidence { id: EvidenceId },
}

/// evidence と ordered edge を保持する最小 graph。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EvidenceGraph {
    evidence: Vec<Evidence>,
    edges: Vec<Edge>,
}

impl EvidenceGraph {
    pub fn add_evidence(&mut self, evidence: Evidence) -> Result<(), GraphError> {
        evidence
            .validate_required_fields()
            .map_err(|source| GraphError::InvalidEvidence { source })?;
        if self
            .evidence
            .iter()
            .any(|entry| entry.id() == evidence.id())
        {
            return Err(GraphError::DuplicateEvidence {
                id: evidence.id().clone(),
            });
        }
        self.evidence.push(evidence);
        Ok(())
    }

    pub fn add_edge(&mut self, edge: Edge) -> Result<(), GraphError> {
        let referenced_evidence = match &edge {
            Edge::Supports { observation, .. } | Edge::Contradicts { observation, .. } => {
                Some(observation)
            }
            Edge::Evaluates {
                subject: ReviewSubject::Evidence(evidence),
                ..
            }
            | Edge::Invalidates {
                subject: InvalidationSubject::Evidence(evidence),
                ..
            } => Some(evidence),
            _ => None,
        };
        if let Some(id) = referenced_evidence
            && !self.evidence.iter().any(|entry| entry.id() == id)
        {
            return Err(GraphError::MissingEvidence { id: id.clone() });
        }
        self.edges.push(edge);
        Ok(())
    }

    pub fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
}
