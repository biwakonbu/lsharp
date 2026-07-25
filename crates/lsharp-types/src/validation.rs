//! M2-03 intent validation の純粋な判定 model。
//!
//! parser/manifest の入力と CLI の text/JSON projection は後続 surface とし、ここでは
//! graph の observable facts を report へ写像する。`Unknown` は欠落を成功扱いにせず、
//! `Fail` は contradiction が観測された場合に限定する。

use crate::evidence::{Edge, Evidence, EvidenceGraph, EvidenceMethod, EvidenceOutcome, GraphError};
use crate::intent::{ClaimId, IntentId, IntentNode};
use serde::Serialize;

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

/// `validate` が返す fact-oriented report。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    status: ValidationStatus,
    trace_gaps: Vec<TraceGap>,
    open_questions: usize,
    independent_reviews: usize,
    contradicting_observations: usize,
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
        text
    }

    fn to_wire(&self) -> ValidationReportWire {
        ValidationReportWire {
            status: self.status.as_str(),
            trace_gaps: self.trace_gaps.iter().map(TraceGapWire::from_gap).collect(),
            open_questions: self.open_questions,
            independent_reviews: self.independent_reviews,
            contradicting_observations: self.contradicting_observations,
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
}

#[derive(Debug, Serialize)]
struct TraceGapWire {
    code: &'static str,
    subject_id: String,
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

    pub fn add_edge(&mut self, edge: Edge) -> Result<(), GraphError> {
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

    pub fn validate(&self) -> ValidationReport {
        validate_graph(self)
    }
}

fn validate_graph(graph: &IntentGraph) -> ValidationReport {
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
                && evidence.independence() == crate::evidence::Independence::IndependentReview
        })
        .count();
    let contradictory_ids = contradictory_evidence_ids(graph);
    let status = if !contradictory_ids.is_empty() {
        ValidationStatus::Fail
    } else if !trace_gaps.is_empty() || open_questions > 0 || independent_reviews == 0 {
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
