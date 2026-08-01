use lsharp_types::intent::review_wire::{ReviewWireError, parse_review_wire};
use serde_json::{Value, json};

const REVIEW_PROVENANCE_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/schemas/review-provenance-v1.schema.json"
));

const VALID_PUBLIC_KEY: &str = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc";

fn valid_wire(signature: &str, public_key: &str) -> Value {
    json!({
        "schema_version": 1,
        "attestations": [{
            "review_id": "review:checkout/reviewer-001",
            "subject_digest": "sha256:subject-001",
            "source_commit": "0123456789abcdef0123456789abcdef01234567",
            "provenance_digest": "sha256:review-001",
            "provider": "github",
            "key_id": "org/reviews-2026",
            "algorithm": "ed25519",
            "signature": signature,
            "issued_at": "2026-08-01T00:00:00Z",
            "expires_at": null,
            "sequence": 1
        }],
        "lifecycle": [],
        "trust_store": [{
            "provider": "github",
            "key_id": "org/reviews-2026",
            "algorithm": "ed25519",
            "public_key": public_key
        }]
    })
}

#[test]
fn review_wire_schema_rejects_noncanonical_base64url_tail_bits() {
    let schema: Value = serde_json::from_str(REVIEW_PROVENANCE_SCHEMA)
        .expect("review wire schema は JSON であるべき");
    jsonschema::draft202012::meta::validate(&schema)
        .expect("review wire schema は Draft 2020-12 に適合するべき");
    let validator = jsonschema::draft202012::new(&schema)
        .expect("review wire schema の validator を構築できるべき");

    let valid = valid_wire("AAECAw", VALID_PUBLIC_KEY);
    assert!(
        validator.is_valid(&valid),
        "canonical base64url は受理するべき"
    );

    let invalid_signature = valid_wire("AB", VALID_PUBLIC_KEY);
    assert!(
        !validator.is_valid(&invalid_signature),
        "signature の未使用 tail bit は schema で拒否するべき"
    );
    assert!(matches!(
        parse_review_wire(&serde_json::to_string(&invalid_signature).unwrap()),
        Err(ReviewWireError::InvalidSignatureEncoding { .. })
    ));

    let invalid_public_key = valid_wire("AAECAw", &format!("{}B", &VALID_PUBLIC_KEY[..42]));
    assert!(
        !validator.is_valid(&invalid_public_key),
        "Ed25519 public key の未使用 tail bit は schema で拒否するべき"
    );
    assert!(matches!(
        parse_review_wire(&serde_json::to_string(&invalid_public_key).unwrap()),
        Err(ReviewWireError::InvalidPublicKeyEncoding { .. })
    ));
}

#[test]
fn review_wire_schema_rejects_impossible_base64url_length() {
    let schema: Value = serde_json::from_str(REVIEW_PROVENANCE_SCHEMA)
        .expect("review wire schema は JSON であるべき");
    let validator = jsonschema::draft202012::new(&schema)
        .expect("review wire schema の validator を構築できるべき");
    let invalid = valid_wire("A", VALID_PUBLIC_KEY);

    assert!(
        !validator.is_valid(&invalid),
        "length mod 4 == 1 の base64url は schema で拒否するべき"
    );
    assert!(matches!(
        parse_review_wire(&serde_json::to_string(&invalid).unwrap()),
        Err(ReviewWireError::InvalidSignatureEncoding { .. })
    ));
}

#[test]
fn review_wire_schema_rejects_duplicate_trust_store_entries() {
    let schema: Value = serde_json::from_str(REVIEW_PROVENANCE_SCHEMA)
        .expect("review wire schema は JSON であるべき");
    let validator = jsonschema::draft202012::new(&schema)
        .expect("review wire schema の validator を構築できるべき");

    let mut duplicate = valid_wire("AAECAw", VALID_PUBLIC_KEY);
    let entry = duplicate["trust_store"][0].clone();
    duplicate["trust_store"] = json!([entry.clone(), entry]);

    assert!(
        !validator.is_valid(&duplicate),
        "同一 trust-store entry の重複は schema で拒否するべき"
    );
    assert!(matches!(
        parse_review_wire(&serde_json::to_string(&duplicate).unwrap()),
        Err(ReviewWireError::TrustStore(
            lsharp_types::intent::review_trust_store::TrustStoreError::DuplicateKey { .. }
        ))
    ));
}

#[test]
fn review_wire_schema_rejects_whitespace_only_required_fields() {
    let schema: Value = serde_json::from_str(REVIEW_PROVENANCE_SCHEMA)
        .expect("review wire schema は JSON であるべき");
    let validator = jsonschema::draft202012::new(&schema)
        .expect("review wire schema の validator を構築できるべき");

    let mut attestation = valid_wire("AAECAw", VALID_PUBLIC_KEY);
    attestation["attestations"][0]["subject_digest"] = json!(" \t");
    assert!(
        !validator.is_valid(&attestation),
        "attestation required field の whitespace-only は schema で拒否するべき"
    );
    assert!(matches!(
        parse_review_wire(&serde_json::to_string(&attestation).unwrap()),
        Err(ReviewWireError::Attestation(
            lsharp_types::intent::review_attestation::AttestationError::EmptyField {
                field: "subject_digest"
            }
        ))
    ));

    let mut lifecycle = valid_wire("AAECAw", VALID_PUBLIC_KEY);
    lifecycle["lifecycle"] = json!([{
        "review_id": "review:checkout/reviewer-001",
        "sequence": 1,
        "state": "active",
        "effective_at": "2026-08-01T00:00:00Z",
        "reason_digest": "\n"
    }]);
    assert!(
        !validator.is_valid(&lifecycle),
        "optional reason_digest の whitespace-only は schema で拒否するべき"
    );
    assert!(matches!(
        parse_review_wire(&serde_json::to_string(&lifecycle).unwrap()),
        Err(ReviewWireError::Lifecycle(
            lsharp_types::intent::review_lifecycle::LifecycleError::EmptyField {
                field: "reason_digest"
            }
        ))
    ));

    let mut trust = valid_wire("AAECAw", VALID_PUBLIC_KEY);
    trust["trust_store"][0]["provider"] = json!("\u{00a0}");
    assert!(
        !validator.is_valid(&trust),
        "trust-store required field の whitespace-only は schema で拒否するべき"
    );
    assert!(matches!(
        parse_review_wire(&serde_json::to_string(&trust).unwrap()),
        Err(ReviewWireError::TrustStore(
            lsharp_types::intent::review_trust_store::TrustStoreError::EmptyField {
                field: "provider"
            }
        ))
    ));
}
