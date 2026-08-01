use ed25519_dalek::{Signer, SigningKey};
use lsharp_types::intent::review_attestation::{AttestationAlgorithm, ReviewAttestation};
use lsharp_types::intent::review_trust_store::{ReviewTrustKey, ReviewTrustStore};
use lsharp_types::intent::review_verification_receipt::{ReceiptError, ReviewVerificationReceipt};

const ATTESTATION_DIGEST: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TRUST_STORE_DIGEST: &str =
    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const RECEIPT_CANONICAL_HEX: &str = "6c73686172702e7265766965772d766572696669636174696f6e2d726563656970742e763100000000000000001a7265766965773a6f72646572732f72657669657765722d30303100000000000000087665726966696564000000000000000667697468756200000000000000106f72672f726576696577732d3230323600000000000000076564323535313900000000000000477368613235363a6161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616161616100000000000000477368613235363a626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262626262620000000000000014323032362d30382d30325430303a30303a30305a";

fn fixture_receipt() -> ReviewVerificationReceipt {
    ReviewVerificationReceipt::new(
        "review:orders/reviewer-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        ATTESTATION_DIGEST,
        TRUST_STORE_DIGEST,
        "2026-08-02T00:00:00Z",
    )
    .expect("valid verification receipt")
}

#[test]
fn receipt_canonical_bytes_match_shared_native_fixture() {
    assert_eq!(
        hex(&fixture_receipt().canonical_bytes()),
        RECEIPT_CANONICAL_HEX
    );
    let json = fixture_receipt().to_json_string().unwrap();
    assert!(json.contains("\"state\":\"verified\""));
}

#[test]
fn receipt_is_created_only_after_rust_signature_verification() {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let mut attestation = ReviewAttestation::new(
        "review:orders/reviewer-001",
        "sha256:graph-001",
        "0123456789abcdef0123456789abcdef01234567",
        "sha256:review-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z",
        None,
        1,
        vec![0; 64],
    )
    .unwrap();
    let signature = signing_key.sign(&attestation.canonical_bytes());
    attestation
        .set_signature(signature.to_bytes().to_vec())
        .unwrap();
    let mut trust_store = ReviewTrustStore::default();
    trust_store
        .add_key(
            ReviewTrustKey::new(
                "github",
                "org/reviews-2026",
                AttestationAlgorithm::Ed25519,
                signing_key.verifying_key().to_bytes().to_vec(),
            )
            .unwrap(),
        )
        .unwrap();

    let receipt = ReviewVerificationReceipt::from_verified_signature(
        &attestation,
        &trust_store,
        TRUST_STORE_DIGEST,
        "2026-08-02T00:00:00Z",
    )
    .expect("verified signature should produce a receipt");
    assert_eq!(receipt.review_id(), "review:orders/reviewer-001");
    assert_eq!(receipt.state(), "verified");
    assert!(receipt.attestation_digest().starts_with("sha256:"));
}

#[test]
fn receipt_does_not_hide_an_untrusted_key() {
    let attestation = ReviewAttestation::new(
        "review:orders/reviewer-001",
        "sha256:graph-001",
        "0123456789abcdef0123456789abcdef01234567",
        "sha256:review-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z",
        None,
        1,
        vec![7; 64],
    )
    .unwrap();

    assert!(matches!(
        ReviewVerificationReceipt::from_verified_signature(
            &attestation,
            &ReviewTrustStore::default(),
            TRUST_STORE_DIGEST,
            "2026-08-02T00:00:00Z",
        ),
        Err(ReceiptError::UntrustedKey)
    ));
}

#[test]
fn receipt_rejects_nonexistent_verification_date() {
    assert!(matches!(
        ReviewVerificationReceipt::new(
            "review:orders/reviewer-001",
            "github",
            "org/reviews-2026",
            AttestationAlgorithm::Ed25519,
            ATTESTATION_DIGEST,
            TRUST_STORE_DIGEST,
            "2026-02-30T00:00:00Z",
        ),
        Err(ReceiptError::InvalidTimestamp { .. })
    ));
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
