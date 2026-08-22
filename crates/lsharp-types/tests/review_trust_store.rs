use lsharp_types::intent::review_attestation::AttestationAlgorithm;
use lsharp_types::intent::review_trust_store::{ReviewTrustKey, ReviewTrustStore, TrustStoreError};
use lsharp_types::intent::review_wire::parse_review_wire;

fn key(provider: &str, key_id: &str) -> ReviewTrustKey {
    ReviewTrustKey::new(provider, key_id, AttestationAlgorithm::Ed25519, vec![7; 32])
        .expect("valid Ed25519 public key")
}

fn rotated_key(provider: &str, key_id: &str, active: bool) -> ReviewTrustKey {
    key(provider, key_id).with_active(active)
}

#[test]
fn trust_store_is_explicit_and_deterministic() {
    let mut store = ReviewTrustStore::default();
    store.add_key(key("github", "org/reviews-2026")).unwrap();
    store.add_key(key("scm", "release/reviews-2026")).unwrap();

    assert!(store.contains("github", "org/reviews-2026", AttestationAlgorithm::Ed25519));
    assert_eq!(
        store
            .active_key("github", AttestationAlgorithm::Ed25519)
            .unwrap()
            .key_id(),
        "org/reviews-2026"
    );
    assert!(!store.contains("github", "missing", AttestationAlgorithm::Ed25519));
    assert_eq!(
        store
            .entries()
            .iter()
            .map(|entry| (entry.provider(), entry.key_id()))
            .collect::<Vec<_>>(),
        vec![
            ("github", "org/reviews-2026"),
            ("scm", "release/reviews-2026"),
        ]
    );
}

#[test]
fn trust_store_rejects_empty_duplicate_and_wrong_sized_keys() {
    assert!(matches!(
        ReviewTrustKey::new(
            "github",
            "org/reviews-2026",
            AttestationAlgorithm::Ed25519,
            vec![0; 31],
        ),
        Err(TrustStoreError::InvalidPublicKeyLength {
            expected: 32,
            actual: 31
        })
    ));

    let mut store = ReviewTrustStore::default();
    store.add_key(key("github", "org/reviews-2026")).unwrap();
    assert!(matches!(
        store.add_key(key("github", "org/reviews-2026")),
        Err(TrustStoreError::DuplicateKey { .. })
    ));
}

#[test]
fn wire_accepts_optional_explicit_trust_store_and_round_trips_it() {
    let wire = r#"
    {
      "schema_version": 1,
      "attestations": [],
      "lifecycle": [],
      "trust_store": [
        {
          "provider": "github",
          "key_id": "org/reviews-2026",
          "algorithm": "ed25519",
          "public_key": "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc"
        }
      ]
    }
    "#;
    let document = parse_review_wire(wire).expect("explicit trust store should parse");
    let store = document
        .trust_store()
        .expect("trust store should be present");
    assert!(store.contains("github", "org/reviews-2026", AttestationAlgorithm::Ed25519));
    let output = document.to_json_string().unwrap();
    assert!(output.contains("\"trust_store\""));
    let reparsed = parse_review_wire(&output).unwrap();
    assert_eq!(reparsed, document);
}

#[test]
fn trust_store_selects_one_active_key_and_allows_retired_rotation() {
    let mut store = ReviewTrustStore::default();
    store
        .add_key(rotated_key("github", "org/reviews-2025", false))
        .unwrap();
    store
        .add_key(rotated_key("github", "org/reviews-2026", true))
        .unwrap();

    assert_eq!(
        store
            .active_key("github", AttestationAlgorithm::Ed25519)
            .unwrap()
            .key_id(),
        "org/reviews-2026"
    );
}

#[test]
fn trust_store_rejects_ambiguous_active_rotation() {
    let mut store = ReviewTrustStore::default();
    store
        .add_key(rotated_key("github", "org/reviews-2025", true))
        .unwrap();
    assert!(matches!(
        store.add_key(rotated_key("github", "org/reviews-2026", true)),
        Err(TrustStoreError::MultipleActiveKeys { .. })
    ));
}

#[test]
fn trust_store_wire_preserves_retired_key_state() {
    let wire = r#"
    {
      "schema_version": 1,
      "attestations": [],
      "lifecycle": [],
      "trust_store": [
        {
          "provider": "github",
          "key_id": "org/reviews-2025",
          "algorithm": "ed25519",
          "public_key": "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc",
          "active": false
        },
        {
          "provider": "github",
          "key_id": "org/reviews-2026",
          "algorithm": "ed25519",
          "public_key": "CAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg",
          "active": true
        }
      ]
    }
    "#;
    let document = parse_review_wire(wire).expect("rotated trust store should parse");
    assert!(
        !document
            .trust_store()
            .unwrap()
            .get("github", "org/reviews-2025", AttestationAlgorithm::Ed25519)
            .unwrap()
            .is_active()
    );
    assert_eq!(
        document
            .trust_store()
            .unwrap()
            .active_key("github", AttestationAlgorithm::Ed25519)
            .unwrap()
            .key_id(),
        "org/reviews-2026"
    );
}
