//! source metadata の node registry 投影を検証する contract tests。

use lsharp_syntax::parse;
use lsharp_types::evidence::ReviewVisibility;
use lsharp_types::intent::{NodeKind, StableIdError};
use lsharp_types::metadata_contract::inventory_contract_suites;
use lsharp_types::validation_source::{source_program_to_intent_graph, SourceGraphError};

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
fn source_adapter_preserves_every_graph_owned_node_kind() {
    let program = parse(
        r#"
        (defn checkout []
          :intent "intent:checkout/safe-cancel" "Users can cancel an order"
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :assumption "assumption:checkout/state" "Shipment state is authoritative"
          :open-question "open-question:checkout/label" "Can cancellation happen after a label?"
          true)
        "#,
    )
    .expect("all graph-owned node kind fixture は parse できるべき");

    let graph =
        source_program_to_intent_graph(&program).expect("all graph-owned node が投影できるべき");
    assert_eq!(
        graph
            .nodes()
            .iter()
            .map(|node| (node.kind(), node.stable_id().as_str(), node.text()))
            .collect::<Vec<_>>(),
        vec![
            (
                NodeKind::Intent,
                "intent:checkout/safe-cancel",
                "Users can cancel an order"
            ),
            (
                NodeKind::Claim,
                "claim:checkout/cancel",
                "The API rejects shipped orders"
            ),
            (
                NodeKind::Assumption,
                "assumption:checkout/state",
                "Shipment state is authoritative"
            ),
            (
                NodeKind::OpenQuestion,
                "open-question:checkout/label",
                "Can cancellation happen after a label?"
            ),
        ]
    );
}

#[test]
fn source_adapter_projects_type_definition_metadata_nodes() {
    let program = parse(
        r#"
        (type (Result e)
          (Ok Int)
          (Err e)
          :intent "intent:checkout/result" "The result models checkout completion"
          :claim "claim:checkout/result-total" "Every checkout returns a result")
        "#,
    )
    .expect("type definition metadata fixture は parse できるべき");

    let graph =
        source_program_to_intent_graph(&program).expect("type definition nodes が投影できるべき");
    assert_eq!(
        graph
            .nodes()
            .iter()
            .map(|node| (node.stable_id().as_str(), node.text()))
            .collect::<Vec<_>>(),
        vec![
            (
                "intent:checkout/result",
                "The result models checkout completion"
            ),
            (
                "claim:checkout/result-total",
                "Every checkout returns a result"
            ),
        ]
    );
    assert!(graph.nodes().iter().all(|node| node.source_span().end > 0));
}

#[test]
fn source_adapter_projects_record_definition_metadata_nodes() {
    let program = parse(
        r#"
        (type (Point a)
          (record
            (: x Int)
            (: y a))
          :intent "intent:geometry/point" "A point has two coordinates"
          :claim "claim:geometry/point-typed" "Each coordinate follows the declared type")
        "#,
    )
    .expect("record definition metadata fixture は parse できるべき");

    let graph =
        source_program_to_intent_graph(&program).expect("record definition nodes が投影できるべき");
    assert_eq!(
        graph
            .nodes()
            .iter()
            .map(|node| (node.stable_id().as_str(), node.text()))
            .collect::<Vec<_>>(),
        vec![
            ("intent:geometry/point", "A point has two coordinates"),
            (
                "claim:geometry/point-typed",
                "Each coordinate follows the declared type"
            ),
        ]
    );
    assert!(graph.nodes().iter().all(|node| node.source_span().end > 0));
}

#[test]
fn source_adapter_projects_nested_module_private_and_impl_metadata_in_declaration_order() {
    const SOURCE: &str = r#"
        (module Checkout
          (defn top []
            :intent "intent:checkout/top" "Top declaration"
            true)
          (private
            (defn hidden []
              :claim "claim:checkout/hidden" "Private declaration"
              true))
          (impl (Show Int)
            (defn render []
              :assumption "assumption:checkout/render" "Render is deterministic"
              :open-question "open-question:checkout/render" "Need external review"
              true)))
        "#;
    let program = parse(SOURCE).expect("nested declaration metadata fixture は parse できるべき");

    let graph = source_program_to_intent_graph(&program)
        .expect("nested module/private/impl metadata が投影できるべき");
    assert_eq!(
        graph
            .nodes()
            .iter()
            .map(|node| (node.stable_id().as_str(), node.text()))
            .collect::<Vec<_>>(),
        vec![
            ("intent:checkout/top", "Top declaration"),
            ("claim:checkout/hidden", "Private declaration"),
            ("assumption:checkout/render", "Render is deterministic"),
            ("open-question:checkout/render", "Need external review"),
        ]
    );

    let spans = graph
        .nodes()
        .iter()
        .map(|node| node.source_span())
        .collect::<Vec<_>>();
    assert!(spans.windows(2).all(|pair| pair[0].start < pair[1].start));
    assert!(SOURCE[spans[0].start..spans[0].end].contains("Top declaration"));
    assert!(SOURCE[spans[1].start..spans[1].end].contains("Private declaration"));
    assert!(SOURCE[spans[2].start..spans[2].end].contains("Render is deterministic"));
    assert!(SOURCE[spans[3].start..spans[3].end].contains("Need external review"));
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

    const SOURCE: &str = r#"(defn cancel [] :intent "" "" true)"#;
    let empty = parse(SOURCE).expect("empty node fixture は parse できるべき");
    let error = source_program_to_intent_graph(&empty)
        .expect_err("empty node text は source span 付きで拒否するべき");
    let SourceGraphError::InvalidNodeField { field, value, span } = error else {
        panic!("empty node text の source diagnostic を期待しました: {error:?}");
    };
    assert_eq!(field, "text");
    assert_eq!(value, "");
    assert!(span.start < span.end);
    assert!(SOURCE[span.start..span.end].contains(":intent"));
}

#[test]
fn source_adapter_reports_invalid_node_id_with_directive_span() {
    const SOURCE: &str = r#"(defn cancel [] :claim "claim:checkout/bad/key" "invalid ID" true)"#;
    let program = parse(SOURCE).expect("invalid node ID fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("invalid node ID は source span 付きで拒否するべき");
    let SourceGraphError::NodeIdAt { span, source } = error else {
        panic!("invalid node ID の source diagnostic を期待しました: {error:?}");
    };
    assert!(matches!(
        source,
        StableIdError::InvalidSegment {
            field: "key",
            value
        } if value == "bad/key"
    ));
    assert!(span.start < span.end);
    assert!(SOURCE[span.start..span.end].contains(":claim"));
}

#[test]
fn source_adapter_rejects_whitespace_only_node_text() {
    const SOURCE: &str = r#"(defn cancel [] :claim "claim:checkout/whitespace-text" "  " true)"#;
    let program = parse(SOURCE).expect("whitespace node text fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("whitespace node text は source span 付きで拒否するべき");
    let SourceGraphError::InvalidNodeField { field, value, span } = error else {
        panic!("whitespace node text の source diagnostic を期待しました: {error:?}");
    };
    assert_eq!(field, "text");
    assert_eq!(value, "  ");
    assert!(span.start < span.end);
    assert!(SOURCE[span.start..span.end].contains(":claim"));
}

#[test]
fn source_adapter_reports_empty_node_text_before_invalid_stable_id() {
    const SOURCE: &str = r#"(defn cancel [] :claim "claim:checkout/bad/key" "  " true)"#;
    let program =
        parse(SOURCE).expect("empty node text and invalid stable ID fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("empty node text は invalid stable ID より先に source span 付きで拒否するべき");
    let SourceGraphError::InvalidNodeField { field, value, span } = error else {
        panic!("empty node text の precedence diagnostic を期待しました: {error:?}");
    };
    assert_eq!(field, "text");
    assert_eq!(value, "  ");
    assert!(span.start < span.end);
    assert!(SOURCE[span.start..span.end].contains(":claim"));
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
fn source_adapter_projects_review_registry_forms_and_rejects_unknown_visibility() {
    let program = parse(
        r#"
        (defn checkout-review []
          :review "review:checkout/reviewer-001" "sha256:review-provenance-001" "redacted"
          true)
        "#,
    )
    .expect("source review registry fixture は parse できるべき");

    let graph = source_program_to_intent_graph(&program)
        .expect("source review registry が graph へ投影できるべき");
    assert_eq!(graph.reviews().len(), 1);
    assert_eq!(
        graph.reviews()[0].id().as_str(),
        "review:checkout/reviewer-001"
    );
    assert_eq!(
        graph.reviews()[0].provenance_digest(),
        "sha256:review-provenance-001"
    );
    assert_eq!(graph.reviews()[0].visibility(), ReviewVisibility::Redacted);

    let invalid = parse(
        r#"
        (defn checkout-review []
          :review "review:checkout/reviewer-001" "sha256:review-provenance-001" "private"
          true)
        "#,
    )
    .expect("invalid visibility fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&invalid),
        Err(SourceGraphError::InvalidReviewField {
            field: "visibility",
            value,
            ..
        }) if value == "private"
    ));
}

#[test]
fn source_adapter_rejects_empty_review_id_as_invalid_review_field() {
    let program = parse(
        r#"
        (defn checkout-review []
          :review "" "sha256:review-provenance-001" "public"
          true)
        "#,
    )
    .expect("empty review ID fixture は parse できるべき");

    assert!(matches!(
        source_program_to_intent_graph(&program),
        Err(SourceGraphError::InvalidReviewField {
            field: "id",
            value,
            ..
        }) if value.is_empty()
    ));
}

#[test]
fn source_adapter_reports_blank_review_digest_before_invalid_review_id() {
    let program = parse(
        r#"
        (defn checkout-review []
          :review "review:checkout" "  " "public"
          true)
        "#,
    )
    .expect("blank review digest and invalid ID fixture は parse できるべき");

    assert!(matches!(
        source_program_to_intent_graph(&program),
        Err(SourceGraphError::InvalidReviewField {
            field: "provenance_digest",
            value,
            ..
        }) if value == "  "
    ));
}

#[test]
fn source_adapter_reports_duplicate_reviews_with_both_source_spans() {
    const SOURCE: &str = r#"
        (defn first []
          :review "review:checkout/reviewer-001" "sha256:review-provenance-001" "public"
          true)
        (defn second []
          :review "review:checkout/reviewer-001" "sha256:review-provenance-002" "redacted"
          true)
        "#;
    let program = parse(SOURCE).expect("duplicate review fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("duplicate source review は span 付きで拒否するべき");
    let SourceGraphError::DuplicateReview {
        id,
        first_span,
        duplicate_span,
    } = error
    else {
        panic!("duplicate review の source diagnostic を期待しました: {error:?}");
    };

    assert_eq!(id, "review:checkout/reviewer-001");
    assert!(first_span.start < duplicate_span.start);
    assert!(first_span.end <= duplicate_span.start);
    assert!(SOURCE[first_span.start..first_span.end].contains("review-provenance-001"));
    assert!(SOURCE[duplicate_span.start..duplicate_span.end].contains("review-provenance-002"));
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
