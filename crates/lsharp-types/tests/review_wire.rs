use lsharp_types::intent::review_wire::{parse_review_wire, ReviewWireError};

const VALID_WIRE: &str = r#"
{
  "schema_version": 1,
  "attestations": [
    {
      "review_id": "review:orders/reviewer-001",
      "subject_digest": "sha256:graph-001",
      "source_commit": "0123456789abcdef0123456789abcdef01234567",
      "provenance_digest": "sha256:review-001",
      "provider": "github",
      "key_id": "org/reviews-2026",
      "algorithm": "ed25519",
      "signature": "AQID",
      "issued_at": "2026-08-01T00:00:00Z",
      "expires_at": null,
      "sequence": 1
    }
  ],
  "lifecycle": [
    {
      "review_id": "review:orders/reviewer-001",
      "sequence": 1,
      "state": "active",
      "effective_at": "2026-08-01T00:00:00Z",
      "reason_digest": null
    }
  ]
}
"#;

const OUT_OF_ORDER_LIFECYCLE_WIRE: &str = r#"
{
  "schema_version": 1,
  "attestations": [],
  "lifecycle": [
    {
      "review_id": "review:orders/reviewer-001",
      "sequence": 2,
      "state": "revoked",
      "effective_at": "2026-08-02T00:00:00Z",
      "reason_digest": "sha256:reason-001"
    },
    {
      "review_id": "review:orders/reviewer-001",
      "sequence": 1,
      "state": "active",
      "effective_at": "2026-08-01T00:00:00Z",
      "reason_digest": null
    }
  ]
}
"#;

#[test]
fn wire_round_trip_preserves_attestation_and_lifecycle_facts() {
    let document = parse_review_wire(VALID_WIRE).expect("valid review wire should parse");
    assert_eq!(document.schema_version(), 1);
    assert_eq!(document.attestations().len(), 1);
    assert_eq!(
        document.attestations()[0].review_id().as_str(),
        "review:orders/reviewer-001"
    );
    assert_eq!(document.attestations()[0].signature(), &[1, 2, 3]);
    assert_eq!(
        document.lifecycle().state_for("review:orders/reviewer-001"),
        Some(lsharp_types::intent::review_lifecycle::ReviewLifecycleState::Active)
    );

    let output = document
        .to_json_string()
        .expect("review wire should serialize deterministically");
    let reparsed = parse_review_wire(&output).expect("serialized wire should round-trip");
    assert_eq!(reparsed, document);
    assert!(output.contains("\"signature\":\"AQID\""));
}

#[test]
fn wire_lifecycle_declaration_order_does_not_change_reduced_state() {
    let document = parse_review_wire(OUT_OF_ORDER_LIFECYCLE_WIRE)
        .expect("lifecycle events should be normalized by review ID and sequence");

    assert_eq!(
        document.lifecycle().state_for("review:orders/reviewer-001"),
        Some(lsharp_types::intent::review_lifecycle::ReviewLifecycleState::Revoked)
    );
    assert_eq!(
        document
            .lifecycle()
            .events()
            .iter()
            .map(|event| event.sequence())
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    let projected: serde_json::Value = serde_json::from_str(
        &document
            .to_json_string()
            .expect("normalized lifecycle wire should serialize"),
    )
    .expect("normalized lifecycle projection should be valid JSON");
    assert_eq!(
        projected["lifecycle"]
            .as_array()
            .expect("lifecycle projection should be an array")
            .iter()
            .map(|event| {
                event["sequence"]
                    .as_u64()
                    .expect("sequence should be unsigned")
            })
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn wire_rejects_unknown_and_duplicate_fields_at_each_object_boundary() {
    let unknown = VALID_WIRE.replace(
        "\"schema_version\": 1,",
        "\"schema_version\": 1, \"extra\": true,",
    );
    assert!(matches!(
        parse_review_wire(&unknown),
        Err(ReviewWireError::Schema { .. })
    ));

    let duplicate = VALID_WIRE.replace(
        "\"schema_version\": 1,",
        "\"schema_version\": 1, \"schema_version\": 1,",
    );
    assert!(matches!(
        parse_review_wire(&duplicate),
        Err(ReviewWireError::Schema { .. })
    ));

    let nested_duplicate = VALID_WIRE.replace(
        "\"provider\": \"github\",",
        "\"provider\": \"github\", \"provider\": \"github\",",
    );
    assert!(matches!(
        parse_review_wire(&nested_duplicate),
        Err(ReviewWireError::Schema { .. })
    ));
}

#[test]
fn wire_rejects_invalid_version_algorithm_signature_and_required_arrays() {
    let version = VALID_WIRE.replace("\"schema_version\": 1", "\"schema_version\": 2");
    assert!(matches!(
        parse_review_wire(&version),
        Err(ReviewWireError::UnsupportedVersion { version: 2 })
    ));

    let algorithm = VALID_WIRE.replace("\"algorithm\": \"ed25519\"", "\"algorithm\": \"rsa\"");
    assert!(matches!(
        parse_review_wire(&algorithm),
        Err(ReviewWireError::Attestation(_))
    ));

    let signature = VALID_WIRE.replace("\"signature\": \"AQID\"", "\"signature\": \"!\"");
    assert!(matches!(
        parse_review_wire(&signature),
        Err(ReviewWireError::InvalidSignatureEncoding { .. })
    ));

    let missing = VALID_WIRE.replace("\"lifecycle\": [", "\"lifecycle_missing\": [");
    assert!(matches!(
        parse_review_wire(&missing),
        Err(ReviewWireError::Schema { .. })
    ));
}

#[test]
fn wire_rejects_noncanonical_lifecycle_effective_timestamp() {
    let malformed = VALID_WIRE.replace(
        "\"effective_at\": \"2026-08-01T00:00:00Z\"",
        "\"effective_at\": \"2026-02-30T00:00:00Z\"",
    );

    assert!(matches!(
        parse_review_wire(&malformed),
        Err(ReviewWireError::Lifecycle(
            lsharp_types::intent::review_lifecycle::LifecycleError::InvalidTimestamp {
                field: "effective_at",
                value
            }
        )) if value == "2026-02-30T00:00:00Z"
    ));
}
