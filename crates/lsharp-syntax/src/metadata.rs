use crate::ast::{Expr, TypeExpr};
use crate::span::Span;

/// source 上の順序と範囲を保持した metadata form。
///
/// 既存の [`crate::ast::Metadata`] field は互換 projection として残し、
/// canonical contract IR への変換はこの lossless form を入力にする。
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataForm {
    span: Span,
    pub kind: MetadataFormKind,
}

impl MetadataForm {
    pub fn new(span: Span, kind: MetadataFormKind) -> Self {
        Self { span, kind }
    }

    /// directive の `:` から payload 末尾までの source span。
    pub fn span(&self) -> Span {
        self.span
    }
}

/// source から lossless に受け取る evidence record の required fields。
///
/// 文字列 enum は source と canonical model の間で勝手に補正せず、types adapter が
/// typed value へ変換するときに検証する。sampling の shrinks/coverage は optional field
/// として保持し、省略時は空の deterministic plan へ投影する。
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceForm {
    id: String,
    subject: String,
    method: String,
    outcome: String,
    runner: String,
    target: String,
    source_commit: String,
    artifact_digest: String,
    cases: usize,
    seed: u64,
    generator: String,
    shrinks: Vec<u64>,
    coverage: Vec<(String, usize)>,
    producer: String,
    tool_version: String,
    timestamp: String,
    independence: String,
}

impl EvidenceForm {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: String,
        subject: String,
        method: String,
        outcome: String,
        runner: String,
        target: String,
        source_commit: String,
        artifact_digest: String,
        cases: usize,
        seed: u64,
        generator: String,
        shrinks: Vec<u64>,
        coverage: Vec<(String, usize)>,
        producer: String,
        tool_version: String,
        timestamp: String,
        independence: String,
    ) -> Self {
        Self {
            id,
            subject,
            method,
            outcome,
            runner,
            target,
            source_commit,
            artifact_digest,
            cases,
            seed,
            generator,
            shrinks,
            coverage,
            producer,
            tool_version,
            timestamp,
            independence,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn method(&self) -> &str {
        &self.method
    }

    pub fn outcome(&self) -> &str {
        &self.outcome
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

    pub fn coverage(&self) -> &[(String, usize)] {
        &self.coverage
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

    pub fn independence(&self) -> &str {
        &self.independence
    }
}

/// canonical `:case` 内の `(expect actual expected)` entry。
#[derive(Debug, Clone, PartialEq)]
pub struct CaseExpectation {
    source_span: Span,
    actual: Expr,
    expected: Expr,
}

impl CaseExpectation {
    pub(crate) fn new(source_span: Span, actual: Expr, expected: Expr) -> Self {
        Self {
            source_span,
            actual,
            expected,
        }
    }

    pub fn source_span(&self) -> Span {
        self.source_span
    }

    pub fn actual(&self) -> &Expr {
        &self.actual
    }

    pub fn expected(&self) -> &Expr {
        &self.expected
    }
}

/// canonical `:property` の binder。
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyBinder {
    source_span: Span,
    name: String,
    ty: TypeExpr,
}

impl PropertyBinder {
    pub(crate) fn new(source_span: Span, name: String, ty: TypeExpr) -> Self {
        Self {
            source_span,
            name,
            ty,
        }
    }

    pub fn source_span(&self) -> Span {
        self.source_span
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ty(&self) -> &TypeExpr {
        &self.ty
    }
}

/// `:property` 内の `(for-all ...)` declaration。
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyForm {
    source_span: Span,
    binders: Vec<PropertyBinder>,
    preconditions: Vec<Expr>,
    postcondition: Expr,
    cases: Option<usize>,
    seed: Option<u64>,
    shrink: Option<bool>,
}

impl PropertyForm {
    pub(crate) fn new(
        source_span: Span,
        binders: Vec<PropertyBinder>,
        preconditions: Vec<Expr>,
        postcondition: Expr,
        cases: Option<usize>,
        seed: Option<u64>,
        shrink: Option<bool>,
    ) -> Self {
        Self {
            source_span,
            binders,
            preconditions,
            postcondition,
            cases,
            seed,
            shrink,
        }
    }

    pub fn source_span(&self) -> Span {
        self.source_span
    }

    pub fn binders(&self) -> &[PropertyBinder] {
        &self.binders
    }

    pub fn preconditions(&self) -> &[Expr] {
        &self.preconditions
    }

    pub fn postcondition(&self) -> &Expr {
        &self.postcondition
    }

    pub fn cases(&self) -> Option<usize> {
        self.cases
    }

    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    pub fn shrink(&self) -> Option<bool> {
        self.shrink
    }
}

/// source から lossless に保持する contract form。
#[derive(Debug, Clone, PartialEq)]
pub enum MetadataFormKind {
    /// source から明示した intent node。ID は `intent:namespace/key` wire 形式。
    Intent { id: String, text: String },
    /// source から明示した claim node。ID は `claim:namespace/key` wire 形式。
    Claim { id: String, text: String },
    /// source から明示した assumption node。ID は `assumption:namespace/key` wire 形式。
    Assumption { id: String, text: String },
    /// source から明示した open-question node。ID は `open-question:namespace/key` wire 形式。
    OpenQuestion { id: String, text: String },
    /// source から opaque review registry として明示した review。
    ///
    /// 本文や reviewer の個人情報は保持せず、外部 provenance digest と公開範囲だけを
    /// canonical review registry へ投影する。
    Review {
        id: String,
        provenance_digest: String,
        visibility: String,
    },
    /// source から intent と claim を接続する typed edge。
    Motivates { intent: String, claim: String },
    /// source から claim と assumption を接続する typed edge。
    ConstrainedBy { claim: String, assumption: String },
    /// source から claim と executable contract を接続する typed edge。
    TestedBy { claim: String, contract: String },
    /// source から observation evidence と claim を接続する typed edge。
    Supports { observation: String, claim: String },
    /// source から contradictory observation evidence と claim を接続する typed edge。
    Contradicts { observation: String, claim: String },
    /// source から review と intent/claim/evidence subject を接続する typed edge。
    Evaluates { review: String, subject: String },
    /// source から change と review/evidence subject を接続する typed edge。
    Invalidates { change: String, subject: String },
    /// source から required provenance 付き evidence record を登録する。
    Evidence { record: Box<EvidenceForm> },
    /// legacy `:example [expr ...]`。一つの directive 内の grouping を維持する。
    LegacyExample { expressions: Vec<Expr> },
    /// legacy `:invariant predicate`。
    LegacyInvariant { predicate: Expr },
    /// canonical `:case [(expect actual expected) ...]`。
    Case { expectations: Vec<CaseExpectation> },
    /// canonical `:assert [predicate ...]`。一つの directive 内の grouping を維持する。
    Assertion { predicates: Vec<Expr> },
    /// canonical `:property [(for-all ...)]`。declaration grouping を維持する。
    Property { properties: Vec<PropertyForm> },
}
