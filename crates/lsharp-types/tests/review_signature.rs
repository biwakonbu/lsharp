use ed25519_dalek::{Signer, SigningKey};
use lsharp_types::intent::review_attestation::{
    AttestationAlgorithm, AttestationVerificationError, ReviewAttestation, ReviewVerificationState,
};
use lsharp_types::intent::review_trust_store::{ReviewTrustKey, ReviewTrustStore};

fn unsigned_attestation() -> ReviewAttestation {
    ReviewAttestation::new(
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
    .expect("valid attestation")
}

fn signed_attestation(signing_key: &SigningKey) -> ReviewAttestation {
    let mut attestation = unsigned_attestation();
    let signature = signing_key.sign(&attestation.canonical_bytes());
    attestation
        .set_signature(signature.to_bytes().to_vec())
        .expect("signature is non-empty");
    attestation
}

fn trust_store(signing_key: &SigningKey) -> ReviewTrustStore {
    let mut store = ReviewTrustStore::default();
    store
        .add_key(
            ReviewTrustKey::new(
                "github",
                "org/reviews-2026",
                AttestationAlgorithm::Ed25519,
                signing_key.verifying_key().to_bytes().to_vec(),
            )
            .expect("valid public key"),
        )
        .expect("unique trust key");
    store
}

#[test]
fn trusted_ed25519_signature_is_verified() {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation = signed_attestation(&signing_key);
    assert_eq!(
        attestation.verify(&trust_store(&signing_key)),
        Ok(ReviewVerificationState::Verified)
    );
}

#[test]
fn missing_trust_key_is_unverified_instead_of_implicitly_trusted() {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation = signed_attestation(&signing_key);
    assert_eq!(
        attestation.verify(&ReviewTrustStore::default()),
        Ok(ReviewVerificationState::Unverified)
    );
}

#[test]
fn tampered_signature_and_malformed_length_fail_closed() {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let mut tampered = signed_attestation(&signing_key);
    let mut signature = tampered.signature().to_vec();
    signature[0] ^= 0xff;
    tampered.set_signature(signature).unwrap();
    assert!(matches!(
        tampered.verify(&trust_store(&signing_key)),
        Err(AttestationVerificationError::SignatureMismatch)
    ));

    let mut malformed = unsigned_attestation();
    malformed.set_signature(vec![0; 63]).unwrap();
    assert!(matches!(
        malformed.verify(&trust_store(&signing_key)),
        Err(AttestationVerificationError::InvalidSignatureLength { actual: 63 })
    ));
}
