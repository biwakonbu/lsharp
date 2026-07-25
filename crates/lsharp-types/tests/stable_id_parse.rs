use lsharp_types::intent::{
    AssumptionId, ChangeId, ClaimId, ContractId, EvidenceId, IntentId, NodeKind, OpenQuestionId,
    ReviewId, StableId, StableIdError,
};

#[test]
fn stable_id_wire_values_round_trip_without_losing_parts() {
    let cases = [
        ("intent:checkout/safe-cancel", NodeKind::Intent),
        ("claim:checkout/cancel-rejects-shipped", NodeKind::Claim),
        (
            "assumption:checkout/state-authoritative",
            NodeKind::Assumption,
        ),
        (
            "open-question:checkout/cancel-after-label",
            NodeKind::OpenQuestion,
        ),
        ("contract:checkout/cancel-case", NodeKind::Contract),
        ("evidence:checkout/cancel-case-001", NodeKind::Evidence),
        ("review:checkout/independent-001", NodeKind::Review),
        ("change:checkout/api-v2", NodeKind::Change),
    ];

    for (wire, kind) in cases {
        let parsed = StableId::parse(wire).expect("valid stable ID wire value");
        assert_eq!(parsed.as_str(), wire);
        assert_eq!(parsed.kind(), kind);
        assert_eq!(
            parsed.namespace(),
            wire.split(':').nth(1).unwrap().split('/').next().unwrap()
        );
        assert_eq!(StableId::parse(parsed.as_str()).unwrap(), parsed);
    }
}

#[test]
fn typed_ids_parse_only_their_declared_node_kind() {
    assert_eq!(
        IntentId::parse("intent:checkout/safe-cancel")
            .unwrap()
            .as_str(),
        "intent:checkout/safe-cancel"
    );
    assert_eq!(
        ClaimId::parse("claim:checkout/cancel-rejects-shipped")
            .unwrap()
            .as_str(),
        "claim:checkout/cancel-rejects-shipped"
    );
    assert_eq!(
        AssumptionId::parse("assumption:checkout/state-authoritative")
            .unwrap()
            .kind(),
        NodeKind::Assumption
    );
    assert_eq!(
        OpenQuestionId::parse("open-question:checkout/state")
            .unwrap()
            .kind(),
        NodeKind::OpenQuestion
    );
    assert_eq!(
        ContractId::parse("contract:checkout/cancel-case")
            .unwrap()
            .kind(),
        NodeKind::Contract
    );
    assert_eq!(
        EvidenceId::parse("evidence:checkout/case-001")
            .unwrap()
            .kind(),
        NodeKind::Evidence
    );
    assert_eq!(
        ReviewId::parse("review:checkout/independent-001")
            .unwrap()
            .kind(),
        NodeKind::Review
    );
    assert_eq!(
        ChangeId::parse("change:checkout/api-v2").unwrap().kind(),
        NodeKind::Change
    );
    assert!(matches!(
        IntentId::parse("claim:checkout/safe-cancel"),
        Err(StableIdError::UnexpectedKind { .. })
    ));
    assert!(matches!(
        AssumptionId::parse("open-question:checkout/state"),
        Err(StableIdError::UnexpectedKind { .. })
    ));
}

#[test]
fn stable_id_parser_rejects_unknown_kind_and_malformed_wire_values() {
    for wire in [
        "unknown:checkout/value",
        "intent:checkout",
        "intent/checkout/value",
        "intent:checkout/value/extra",
        "intent:checkout/value:extra",
        ":checkout/value",
        "intent:/value",
        "intent:checkout/",
        "intent:checkout/value with spaces",
    ] {
        assert!(
            matches!(
                StableId::parse(wire),
                Err(StableIdError::InvalidWireFormat { .. })
                    | Err(StableIdError::InvalidSegment { .. })
            ),
            "malformed stable ID should fail closed: {wire}"
        );
    }
}
