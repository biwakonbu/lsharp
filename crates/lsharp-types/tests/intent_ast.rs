use lsharp_syntax::span::Span;
use lsharp_types::intent::{
    Assumption, AssumptionId, Claim, ClaimId, Intent, IntentId, IntentNode, NodeKind, OpenQuestion,
    OpenQuestionId, StableIdError,
};

#[test]
fn stable_ids_use_explicit_kind_namespace_and_key() {
    let intent = IntentId::new("checkout", "payment-success").expect("valid intent id");
    let claim = ClaimId::new("checkout", "payment-success").expect("valid claim id");

    assert_eq!(intent.as_str(), "intent:checkout/payment-success");
    assert_eq!(claim.as_str(), "claim:checkout/payment-success");
    assert_eq!(intent.kind(), NodeKind::Intent);
    assert_ne!(intent.as_str(), claim.as_str());
}

#[test]
fn stable_ids_are_independent_of_source_span_and_node_order() {
    let first = Intent::new(
        IntentId::new("orders", "cancelled-is-terminal").expect("valid intent id"),
        "Cancellation is terminal",
        Span::new(10, 40),
    )
    .expect("valid intent");
    let moved = Intent::new(
        IntentId::new("orders", "cancelled-is-terminal").expect("valid intent id"),
        "Cancellation is terminal",
        Span::new(900, 930),
    )
    .expect("valid intent");

    assert_eq!(first.id(), moved.id());
    assert_ne!(first.source_span(), moved.source_span());
}

#[test]
fn intent_ast_preserves_typed_nodes_and_source_spans() {
    let span = Span::new(3, 19);
    let nodes = [
        IntentNode::Intent(
            Intent::new(
                IntentId::new("orders", "safe-cancel").expect("valid id"),
                "Users can cancel an order before shipment",
                span,
            )
            .expect("valid intent"),
        ),
        IntentNode::Claim(
            Claim::new(
                ClaimId::new("orders", "cancel-api-rejects-shipped").expect("valid id"),
                "The cancel API rejects shipped orders",
                span,
            )
            .expect("valid claim"),
        ),
        IntentNode::Assumption(
            Assumption::new(
                AssumptionId::new("orders", "shipment-state-authoritative").expect("valid id"),
                "Shipment state is authoritative",
                span,
            )
            .expect("valid assumption"),
        ),
        IntentNode::OpenQuestion(
            OpenQuestion::new(
                OpenQuestionId::new("orders", "cancel-after-label").expect("valid id"),
                "Can cancellation happen after a label is printed?",
                span,
            )
            .expect("valid question"),
        ),
    ];

    assert_eq!(nodes[0].kind(), NodeKind::Intent);
    assert_eq!(nodes[1].kind(), NodeKind::Claim);
    assert_eq!(nodes[2].kind(), NodeKind::Assumption);
    assert_eq!(nodes[3].kind(), NodeKind::OpenQuestion);
    assert!(nodes.iter().all(|node| node.source_span() == span));
    assert_eq!(nodes[0].text(), "Users can cancel an order before shipment");
}

#[test]
fn stable_id_rejects_empty_or_non_segment_parts() {
    for (namespace, key) in [("", "valid"), ("orders", ""), ("orders/paid", "valid")] {
        assert!(matches!(
            IntentId::new(namespace, key),
            Err(StableIdError::InvalidSegment { .. })
        ));
    }
}
