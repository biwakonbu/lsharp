use lsharp_syntax::parse;
use lsharp_types::metadata_contract::inventory_contract_suites;
use lsharp_types::validation_source::{SourceGraphError, source_program_to_intent_graph};

#[test]
fn source_adapter_registers_typed_nodes_without_deriving_ids_from_span_or_order() {
    let program = parse(
        r#"
        (module Checkout
          (defn cancel []
            :intent "intent:checkout/safe-cancel" "Users can cancel an order"
            :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
            true))
        "#,
    )
    .expect("source fixture は parse できるべき");

    let graph = source_program_to_intent_graph(&program).expect("source graph が構築できるべき");
    assert_eq!(
        graph
            .nodes()
            .iter()
            .map(|node| (node.stable_id().as_str(), node.text()))
            .collect::<Vec<_>>(),
        vec![
            ("intent:checkout/safe-cancel", "Users can cancel an order"),
            (
                "claim:checkout/cancel-rejects-shipped",
                "The API rejects shipped orders"
            ),
        ]
    );
    assert!(graph.nodes().iter().all(|node| node.source_span().end > 0));
}

#[test]
fn source_adapter_rejects_duplicate_ids_and_typed_kind_mismatch() {
    let duplicate = parse(
        r#"
        (defn first [] :intent "intent:checkout/same" "first" true)
        (defn second [] :intent "intent:checkout/same" "second" true)
        "#,
    )
    .expect("duplicate fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&duplicate),
        Err(SourceGraphError::DuplicateNode { .. })
    ));

    let mismatch = parse(r#"(defn cancel [] :claim "intent:checkout/wrong-kind" "claim" true)"#)
        .expect("kind mismatch fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&mismatch),
        Err(SourceGraphError::KindMismatch { .. })
    ));

    let empty = parse(r#"(defn cancel [] :intent "" "" true)"#)
        .expect("empty node fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&empty),
        Err(SourceGraphError::Node(_))
    ));
}

#[test]
fn source_adapter_reports_duplicate_node_with_both_source_spans() {
    const SOURCE: &str = r#"
        (module Checkout
          (private
            (defn first []
              :intent "intent:checkout/same" "first declaration"
              true))
          (impl (Show Int)
            (defn second []
              :intent "intent:checkout/same" "second declaration"
              true)))
        "#;
    let program = parse(SOURCE).expect("nested duplicate fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("duplicate source node は span 付きで拒否するべき");
    let SourceGraphError::DuplicateNode {
        id,
        first_span,
        duplicate_span,
    } = error
    else {
        panic!("duplicate node の source diagnostic を期待しました: {error:?}");
    };

    assert_eq!(id, "intent:checkout/same");
    assert!(first_span.start < duplicate_span.start);
    assert!(first_span.end <= duplicate_span.start);
    assert!(SOURCE[first_span.start..first_span.end].contains("first declaration"));
    assert!(SOURCE[duplicate_span.start..duplicate_span.end].contains("second declaration"));
}

#[test]
fn source_only_metadata_does_not_create_an_empty_contract_suite() {
    let program =
        parse(r#"(defn cancel [] :intent "intent:checkout/safe-cancel" "Users can cancel" true)"#)
            .expect("source intent fixture は parse できるべき");

    let suites =
        inventory_contract_suites(&program).expect("source metadata は inventory できるべき");
    assert!(suites.is_empty());
}

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
        Err(SourceGraphError::EdgeId(_))
    ));
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
fn source_adapter_registers_evidence_records_before_support_edges() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :evidence "evidence:checkout/cancel-observation"
            :subject "claim:checkout/cancel-rejects-shipped"
            :method "case"
            :outcome "pass"
            :runner "cargo-test"
            :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef"
            :artifact-digest "sha256:abc123"
            :cases 1
            :seed 42
            :generator "checkout-cancel-fixture"
            :producer "lsharp-test"
            :tool-version "0.2.0"
            :timestamp "2026-07-25T00:00:00Z"
            :independence "same-author"
          :supports "evidence:checkout/cancel-observation" "claim:checkout/cancel-rejects-shipped"
          true)
        "#,
    )
    .expect("evidence source fixture は parse できるべき");

    let graph = source_program_to_intent_graph(&program)
        .expect("evidence record と supports edge が構築できるべき");
    assert_eq!(graph.evidence().len(), 1);
    assert_eq!(
        graph.evidence()[0].id().as_str(),
        "evidence:checkout/cancel-observation"
    );
    assert!(matches!(
        graph.evidence()[0].subject(),
        lsharp_types::evidence::EvidenceSubject::Claim(claim)
            if claim.as_str() == "claim:checkout/cancel-rejects-shipped"
    ));
    assert!(matches!(
        &graph.edges()[0],
        lsharp_types::evidence::Edge::Supports { observation, claim }
            if observation.as_str() == "evidence:checkout/cancel-observation"
                && claim.as_str() == "claim:checkout/cancel-rejects-shipped"
    ));
}

#[test]
fn source_adapter_projects_optional_sampling_fields() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :evidence "evidence:checkout/cancel-observation"
            :subject "claim:checkout/cancel-rejects-shipped"
            :method "property"
            :outcome "pass"
            :runner "cargo-test"
            :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef"
            :artifact-digest "sha256:abc123"
            :cases 3
            :seed 42
            :generator "checkout-cancel-fixture"
            :shrinks [8 3 1]
            :coverage [("negative" 2) ("positive" 1)]
            :producer "lsharp-test"
            :tool-version "0.2.0"
            :timestamp "2026-07-25T00:00:00Z"
            :independence "same-author"
          true)
        "#,
    )
    .expect("optional sampling source fixture は parse できるべき");

    let graph = source_program_to_intent_graph(&program)
        .expect("optional sampling fields は canonical evidence に投影されるべき");
    let execution = graph.evidence()[0].execution();
    assert_eq!(execution.shrinks(), &[8, 3, 1]);
    assert_eq!(
        execution.coverage().get("negative"),
        Some(&2),
        "coverage は bucket 名を保持するべき"
    );
    assert_eq!(execution.coverage().get("positive"), Some(&1));

    let sampling = graph.to_manifest_json_value()["evidence"][0]["execution"]["sampling"].clone();
    assert_eq!(sampling["shrinks"], serde_json::json!([8, 3, 1]));
    assert_eq!(
        sampling["coverage"],
        serde_json::json!({"negative": 2, "positive": 1})
    );
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
        Err(SourceGraphError::EdgeId(_))
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
            evidence_id
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
            evidence_id
        }) if evidence_id == "evidence:checkout/cancel-counterexample"
    ));
}
