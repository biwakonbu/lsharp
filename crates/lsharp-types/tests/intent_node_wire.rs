use lsharp_syntax::span::Span;
use lsharp_types::intent::{IntentNode, IntentNodeError, NodeKind, StableIdError};

#[test]
fn wire_parts_build_each_supported_intent_node_kind() {
    let span = Span::new(12, 34);
    let cases = [
        (
            "intent:checkout/safe-cancel",
            NodeKind::Intent,
            "Users can cancel before shipment",
        ),
        (
            "claim:checkout/cancel-rejects-shipped",
            NodeKind::Claim,
            "The API rejects shipped orders",
        ),
        (
            "assumption:checkout/state-authoritative",
            NodeKind::Assumption,
            "Shipment state is authoritative",
        ),
        (
            "open-question:checkout/cancel-after-label",
            NodeKind::OpenQuestion,
            "Can cancellation happen after a label is printed?",
        ),
    ];

    for (wire, kind, text) in cases {
        let node = IntentNode::from_wire_parts(wire, text, span).expect("wire node is valid");
        assert_eq!(node.kind(), kind);
        assert_eq!(node.stable_id().as_str(), wire);
        assert_eq!(node.text(), text);
        assert_eq!(node.source_span(), span);
    }
}

#[test]
fn wire_parts_reject_graph_only_kinds_before_node_construction() {
    for wire in [
        "contract:checkout/cancel-case",
        "evidence:checkout/cancel-case-001",
        "review:checkout/independent-001",
        "change:checkout/api-v2",
    ] {
        assert!(matches!(
            IntentNode::from_wire_parts(wire, "not an AST node", Span::dummy()),
            Err(IntentNodeError::UnsupportedKind { .. })
        ));
    }
}

#[test]
fn wire_parts_preserve_fail_closed_id_and_text_errors() {
    assert!(matches!(
        IntentNode::from_wire_parts("unknown:checkout/value", "text", Span::dummy()),
        Err(IntentNodeError::StableId(
            StableIdError::InvalidWireFormat { .. }
        ))
    ));
    assert!(matches!(
        IntentNode::from_wire_parts("intent:checkout/value", "   ", Span::dummy()),
        Err(IntentNodeError::NodeText(_))
    ));
}
