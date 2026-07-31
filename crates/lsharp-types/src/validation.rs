//! M2-03 intent validation の純粋な判定 model。
//!
//! parser/manifest の入力と CLI の text/JSON projection は後続 surface とし、ここでは
//! graph の observable facts を report へ写像する。`Unknown` は欠落を成功扱いにせず、
//! `Fail` は contradiction が観測された場合に限定する。

use crate::evidence::{
    Edge, Evidence, EvidenceGraph, EvidenceMethod, EvidenceOutcome, GraphError,
    InvalidationSubject, ReviewRecord, ReviewSubject,
};
use crate::intent::{
    ClaimId, EvidenceId, IntentId, IntentNode, ReviewId, StableId,
    review_attestation::{
        encode_signature_base64url, ReviewAttestation, ReviewVerificationState,
    },
};
pub(crate) use crate::validation_identity::ReviewEvidenceIdentityWire;
pub use crate::validation_identity::{
    ReviewEvidenceIdentity, ReviewEvidenceIdentityError, ReviewEvidenceIdentityProjectionError,
};
use serde::Serialize;
use lsharp_syntax::span::Span;

/// node の trace が欠けている箇所。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceGap {
    IntentWithoutClaim { intent: IntentId },
    ClaimWithoutTest { claim: ClaimId },
}

impl TraceGap {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::IntentWithoutClaim { .. } => "trace-gap.intent-without-claim",
            Self::ClaimWithoutTest { .. } => "trace-gap.claim-without-test",
        }
    }
}

/// invalidation を反映した stale subject の deterministic な projection。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StaleSubjects {
    stale_reviews: Vec<ReviewId>,
    stale_evidence: Vec<EvidenceId>,
}

impl StaleSubjects {
    pub fn reviews(&self) -> &[ReviewId] {
        &self.stale_reviews
    }

    pub fn evidence(&self) -> &[EvidenceId] {
        &self.stale_evidence
    }

    fn push_review(&mut self, id: ReviewId) {
        if !self.stale_reviews.contains(&id) {
            self.stale_reviews.push(id);
        }
    }

    fn push_evidence(&mut self, id: EvidenceId) {
        if !self.stale_evidence.contains(&id) {
            self.stale_evidence.push(id);
        }
    }
}

/// intent validation の assurance status。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationStatus {
    Pass,
    Fail,
    Unknown,
}

impl ValidationStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Unknown => "unknown",
        }
    }
}

/// canonical attestation verifier が返した review state の report fact。
///
/// verifier の署名/lifecycle/clock 入力はこの型の外側で解決し、report は明示された
/// state だけを deterministic に投影する。`Invalid` は入力診断として report へ入れない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewVerificationFact {
    review_id: ReviewId,
    state: ReviewVerificationState,
}

impl ReviewVerificationFact {
    pub fn new(
        review_id: ReviewId,
        state: ReviewVerificationState,
    ) -> Result<Self, ReviewVerificationProjectionError> {
        if state == ReviewVerificationState::Invalid {
            return Err(ReviewVerificationProjectionError::InvalidState {
                id: review_id,
                state,
            });
        }
        Ok(Self { review_id, state })
    }

    pub fn review_id(&self) -> &ReviewId {
        &self.review_id
    }

    pub const fn state(&self) -> ReviewVerificationState {
        self.state
    }
}

/// source の named-field attestation を report へ投影する deterministic record。
///
/// verifier が解決した state、source directive span、署名対象の canonical bytes を同じ
/// JSON object に束ね、Rust/selfhost/native の producer が同一 fixture を比較できるようにする。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewAttestationProjection {
    review_id: String,
    subject_digest: String,
    source_commit: String,
    provenance_digest: String,
    provider: String,
    key_id: String,
    algorithm: String,
    signature: String,
    issued_at: String,
    expires_at: Option<String>,
    sequence: u64,
    state: ReviewVerificationState,
    canonical_bytes: Vec<u8>,
    span: Span,
}

impl ReviewAttestationProjection {
    pub fn new(
        attestation: &ReviewAttestation,
        state: ReviewVerificationState,
        span: Span,
    ) -> Self {
        Self {
            review_id: attestation.review_id().as_str().to_string(),
            subject_digest: attestation.subject_digest().to_string(),
            source_commit: attestation.source_commit().to_string(),
            provenance_digest: attestation.provenance_digest().to_string(),
            provider: attestation.provider().to_string(),
            key_id: attestation.key_id().to_string(),
            algorithm: attestation.algorithm().as_str().to_string(),
            signature: encode_signature_base64url(attestation.signature()),
            issued_at: attestation.issued_at().to_string(),
            expires_at: attestation.expires_at().map(ToOwned::to_owned),
            sequence: attestation.sequence(),
            state,
            canonical_bytes: attestation.canonical_bytes(),
            span,
        }
    }

    pub fn review_id(&self) -> &str {
        &self.review_id
    }
}

/// verification fact を report へ投影できない理由。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReviewVerificationProjectionError {
    #[error("review verification state が invalid です: id={id:?}, state={state:?}")]
    InvalidState {
        id: ReviewId,
        state: ReviewVerificationState,
    },
    #[error("review verification fact が重複しています: id={id:?}")]
    DuplicateReview { id: ReviewId },
}

/// `validate` が返す fact-oriented report。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    status: ValidationStatus,
    trace_gaps: Vec<TraceGap>,
    open_questions: usize,
    independent_reviews: usize,
    contradicting_observations: usize,
    stale_reviews: usize,
    stale_evidence: usize,
    review_evidence_identity: Option<ReviewEvidenceIdentity>,
    review_verifications: Option<Vec<ReviewVerificationFact>>,
    review_attestations: Option<Vec<ReviewAttestationProjection>>,
}

impl ValidationReport {
    pub fn status(&self) -> ValidationStatus {
        self.status
    }

    pub fn trace_gaps(&self) -> &[TraceGap] {
        &self.trace_gaps
    }

    pub fn open_questions(&self) -> usize {
        self.open_questions
    }

    pub fn independent_reviews(&self) -> usize {
        self.independent_reviews
    }

    pub fn contradicting_observations(&self) -> usize {
        self.contradicting_observations
    }

    pub fn stale_reviews(&self) -> usize {
        self.stale_reviews
    }

    pub fn stale_evidence(&self) -> usize {
        self.stale_evidence
    }

    pub fn review_verifications(&self) -> Option<&[ReviewVerificationFact]> {
        self.review_verifications.as_deref()
    }

    pub fn review_evidence_identity(&self) -> Option<&ReviewEvidenceIdentity> {
        self.review_evidence_identity.as_ref()
    }

    /// source-owned attestation の named fields/canonical bytes/span を report に追加する。
    pub fn with_review_attestations(
        mut self,
        attestations: Vec<ReviewAttestationProjection>,
    ) -> Self {
        if !attestations.is_empty() {
            self.review_attestations = Some(attestations);
        }
        self
    }

    /// explicit review input identity を report に追加する。
    pub fn with_review_evidence_identity(mut self, identity: ReviewEvidenceIdentity) -> Self {
        self.review_evidence_identity = Some(identity);
        self
    }

    /// planned `lsharp validate --format json` projection。
    ///
    /// `verified` は意図的に含めず、consumer が conformance と intent validation の
    /// policy を別々に決められるようにする。
    pub fn to_json_string(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.to_wire())
    }

    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self.to_wire())
            .expect("validation report の wire shape は serializable")
    }

    /// planned `lsharp validate` の deterministic text projection。
    ///
    /// JSON と同じ事実だけを固定順で出力し、policy 用の `verified` shortcut は持たない。
    pub fn to_text(&self) -> String {
        use std::fmt::Write;

        let mut text = String::new();
        writeln!(&mut text, "status: {}", self.status.as_str()).expect("String は write 可能");
        for gap in &self.trace_gaps {
            writeln!(&mut text, "{}: {}", gap.code(), gap.subject_id())
                .expect("String は write 可能");
        }
        writeln!(&mut text, "open-questions: {}", self.open_questions)
            .expect("String は write 可能");
        writeln!(
            &mut text,
            "independent-reviews: {}",
            self.independent_reviews
        )
        .expect("String は write 可能");
        writeln!(
            &mut text,
            "contradicting-observations: {}",
            self.contradicting_observations
        )
        .expect("String は write 可能");
        writeln!(&mut text, "stale-reviews: {}", self.stale_reviews).expect("String は write 可能");
        writeln!(&mut text, "stale-evidence: {}", self.stale_evidence)
            .expect("String は write 可能");
        if let Some(identity) = &self.review_evidence_identity {
            writeln!(
                &mut text,
                "review-evidence-identity: subject={} source={} artifact={} trust-store={} lifecycle={} now={}",
                identity.subject_digest(),
                identity.source_commit(),
                identity.artifact_digest(),
                identity.trust_store_digest().unwrap_or("-"),
                identity.lifecycle_digest().unwrap_or("-"),
                identity.now(),
            )
            .expect("String は write 可能");
        }
        if let Some(verifications) = &self.review_verifications {
            for verification in verifications {
                writeln!(
                    &mut text,
                    "review-verification: {}={}",
                    verification.review_id().as_str(),
                    verification.state().as_str()
                )
                .expect("String は write 可能");
            }
        }
        text
    }

    fn to_wire(&self) -> ValidationReportWire {
        ValidationReportWire {
            status: self.status.as_str(),
            trace_gaps: self.trace_gaps.iter().map(TraceGapWire::from_gap).collect(),
            open_questions: self.open_questions,
            independent_reviews: self.independent_reviews,
            contradicting_observations: self.contradicting_observations,
            stale_reviews: self.stale_reviews,
            stale_evidence: self.stale_evidence,
            review_evidence_identity: self
                .review_evidence_identity
                .as_ref()
                .map(ReviewEvidenceIdentityWire::from_identity),
            review_verifications: self.review_verifications.as_ref().map(|verifications| {
                verifications
                    .iter()
                    .map(ReviewVerificationWire::from_fact)
                    .collect()
            }),
            review_attestations: self.review_attestations.as_ref().map(|attestations| {
                attestations
                    .iter()
                    .map(ReviewAttestationProjectionWire::from_projection)
                    .collect()
            }),
        }
    }
}

#[derive(Debug, Serialize)]
struct ValidationReportWire {
    status: &'static str,
    trace_gaps: Vec<TraceGapWire>,
    open_questions: usize,
    independent_reviews: usize,
    contradicting_observations: usize,
    stale_reviews: usize,
    stale_evidence: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    review_evidence_identity: Option<ReviewEvidenceIdentityWire>,
    #[serde(skip_serializing_if = "Option::is_none")]
    review_verifications: Option<Vec<ReviewVerificationWire>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    review_attestations: Option<Vec<ReviewAttestationProjectionWire>>,
}

#[derive(Debug, Serialize)]
struct TraceGapWire {
    code: &'static str,
    subject_id: String,
}

#[derive(Debug, Serialize)]
struct ReviewVerificationWire {
    review_id: String,
    state: &'static str,
}

impl ReviewVerificationWire {
    fn from_fact(fact: &ReviewVerificationFact) -> Self {
        Self {
            review_id: fact.review_id().as_str().to_string(),
            state: fact.state().as_str(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ReviewAttestationProjectionWire {
    review_id: String,
    subject_digest: String,
    source_commit: String,
    provenance_digest: String,
    provider: String,
    key_id: String,
    algorithm: String,
    signature: String,
    issued_at: String,
    expires_at: Option<String>,
    sequence: u64,
    state: &'static str,
    canonical_bytes: Vec<u8>,
    span: SourceSpanWire,
}

#[derive(Debug, Serialize)]
struct SourceSpanWire {
    start: usize,
    end: usize,
}

impl ReviewAttestationProjectionWire {
    fn from_projection(projection: &ReviewAttestationProjection) -> Self {
        Self {
            review_id: projection.review_id.clone(),
            subject_digest: projection.subject_digest.clone(),
            source_commit: projection.source_commit.clone(),
            provenance_digest: projection.provenance_digest.clone(),
            provider: projection.provider.clone(),
            key_id: projection.key_id.clone(),
            algorithm: projection.algorithm.clone(),
            signature: projection.signature.clone(),
            issued_at: projection.issued_at.clone(),
            expires_at: projection.expires_at.clone(),
            sequence: projection.sequence,
            state: projection.state.as_str(),
            canonical_bytes: projection.canonical_bytes.clone(),
            span: SourceSpanWire {
                start: projection.span.start,
                end: projection.span.end,
            },
        }
    }
}

impl TraceGapWire {
    fn from_gap(gap: &TraceGap) -> Self {
        match gap {
            TraceGap::IntentWithoutClaim { intent } => Self {
                code: gap.code(),
                subject_id: intent.as_str().to_string(),
            },
            TraceGap::ClaimWithoutTest { claim } => Self {
                code: gap.code(),
                subject_id: claim.as_str().to_string(),
            },
        }
    }
}

impl TraceGap {
    fn subject_id(&self) -> &str {
        match self {
            Self::IntentWithoutClaim { intent } => intent.as_str(),
            Self::ClaimWithoutTest { claim } => claim.as_str(),
        }
    }
}

/// intent node と evidence graph を束ねた M2 validation input。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IntentGraph {
    nodes: Vec<IntentNode>,
    evidence: EvidenceGraph,
    reviews: Vec<ReviewRecord>,
    review_registry_explicit: bool,
    review_evidence_identity: Option<ReviewEvidenceIdentity>,
}

impl IntentGraph {
    pub fn add_node(&mut self, node: IntentNode) -> Result<(), GraphError> {
        if self
            .nodes
            .iter()
            .any(|existing| existing.stable_id() == node.stable_id())
        {
            return Err(GraphError::DuplicateNode {
                duplicate: node.stable_id().clone(),
            });
        }
        self.nodes.push(node);
        Ok(())
    }

    pub fn add_evidence(&mut self, evidence: Evidence) -> Result<(), GraphError> {
        self.evidence.add_evidence(evidence)
    }

    /// 明示された review registry record を graph へ追加する。
    pub fn add_review(&mut self, review: ReviewRecord) -> Result<(), GraphError> {
        review
            .validate_required_fields()
            .map_err(|source| GraphError::InvalidReview { source })?;
        if self
            .reviews
            .iter()
            .any(|existing| existing.id() == review.id())
        {
            return Err(GraphError::DuplicateReview {
                id: review.id().clone(),
            });
        }
        self.reviews.push(review);
        self.review_registry_explicit = true;
        Ok(())
    }

    /// manifest/source input が review registry を明示したことを記録する。
    pub(crate) fn mark_review_registry_explicit(&mut self) {
        self.review_registry_explicit = true;
    }

    pub(crate) fn review_registry_is_explicit(&self) -> bool {
        self.review_registry_explicit
    }

    pub fn review_evidence_identity(&self) -> Option<&ReviewEvidenceIdentity> {
        self.review_evidence_identity.as_ref()
    }

    /// manifest と caller が渡した evidence identity の競合を黙って上書きしない。
    pub fn attach_review_evidence_identity(
        &mut self,
        identity: ReviewEvidenceIdentity,
    ) -> Result<(), ReviewEvidenceIdentityProjectionError> {
        match &self.review_evidence_identity {
            None => {
                self.review_evidence_identity = Some(identity);
                Ok(())
            }
            Some(existing) if existing == &identity => Ok(()),
            Some(_) => Err(ReviewEvidenceIdentityProjectionError::Conflict),
        }
    }

    pub fn add_edge(&mut self, edge: Edge) -> Result<(), GraphError> {
        self.validate_edge_node_endpoints(&edge)?;
        self.evidence.add_edge(edge)
    }

    pub fn nodes(&self) -> &[IntentNode] {
        &self.nodes
    }

    pub fn evidence(&self) -> &[Evidence] {
        self.evidence.evidence()
    }

    pub fn edges(&self) -> &[Edge] {
        self.evidence.edges()
    }

    pub fn reviews(&self) -> &[ReviewRecord] {
        &self.reviews
    }

    /// 既知の stale outcome と invalidation edge を review/evidence へ投影する。
    ///
    /// review の stale は、その review が `evaluates` する evidence だけへ伝播する。
    /// Intent/Claim は node subject のため対象にせず、複数の宣言は追加順を保って重複を除く。
    pub fn stale_subjects(&self) -> StaleSubjects {
        let mut stale = StaleSubjects::default();

        for evidence in self.evidence() {
            if evidence.outcome() == EvidenceOutcome::Stale {
                stale.push_evidence(evidence.id().clone());
            }
        }
        for edge in self.edges() {
            if let Edge::Invalidates { subject, .. } = edge {
                match subject {
                    InvalidationSubject::Review(review) => stale.push_review(review.clone()),
                    InvalidationSubject::Evidence(evidence) => {
                        stale.push_evidence(evidence.clone())
                    }
                }
            }
        }

        let mut review_index = 0;
        while review_index < stale.stale_reviews.len() {
            let review = stale.stale_reviews[review_index].clone();
            for edge in self.edges() {
                if let Edge::Evaluates {
                    review: linked,
                    subject: ReviewSubject::Evidence(evidence),
                } = edge
                    && linked == &review
                {
                    stale.push_evidence(evidence.clone());
                }
            }
            review_index += 1;
        }

        stale
    }

    pub fn validate(&self) -> ValidationReport {
        validate_graph(self, None)
    }

    /// 明示された attestation verification state を report へ投影する。
    ///
    /// fact は review ID 順に並べ替え、重複や `invalid` を fail-closed に拒否する。
    /// 既存の `validate()` は opaque M2 review と後方互換のため state を暗黙に補わない。
    pub fn validate_with_review_verifications(
        &self,
        verifications: &[ReviewVerificationFact],
    ) -> Result<ValidationReport, ReviewVerificationProjectionError> {
        let verifications = normalize_review_verifications(verifications)?;
        Ok(validate_graph(self, Some(&verifications)))
    }

    /// explicit verification fact を registry record へ付与する。
    ///
    /// report にだけ現れる外部 review ID は保持し、manifest へ投影できる同名 record
    /// だけを更新する。入力順は report と同じ canonical sort/dedup policy で検証する。
    pub fn attach_review_verifications(
        &mut self,
        verifications: &[ReviewVerificationFact],
    ) -> Result<(), ReviewVerificationProjectionError> {
        let verifications = normalize_review_verifications(verifications)?;
        for fact in verifications {
            let Some(review) = self
                .reviews
                .iter_mut()
                .find(|review| review.id() == fact.review_id())
            else {
                continue;
            };
            review.set_verification_state(fact.state()).map_err(|_| {
                ReviewVerificationProjectionError::InvalidState {
                    id: fact.review_id().clone(),
                    state: fact.state(),
                }
            })?;
        }
        Ok(())
    }

    fn validate_edge_node_endpoints(&self, edge: &Edge) -> Result<(), GraphError> {
        match edge {
            Edge::Motivates { intent, claim } => {
                self.require_node(intent.stable_id())?;
                self.require_node(claim.stable_id())?;
            }
            Edge::ConstrainedBy { claim, assumption } => {
                self.require_node(claim.stable_id())?;
                self.require_node(assumption.stable_id())?;
            }
            Edge::TestedBy { claim, .. }
            | Edge::Supports { claim, .. }
            | Edge::Contradicts { claim, .. } => {
                self.require_node(claim.stable_id())?;
            }
            Edge::Evaluates { review, subject } => {
                self.require_review_if_registry_is_explicit(review)?;
                match subject {
                    crate::evidence::ReviewSubject::Intent(intent) => {
                        self.require_node(intent.stable_id())?;
                    }
                    crate::evidence::ReviewSubject::Claim(claim) => {
                        self.require_node(claim.stable_id())?;
                    }
                    crate::evidence::ReviewSubject::Evidence(_) => {}
                }
            }
            Edge::Invalidates { subject, .. } => {
                if let crate::evidence::InvalidationSubject::Review(review) = subject {
                    self.require_review_if_registry_is_explicit(review)?;
                }
            }
        }
        Ok(())
    }

    fn require_review_if_registry_is_explicit(
        &self,
        id: &crate::intent::ReviewId,
    ) -> Result<(), GraphError> {
        if !self.review_registry_explicit || self.reviews.iter().any(|review| review.id() == id) {
            Ok(())
        } else {
            Err(GraphError::MissingReview { id: id.clone() })
        }
    }

    fn require_node(&self, id: &StableId) -> Result<(), GraphError> {
        if self.nodes.iter().any(|node| node.stable_id() == id) {
            Ok(())
        } else {
            Err(GraphError::MissingNode { id: id.clone() })
        }
    }
}

fn normalize_review_verifications(
    verifications: &[ReviewVerificationFact],
) -> Result<Vec<ReviewVerificationFact>, ReviewVerificationProjectionError> {
    let mut verifications = verifications.to_vec();
    verifications.sort_by(|left, right| left.review_id().as_str().cmp(right.review_id().as_str()));
    for pair in verifications.windows(2) {
        if pair[0].review_id() == pair[1].review_id() {
            return Err(ReviewVerificationProjectionError::DuplicateReview {
                id: pair[1].review_id().clone(),
            });
        }
    }
    Ok(verifications)
}

fn validate_graph(
    graph: &IntentGraph,
    review_verifications: Option<&[ReviewVerificationFact]>,
) -> ValidationReport {
    let mut trace_gaps = Vec::new();
    let edges = graph.edges();
    for node in graph.nodes() {
        match node {
            IntentNode::Intent(intent) => {
                let linked = edges.iter().any(|edge| {
                    matches!(edge, Edge::Motivates { intent: linked, .. } if linked == intent.id())
                });
                if !linked {
                    trace_gaps.push(TraceGap::IntentWithoutClaim {
                        intent: intent.id().clone(),
                    });
                }
            }
            IntentNode::Claim(claim) => {
                let linked = edges.iter().any(|edge| {
                    matches!(edge, Edge::TestedBy { claim: linked, .. } if linked == claim.id())
                });
                if !linked {
                    trace_gaps.push(TraceGap::ClaimWithoutTest {
                        claim: claim.id().clone(),
                    });
                }
            }
            IntentNode::Assumption(_) | IntentNode::OpenQuestion(_) => {}
        }
    }

    let open_questions = graph
        .nodes()
        .iter()
        .filter(|node| matches!(node, IntentNode::OpenQuestion(_)))
        .count();
    let independent_reviews = graph
        .evidence()
        .iter()
        .filter(|evidence| {
            evidence.method() == EvidenceMethod::Review
                && evidence.outcome() == EvidenceOutcome::Pass
                && evidence.independence() == crate::evidence::Independence::IndependentReview
        })
        .filter(|evidence| match review_verifications {
            None => true,
            Some(verifications) => graph.edges().iter().any(|edge| {
                let Edge::Evaluates {
                    review,
                    subject: ReviewSubject::Evidence(subject),
                } = edge
                else {
                    return false;
                };
                subject == evidence.id()
                    && verifications.iter().any(|fact| {
                        fact.review_id() == review
                            && fact.state() == ReviewVerificationState::Verified
                    })
            }),
        })
        .count();
    let contradictory_ids = contradictory_evidence_ids(graph);
    let stale_subjects = graph.stale_subjects();
    let status = if !contradictory_ids.is_empty() {
        ValidationStatus::Fail
    } else if !trace_gaps.is_empty()
        || open_questions > 0
        || independent_reviews == 0
        || review_verifications.is_some_and(|verifications| {
            verifications
                .iter()
                .any(|fact| fact.state() != ReviewVerificationState::Verified)
        })
        || !stale_subjects.reviews().is_empty()
        || !stale_subjects.evidence().is_empty()
    {
        ValidationStatus::Unknown
    } else {
        ValidationStatus::Pass
    };

    ValidationReport {
        status,
        trace_gaps,
        open_questions,
        independent_reviews,
        contradicting_observations: contradictory_ids.len(),
        stale_reviews: stale_subjects.reviews().len(),
        stale_evidence: stale_subjects.evidence().len(),
        review_evidence_identity: graph.review_evidence_identity.clone(),
        review_verifications: review_verifications.map(ToOwned::to_owned),
        review_attestations: None,
    }
}

fn contradictory_evidence_ids(graph: &IntentGraph) -> Vec<crate::intent::EvidenceId> {
    let mut ids = Vec::new();
    for evidence in graph.evidence() {
        if evidence.outcome() == EvidenceOutcome::Contradicted {
            ids.push(evidence.id().clone());
        }
    }
    for edge in graph.edges() {
        if let Edge::Contradicts { observation, .. } = edge
            && !ids.iter().any(|id| id == observation)
        {
            ids.push(observation.clone());
        }
    }
    ids
}
