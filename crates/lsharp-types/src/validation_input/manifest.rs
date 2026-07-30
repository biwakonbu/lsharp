//! version 1 intent graph manifest の JSON wire schema。
//!
//! 入力境界の serde 型と wire enum の変換を parse/closure 実装から分離する。
//! 型は親 module だけへ公開し、manifest の JSON shape を crate の公開 API にしない。

use serde::de::{Error as DeError, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::collections::BTreeMap;
use std::fmt;

pub(super) const SUPPORTED_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    pub(super) schema_version: u32,
    pub(super) nodes: Vec<NodeInput>,
    #[serde(default, deserialize_with = "deserialize_optional_review_registry")]
    pub(super) reviews: Option<Vec<ReviewInput>>,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_review_evidence_identity"
    )]
    pub(super) review_evidence_identity: Option<ReviewEvidenceIdentityInput>,
    pub(super) evidence: Vec<EvidenceInput>,
    pub(super) edges: Vec<EdgeInput>,
}

/// `reviews` は省略なら registry なし、配列なら明示 registry とする。
/// 明示された `null` は schema 上の配列ではないため、`Option` の通常の null→None
/// 変換に任せず入力エラーにする。
fn deserialize_optional_review_registry<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<ReviewInput>>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<Vec<ReviewInput>>::deserialize(deserializer)?
        .map(Some)
        .ok_or_else(|| D::Error::custom("reviews must be an array when present"))
}

/// `review_evidence_identity` は省略なら identity なし、存在する場合は object とする。
/// 明示された `null` は schema 上の object ではないため、identity なしへ黙って畳み込まない。
fn deserialize_optional_review_evidence_identity<'de, D>(
    deserializer: D,
) -> Result<Option<ReviewEvidenceIdentityInput>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<ReviewEvidenceIdentityInput>::deserialize(deserializer)?
        .map(Some)
        .ok_or_else(|| D::Error::custom("review_evidence_identity must be an object when present"))
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
pub(super) enum ReviewVisibilityInput {
    Public,
    Redacted,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ReviewVerificationStateInput {
    Verified,
    Unverified,
    Stale,
    Revoked,
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
pub(super) struct ReviewInput {
    pub(super) namespace: String,
    pub(super) key: String,
    pub(super) provenance_digest: String,
    pub(super) visibility: ReviewVisibilityInput,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_review_verification_state"
    )]
    pub(super) verification_state: Option<ReviewVerificationStateInput>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReviewEvidenceIdentityInput {
    pub(super) subject_digest: String,
    pub(super) source_commit: String,
    pub(super) artifact_digest: String,
    pub(super) trust_store_digest: RequiredNullableString,
    pub(super) lifecycle_digest: RequiredNullableString,
    pub(super) now: String,
}

/// identity object では nullable field も省略せず、`null` を明示させる。
#[derive(Debug)]
pub(super) struct RequiredNullableString(pub(super) Option<String>);

impl<'de> Deserialize<'de> for RequiredNullableString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer).map(Self)
    }
}

fn deserialize_optional_review_verification_state<'de, D>(
    deserializer: D,
) -> Result<Option<ReviewVerificationStateInput>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<ReviewVerificationStateInput>::deserialize(deserializer)?
        .map(Some)
        .ok_or_else(|| D::Error::custom("verification_state must be a string when present"))
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
    pub(super) coverage: UniqueCoverage,
}

/// JSON object の duplicate key を最後の値で上書きせず、入力エラーとして保持する。
///
/// canonical `SamplingPlan` は `BTreeMap` で duplicate bucket を表現できないため、
/// map へ変換する前の serde visitor で wire-level duplicate を拒否する。
#[derive(Debug, Default)]
pub(super) struct UniqueCoverage(pub(super) BTreeMap<String, usize>);

impl<'de> Deserialize<'de> for UniqueCoverage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct UniqueCoverageVisitor;

        impl<'de> Visitor<'de> for UniqueCoverageVisitor {
            type Value = UniqueCoverage;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object with unique coverage bucket keys")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut coverage = BTreeMap::new();
                while let Some((bucket, count)) = map.next_entry::<String, usize>()? {
                    if coverage.contains_key(&bucket) {
                        return Err(A::Error::custom(format!(
                            "duplicate coverage bucket key: {bucket:?}"
                        )));
                    }
                    coverage.insert(bucket, count);
                }
                Ok(UniqueCoverage(coverage))
            }
        }

        deserializer.deserialize_map(UniqueCoverageVisitor)
    }
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
enum EdgeInputWire {
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

/// edge variant は internally tagged enum のため、serde の通常の enum derive だけでは
/// payload の未知 field を無視してしまう。入力 manifest は versioned wire contract なので、
/// variant ごとの許可 field を先に検査して fail-closed にする。
#[derive(Debug)]
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

impl<'de> Deserialize<'de> for EdgeInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EdgeInputVisitor;

        impl<'de> Visitor<'de> for EdgeInputVisitor {
            type Value = EdgeInput;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an intent graph edge object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut fields = BTreeMap::new();
                while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                    if fields.insert(key.clone(), value).is_some() {
                        return Err(A::Error::custom(format!("duplicate edge field: {key:?}")));
                    }
                }

                let relation = fields
                    .get("relation")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| A::Error::custom("edge relation is required"))?;
                let allowed = match relation {
                    "motivates" => &["relation", "intent", "claim"][..],
                    "constrained-by" => &["relation", "claim", "assumption"][..],
                    "tested-by" => &["relation", "claim", "contract"][..],
                    "supports" | "contradicts" => &["relation", "observation", "claim"][..],
                    "evaluates" => &["relation", "review", "subject"][..],
                    "invalidates" => &["relation", "change", "subject"][..],
                    _ => &["relation"][..],
                };
                if let Some(unexpected) = fields.keys().find(|key| !allowed.contains(&key.as_str()))
                {
                    return Err(A::Error::custom(format!(
                        "unknown field in {relation} edge: {unexpected:?}"
                    )));
                }

                let value = serde_json::Value::Object(fields.into_iter().collect());
                let wire =
                    serde_json::from_value::<EdgeInputWire>(value).map_err(A::Error::custom)?;
                Ok(wire.into())
            }
        }

        deserializer.deserialize_map(EdgeInputVisitor)
    }
}

impl From<EdgeInputWire> for EdgeInput {
    fn from(value: EdgeInputWire) -> Self {
        match value {
            EdgeInputWire::Motivates { intent, claim } => Self::Motivates { intent, claim },
            EdgeInputWire::ConstrainedBy { claim, assumption } => {
                Self::ConstrainedBy { claim, assumption }
            }
            EdgeInputWire::TestedBy { claim, contract } => Self::TestedBy { claim, contract },
            EdgeInputWire::Supports { observation, claim } => Self::Supports { observation, claim },
            EdgeInputWire::Contradicts { observation, claim } => {
                Self::Contradicts { observation, claim }
            }
            EdgeInputWire::Evaluates { review, subject } => Self::Evaluates { review, subject },
            EdgeInputWire::Invalidates { change, subject } => Self::Invalidates { change, subject },
        }
    }
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

impl From<ReviewVisibilityInput> for crate::evidence::ReviewVisibility {
    fn from(value: ReviewVisibilityInput) -> Self {
        match value {
            ReviewVisibilityInput::Public => Self::Public,
            ReviewVisibilityInput::Redacted => Self::Redacted,
        }
    }
}

impl From<ReviewVerificationStateInput>
    for crate::intent::review_attestation::ReviewVerificationState
{
    fn from(value: ReviewVerificationStateInput) -> Self {
        match value {
            ReviewVerificationStateInput::Verified => Self::Verified,
            ReviewVerificationStateInput::Unverified => Self::Unverified,
            ReviewVerificationStateInput::Stale => Self::Stale,
            ReviewVerificationStateInput::Revoked => Self::Revoked,
        }
    }
}
