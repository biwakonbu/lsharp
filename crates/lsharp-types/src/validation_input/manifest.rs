//! version 1 intent graph manifest の JSON wire schema。
//!
//! 入力境界の serde 型と wire enum の変換を parse/closure 実装から分離する。
//! 型は親 module だけへ公開し、manifest の JSON shape を crate の公開 API にしない。

use serde::Deserialize;
use std::collections::BTreeMap;

pub(super) const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub(super) schema_version: u32,
    pub(super) nodes: Vec<NodeInput>,
    pub(super) evidence: Vec<EvidenceInput>,
    pub(super) edges: Vec<EdgeInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum NodeKindInput {
    Intent,
    Claim,
    Assumption,
    OpenQuestion,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NodeInput {
    pub(super) kind: NodeKindInput,
    pub(super) namespace: String,
    pub(super) key: String,
    pub(super) text: String,
    #[serde(default)]
    pub(super) span: SpanInput,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SpanInput {
    pub(super) start: usize,
    pub(super) end: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum EvidenceMethodInput {
    Example,
    Case,
    Assert,
    Property,
    Production,
    Reference,
    Proof,
    Review,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum EvidenceOutcomeInput {
    Pass,
    Fail,
    Contradicted,
    Unknown,
    Stale,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum IndependenceInput {
    SameAuthor,
    IndependentReview,
    ExternalObservation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum SubjectKindInput {
    Intent,
    Claim,
    Contract,
    Evidence,
    Review,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct IdInput {
    pub(super) namespace: String,
    pub(super) key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SubjectInput {
    pub(super) kind: SubjectKindInput,
    pub(super) namespace: String,
    pub(super) key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EvidenceInput {
    pub(super) namespace: String,
    pub(super) key: String,
    pub(super) method: EvidenceMethodInput,
    pub(super) subject: SubjectInput,
    pub(super) outcome: EvidenceOutcomeInput,
    pub(super) execution: ExecutionInput,
    pub(super) provenance: ProvenanceInput,
    pub(super) independence: IndependenceInput,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ExecutionInput {
    pub(super) runner: String,
    pub(super) target: String,
    pub(super) source_commit: String,
    pub(super) artifact_digest: String,
    pub(super) sampling: SamplingInput,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SamplingInput {
    pub(super) cases: usize,
    pub(super) seed: u64,
    pub(super) generator: String,
    #[serde(default)]
    pub(super) shrinks: Vec<u64>,
    #[serde(default)]
    pub(super) coverage: BTreeMap<String, usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProvenanceInput {
    pub(super) producer: String,
    pub(super) tool_version: String,
    pub(super) timestamp: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "relation", rename_all = "kebab-case")]
pub(super) enum EdgeInput {
    Motivates {
        intent: IdInput,
        claim: IdInput,
    },
    ConstrainedBy {
        claim: IdInput,
        assumption: IdInput,
    },
    TestedBy {
        claim: IdInput,
        contract: IdInput,
    },
    Supports {
        observation: IdInput,
        claim: IdInput,
    },
    Contradicts {
        observation: IdInput,
        claim: IdInput,
    },
    Evaluates {
        review: IdInput,
        subject: SubjectInput,
    },
    Invalidates {
        change: IdInput,
        subject: SubjectInput,
    },
}

impl SubjectKindInput {
    pub(super) const fn as_str(&self) -> &'static str {
        match self {
            Self::Intent => "intent",
            Self::Claim => "claim",
            Self::Contract => "contract",
            Self::Evidence => "evidence",
            Self::Review => "review",
        }
    }
}

impl From<EvidenceMethodInput> for crate::evidence::EvidenceMethod {
    fn from(value: EvidenceMethodInput) -> Self {
        match value {
            EvidenceMethodInput::Example => Self::Example,
            EvidenceMethodInput::Case => Self::Case,
            EvidenceMethodInput::Assert => Self::Assert,
            EvidenceMethodInput::Property => Self::Property,
            EvidenceMethodInput::Production => Self::Production,
            EvidenceMethodInput::Reference => Self::Reference,
            EvidenceMethodInput::Proof => Self::Proof,
            EvidenceMethodInput::Review => Self::Review,
        }
    }
}

impl From<EvidenceOutcomeInput> for crate::evidence::EvidenceOutcome {
    fn from(value: EvidenceOutcomeInput) -> Self {
        match value {
            EvidenceOutcomeInput::Pass => Self::Pass,
            EvidenceOutcomeInput::Fail => Self::Fail,
            EvidenceOutcomeInput::Contradicted => Self::Contradicted,
            EvidenceOutcomeInput::Unknown => Self::Unknown,
            EvidenceOutcomeInput::Stale => Self::Stale,
        }
    }
}

impl From<IndependenceInput> for crate::evidence::Independence {
    fn from(value: IndependenceInput) -> Self {
        match value {
            IndependenceInput::SameAuthor => Self::SameAuthor,
            IndependenceInput::IndependentReview => Self::IndependentReview,
            IndependenceInput::ExternalObservation => Self::ExternalObservation,
        }
    }
}
