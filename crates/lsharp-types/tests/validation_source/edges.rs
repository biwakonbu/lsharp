//! intent graph の node-owned / evidence-owned edge closure を検証する contract tests。

use lsharp_syntax::parse;
use lsharp_types::validation_source::{SourceGraphError, source_program_to_intent_graph};

#[test]
fn source_adapter_registers_node_edges_after_collecting_all_declarations() {
    let program = parse(
        r#"
        (module Checkout
          (defn cancel []
            :motivates "intent:checkout/safe-cancel" "claim:checkout/cancel-rejects-shipped"
            :constrained-by "claim:checkout/cancel-rejects-shipped" "assumption:checkout/state-authoritative"
            true)
          (defn intent []
            :intent "intent:checkout/safe-cancel" "Users can cancel an order"
            :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
            :assumption "assumption:checkout/state-authoritative" "Shipment state is authoritative"
            true))
        "#,
    )
    .expect("source edge fixture は parse できるべき");

    let graph =
        source_program_to_intent_graph(&program).expect("source edge graph が構築できるべき");
    assert_eq!(graph.edges().len(), 2);
    assert!(matches!(
        &graph.edges()[0],
        lsharp_types::evidence::Edge::Motivates { intent, claim }
            if intent.as_str() == "intent:checkout/safe-cancel"
                && claim.as_str() == "claim:checkout/cancel-rejects-shipped"
    ));
    assert!(matches!(
        &graph.edges()[1],
        lsharp_types::evidence::Edge::ConstrainedBy { claim, assumption }
            if claim.as_str() == "claim:checkout/cancel-rejects-shipped"
                && assumption.as_str() == "assumption:checkout/state-authoritative"
    ));
}

#[test]
fn source_adapter_rejects_orphan_and_mismatched_edge_endpoints() {
    let orphan = parse(
        r#"(defn cancel [] :motivates "intent:checkout/missing" "claim:checkout/cancel" true)"#,
    )
    .expect("orphan fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&orphan),
        Err(SourceGraphError::MissingNodeReference {
            relation: "motivates.intent",
            ..
        })
    ));

    let mismatch = parse(
        r#"
        (defn cancel []
          :intent "intent:checkout/safe-cancel" "Users can cancel"
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :motivates "claim:checkout/cancel" "intent:checkout/safe-cancel"
          true)
        "#,
    )
    .expect("mismatch fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&mismatch),
        Err(SourceGraphError::EdgeIdAt { .. })
    ));
}

#[test]
fn source_adapter_reports_orphan_edge_with_directive_span() {
    const SOURCE: &str =
        r#"(defn cancel [] :motivates "intent:checkout/missing" "claim:checkout/cancel" true)"#;
    let program = parse(SOURCE).expect("orphan span fixture は parse できるべき");
    let error = source_program_to_intent_graph(&program)
        .expect_err("orphan edge は directive span 付きで拒否するべき");
    let SourceGraphError::MissingNodeReference { relation, span, .. } = error else {
        panic!("orphan edge の source diagnostic を期待しました: {error:?}");
    };

    assert_eq!(relation, "motivates.intent");
    assert!(span.start < span.end);
    assert!(SOURCE[span.start..span.end].starts_with(":motivates"));
}

#[test]
fn source_adapter_reports_malformed_edge_id_with_directive_span() {
    const SOURCE: &str =
        r#"(defn cancel [] :motivates "intent:checkout" "claim:checkout/cancel" true)"#;
    let program = parse(SOURCE).expect("malformed edge ID fixture は parse できるべき");
    let error = source_program_to_intent_graph(&program)
        .expect_err("malformed edge ID は directive span 付きで拒否するべき");
    let SourceGraphError::EdgeIdAt { relation, span, .. } = error else {
        panic!("malformed edge ID の source diagnostic を期待しました: {error:?}");
    };

    assert_eq!(relation, "motivates.intent");
    assert!(span.start < span.end);
    assert!(SOURCE[span.start..span.end].starts_with(":motivates"));
}

#[test]
fn source_adapter_registers_tested_by_claim_contract_edges() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :tested-by "claim:checkout/cancel-rejects-shipped" "contract:checkout/cancel-case"
          true)
        "#,
    )
    .expect("tested-by source fixture は parse できるべき");

    let graph =
        source_program_to_intent_graph(&program).expect("tested-by source graph が構築できるべき");
    assert!(matches!(
        &graph.edges()[0],
        lsharp_types::evidence::Edge::TestedBy { claim, contract }
            if claim.as_str() == "claim:checkout/cancel-rejects-shipped"
                && contract.as_str() == "contract:checkout/cancel-case"
    ));
}

#[test]
fn source_adapter_rejects_orphan_or_mismatched_tested_by_claims() {
    let orphan = parse(
        r#"(defn cancel [] :tested-by "claim:checkout/missing" "contract:checkout/case" true)"#,
    )
    .expect("orphan tested-by fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&orphan),
        Err(SourceGraphError::MissingNodeReference {
            relation: "tested-by.claim",
            ..
        })
    ));

    let mismatch = parse(
        r#"(defn cancel [] :tested-by "intent:checkout/wrong-kind" "contract:checkout/case" true)"#,
    )
    .expect("kind mismatch tested-by fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&mismatch),
        Err(SourceGraphError::EdgeIdAt { .. })
    ));
}

#[test]
fn source_adapter_rejects_evidence_edges_without_registry_entries() {
    let supports = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :supports "evidence:checkout/cancel-observation" "claim:checkout/cancel-rejects-shipped"
          true)
        "#,
    )
    .expect("supports source fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&supports),
        Err(SourceGraphError::EvidenceRegistryRequired {
            relation: "supports",
            evidence_id,
            ..
        }) if evidence_id == "evidence:checkout/cancel-observation"
    ));

    let contradicts = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :contradicts "evidence:checkout/cancel-counterexample" "claim:checkout/cancel-rejects-shipped"
          true)
        "#,
    )
    .expect("contradicts source fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&contradicts),
        Err(SourceGraphError::EvidenceRegistryRequired {
            relation: "contradicts",
            evidence_id,
            ..
        }) if evidence_id == "evidence:checkout/cancel-counterexample"
    ));
}
