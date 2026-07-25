//! v0.2 M2 の intent/evidence graph manifest 出力境界。
//!
//! 入力側の version 1 JSON manifest と同じ wire shape を、順序を保った
//! deterministic な JSON として出力する。graph の判定結果は別の report であり、
//! この manifest に `verified` のような policy shortcut は追加しない。

use crate::evidence::{
    Edge, Evidence, EvidenceMethod, EvidenceOutcome, EvidenceSubject, Independence,
    InvalidationSubject, ReviewSubject,
};
use crate::intent::{IntentNode, StableId};
use crate::validation::IntentGraph;
use serde::Serialize;

const SCHEMA_VERSION: u32 = 1;

/// graph を version 1 manifest JSON に変換する。
pub fn to_manifest_json_string(graph: &IntentGraph) -> serde_json::Result<String> {
    serde_json::to_string(&ManifestWire::from_graph(graph))
}

/// graph を version 1 manifest の JSON value に変換する。
pub fn to_manifest_json_value(graph: &IntentGraph) -> serde_json::Value {
    serde_json::to_value(ManifestWire::from_graph(graph))
        .expect("intent graph manifest の wire shape は serializable")
}

impl IntentGraph {
    /// graph を version 1 manifest JSON に変換する。
    pub fn to_manifest_json_string(&self) -> serde_json::Result<String> {
        to_manifest_json_string(self)
    }

    /// graph を version 1 manifest の JSON value に変換する。
    pub fn to_manifest_json_value(&self) -> serde_json::Value {
        to_manifest_json_value(self)
    }
}

#[derive(Debug, Serialize)]
struct ManifestWire<'a> {
    schema_version: u32,
    nodes: Vec<NodeWire<'a>>,
    evidence: Vec<EvidenceWire<'a>>,
    edges: Vec<EdgeWire<'a>>,
}

impl<'a> ManifestWire<'a> {
    fn from_graph(graph: &'a IntentGraph) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            nodes: graph.nodes().iter().map(NodeWire::from_node).collect(),
            evidence: graph
                .evidence()
                .iter()
                .map(EvidenceWire::from_evidence)
                .collect(),
            edges: graph.edges().iter().map(EdgeWire::from_edge).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct IdWire<'a> {
    namespace: &'a str,
    key: &'a str,
}

impl<'a> From<&'a StableId> for IdWire<'a> {
    fn from(id: &'a StableId) -> Self {
        Self {
            namespace: id.namespace(),
            key: id.key(),
        }
    }
}

#[derive(Debug, Serialize)]
struct SubjectWire<'a> {
    kind: &'static str,
    namespace: &'a str,
    key: &'a str,
}

impl<'a> SubjectWire<'a> {
    fn from_id(kind: &'static str, id: &'a StableId) -> Self {
        Self {
            kind,
            namespace: id.namespace(),
            key: id.key(),
        }
    }
}

#[derive(Debug, Serialize)]
struct SpanWire {
    start: usize,
    end: usize,
}

#[derive(Debug, Serialize)]
struct NodeWire<'a> {
    kind: &'static str,
    namespace: &'a str,
    key: &'a str,
    text: &'a str,
    span: SpanWire,
}

impl<'a> NodeWire<'a> {
    fn from_node(node: &'a IntentNode) -> Self {
        let id = node.stable_id();
        let span = node.source_span();
        Self {
            kind: node.kind().as_str(),
            namespace: id.namespace(),
            key: id.key(),
            text: node.text(),
            span: SpanWire {
                start: span.start,
                end: span.end,
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct EvidenceWire<'a> {
    namespace: &'a str,
    key: &'a str,
    method: &'static str,
    subject: SubjectWire<'a>,
    outcome: &'static str,
    execution: ExecutionWire<'a>,
    provenance: ProvenanceWire<'a>,
    independence: &'static str,
}

impl<'a> EvidenceWire<'a> {
    fn from_evidence(evidence: &'a Evidence) -> Self {
        let id = evidence.id();
        Self {
            namespace: id.namespace(),
            key: id.key(),
            method: evidence_method(evidence.method()),
            subject: evidence_subject(evidence.subject()),
            outcome: evidence_outcome(evidence.outcome()),
            execution: ExecutionWire {
                runner: evidence.execution().runner(),
                target: evidence.execution().target(),
                source_commit: evidence.execution().source_commit(),
                artifact_digest: evidence.execution().artifact_digest(),
                sampling: SamplingWire {
                    cases: evidence.execution().cases(),
                    seed: evidence.execution().seed(),
                    generator: evidence.execution().generator(),
                    shrinks: evidence.execution().shrinks(),
                    coverage: evidence.execution().coverage(),
                },
            },
            provenance: ProvenanceWire {
                producer: evidence.provenance().producer(),
                tool_version: evidence.provenance().tool_version(),
                timestamp: evidence.provenance().timestamp(),
            },
            independence: independence(evidence.independence()),
        }
    }
}

#[derive(Debug, Serialize)]
struct ExecutionWire<'a> {
    runner: &'a str,
    target: &'a str,
    source_commit: &'a str,
    artifact_digest: &'a str,
    sampling: SamplingWire<'a>,
}

#[derive(Debug, Serialize)]
struct SamplingWire<'a> {
    cases: usize,
    seed: u64,
    generator: &'a str,
    shrinks: &'a [u64],
    coverage: &'a std::collections::BTreeMap<String, usize>,
}

#[derive(Debug, Serialize)]
struct ProvenanceWire<'a> {
    producer: &'a str,
    tool_version: &'a str,
    timestamp: &'a str,
}

fn evidence_method(method: EvidenceMethod) -> &'static str {
    match method {
        EvidenceMethod::Example => "example",
        EvidenceMethod::Case => "case",
        EvidenceMethod::Assert => "assert",
        EvidenceMethod::Property => "property",
        EvidenceMethod::Production => "production",
        EvidenceMethod::Reference => "reference",
        EvidenceMethod::Proof => "proof",
        EvidenceMethod::Review => "review",
    }
}

fn evidence_outcome(outcome: EvidenceOutcome) -> &'static str {
    match outcome {
        EvidenceOutcome::Pass => "pass",
        EvidenceOutcome::Fail => "fail",
        EvidenceOutcome::Contradicted => "contradicted",
        EvidenceOutcome::Unknown => "unknown",
        EvidenceOutcome::Stale => "stale",
    }
}

fn independence(value: Independence) -> &'static str {
    match value {
        Independence::SameAuthor => "same-author",
        Independence::IndependentReview => "independent-review",
        Independence::ExternalObservation => "external-observation",
    }
}

fn evidence_subject(subject: &EvidenceSubject) -> SubjectWire<'_> {
    match subject {
        EvidenceSubject::Intent(id) => SubjectWire::from_id("intent", id.stable_id()),
        EvidenceSubject::Claim(id) => SubjectWire::from_id("claim", id.stable_id()),
        EvidenceSubject::Contract(id) => SubjectWire::from_id("contract", id.stable_id()),
    }
}

fn review_subject(subject: &ReviewSubject) -> SubjectWire<'_> {
    match subject {
        ReviewSubject::Intent(id) => SubjectWire::from_id("intent", id.stable_id()),
        ReviewSubject::Claim(id) => SubjectWire::from_id("claim", id.stable_id()),
        ReviewSubject::Evidence(id) => SubjectWire::from_id("evidence", id.stable_id()),
    }
}

fn invalidation_subject(subject: &InvalidationSubject) -> SubjectWire<'_> {
    match subject {
        InvalidationSubject::Review(id) => SubjectWire::from_id("review", id.stable_id()),
        InvalidationSubject::Evidence(id) => SubjectWire::from_id("evidence", id.stable_id()),
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "relation")]
enum EdgeWire<'a> {
    #[serde(rename = "motivates")]
    Motivates {
        intent: IdWire<'a>,
        claim: IdWire<'a>,
    },
    #[serde(rename = "constrained-by")]
    ConstrainedBy {
        claim: IdWire<'a>,
        assumption: IdWire<'a>,
    },
    #[serde(rename = "tested-by")]
    TestedBy {
        claim: IdWire<'a>,
        contract: IdWire<'a>,
    },
    #[serde(rename = "supports")]
    Supports {
        observation: IdWire<'a>,
        claim: IdWire<'a>,
    },
    #[serde(rename = "contradicts")]
    Contradicts {
        observation: IdWire<'a>,
        claim: IdWire<'a>,
    },
    #[serde(rename = "evaluates")]
    Evaluates {
        review: IdWire<'a>,
        subject: SubjectWire<'a>,
    },
    #[serde(rename = "invalidates")]
    Invalidates {
        change: IdWire<'a>,
        subject: SubjectWire<'a>,
    },
}

impl<'a> EdgeWire<'a> {
    fn from_edge(edge: &'a Edge) -> Self {
        match edge {
            Edge::Motivates { intent, claim } => Self::Motivates {
                intent: intent.stable_id().into(),
                claim: claim.stable_id().into(),
            },
            Edge::ConstrainedBy { claim, assumption } => Self::ConstrainedBy {
                claim: claim.stable_id().into(),
                assumption: assumption.stable_id().into(),
            },
            Edge::TestedBy { claim, contract } => Self::TestedBy {
                claim: claim.stable_id().into(),
                contract: contract.stable_id().into(),
            },
            Edge::Supports { observation, claim } => Self::Supports {
                observation: observation.stable_id().into(),
                claim: claim.stable_id().into(),
            },
            Edge::Contradicts { observation, claim } => Self::Contradicts {
                observation: observation.stable_id().into(),
                claim: claim.stable_id().into(),
            },
            Edge::Evaluates { review, subject } => Self::Evaluates {
                review: review.stable_id().into(),
                subject: review_subject(subject),
            },
            Edge::Invalidates { change, subject } => Self::Invalidates {
                change: change.stable_id().into(),
                subject: invalidation_subject(subject),
            },
        }
    }
}
