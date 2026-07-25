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
        Err(SourceGraphError::Graph(_))
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
fn source_only_metadata_does_not_create_an_empty_contract_suite() {
    let program =
        parse(r#"(defn cancel [] :intent "intent:checkout/safe-cancel" "Users can cancel" true)"#)
            .expect("source intent fixture は parse できるべき");

    let suites =
        inventory_contract_suites(&program).expect("source metadata は inventory できるべき");
    assert!(suites.is_empty());
}
