use ed25519_dalek::{Signer, SigningKey};
use lsharp_types::intent::review_attestation::{
    AttestationAlgorithm, AttestationError, AttestationVerificationError, ReviewAttestation,
    ReviewVerificationState,
};
use lsharp_types::intent::review_lifecycle::{
    ReviewLifecycleEvent, ReviewLifecycleRegistry, ReviewLifecycleState,
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

fn signed_expiring_attestation(
    signing_key: &SigningKey,
    issued_at: &str,
    expires_at: &str,
) -> ReviewAttestation {
    let mut attestation = ReviewAttestation::new(
        "review:orders/reviewer-001",
        "sha256:graph-001",
        "0123456789abcdef0123456789abcdef01234567",
        "sha256:review-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        issued_at,
        Some(expires_at.to_string()),
        1,
        vec![0; 64],
    )
    .expect("valid expiring attestation");
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

fn lifecycle(state: ReviewLifecycleState, sequence: u64) -> ReviewLifecycleRegistry {
    let mut registry = ReviewLifecycleRegistry::default();
    let initial = if state.is_terminal() {
        ReviewLifecycleState::Proposed
    } else {
        state
    };
    registry
        .add_event(
            ReviewLifecycleEvent::new(
                "review:orders/reviewer-001",
                if state.is_terminal() {
                    sequence - 2
                } else {
                    sequence
                },
                initial,
                "2026-08-01T00:00:00Z",
                None,
            )
            .expect("valid lifecycle event"),
        )
        .expect("valid lifecycle registry");
    if state.is_terminal() {
        registry
            .add_event(
                ReviewLifecycleEvent::new(
                    "review:orders/reviewer-001",
                    sequence - 1,
                    ReviewLifecycleState::Active,
                    "2026-08-01T12:00:00Z",
                    None,
                )
                .expect("valid active lifecycle event"),
            )
            .expect("valid active lifecycle registry");
        registry
            .add_event(
                ReviewLifecycleEvent::new(
                    "review:orders/reviewer-001",
                    sequence,
                    state,
                    "2026-08-02T00:00:00Z",
                    None,
                )
                .expect("valid terminal lifecycle event"),
            )
            .expect("valid terminal lifecycle registry");
    }
    registry
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

#[test]
fn active_lifecycle_matching_attestation_sequence_is_verified() {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation = signed_attestation(&signing_key);
    assert_eq!(
        attestation.verify_with_lifecycle(
            &trust_store(&signing_key),
            &lifecycle(ReviewLifecycleState::Active, 1)
        ),
        Ok(ReviewVerificationState::Verified)
    );
}

#[test]
fn missing_or_non_active_lifecycle_never_implicitly_verifies() {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation = signed_attestation(&signing_key);
    let store = trust_store(&signing_key);
    assert_eq!(
        attestation.verify_with_lifecycle(&store, &ReviewLifecycleRegistry::default()),
        Ok(ReviewVerificationState::Unverified)
    );
    assert_eq!(
        attestation.verify_with_lifecycle(&store, &lifecycle(ReviewLifecycleState::Proposed, 1)),
        Ok(ReviewVerificationState::Unverified)
    );
    assert_eq!(
        attestation.verify_with_lifecycle(&store, &lifecycle(ReviewLifecycleState::Revoked, 3)),
        Ok(ReviewVerificationState::Revoked)
    );
    assert_eq!(
        attestation.verify_with_lifecycle(&store, &lifecycle(ReviewLifecycleState::Superseded, 3)),
        Ok(ReviewVerificationState::Stale)
    );
}

#[test]
fn active_lifecycle_sequence_mismatch_is_stale() {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation = signed_attestation(&signing_key);
    assert_eq!(
        attestation.verify_with_lifecycle(
            &trust_store(&signing_key),
            &lifecycle(ReviewLifecycleState::Active, 2)
        ),
        Ok(ReviewVerificationState::Stale)
    );
}

#[test]
fn matching_subject_source_and_provenance_stay_verified() {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation = signed_attestation(&signing_key);
    assert_eq!(
        attestation.verify_against(
            &trust_store(&signing_key),
            &lifecycle(ReviewLifecycleState::Active, 1),
            "sha256:graph-001",
            "0123456789abcdef0123456789abcdef01234567",
            "sha256:review-001",
        ),
        Ok(ReviewVerificationState::Verified)
    );
}

#[test]
fn subject_source_or_provenance_mismatch_is_stale() {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation = signed_attestation(&signing_key);
    let store = trust_store(&signing_key);
    let lifecycle = lifecycle(ReviewLifecycleState::Active, 1);
    for (subject, source_commit, provenance) in [
        (
            "sha256:graph-other",
            "0123456789abcdef0123456789abcdef01234567",
            "sha256:review-001",
        ),
        (
            "sha256:graph-001",
            "fedcba9876543210fedcba9876543210fedcba98",
            "sha256:review-001",
        ),
        (
            "sha256:graph-001",
            "0123456789abcdef0123456789abcdef01234567",
            "sha256:review-other",
        ),
    ] {
        assert_eq!(
            attestation.verify_against(&store, &lifecycle, subject, source_commit, provenance,),
            Ok(ReviewVerificationState::Stale)
        );
    }
}

#[test]
fn explicit_clock_enforces_issue_and_expiry_window() {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation =
        signed_expiring_attestation(&signing_key, "2026-08-01T00:00:00Z", "2026-09-01T00:00:00Z");
    let store = trust_store(&signing_key);
    let lifecycle = lifecycle(ReviewLifecycleState::Active, 1);

    assert_eq!(
        attestation.verify_against_at(
            &store,
            &lifecycle,
            "sha256:graph-001",
            "0123456789abcdef0123456789abcdef01234567",
            "sha256:review-001",
            "2026-08-31T23:59:59Z",
        ),
        Ok(ReviewVerificationState::Verified)
    );
    for now in [
        "2026-07-31T23:59:59Z",
        "2026-09-01T00:00:00Z",
        "2026-09-01T00:00:01Z",
    ] {
        assert_eq!(
            attestation.verify_against_at(
                &store,
                &lifecycle,
                "sha256:graph-001",
                "0123456789abcdef0123456789abcdef01234567",
                "sha256:review-001",
                now,
            ),
            Ok(ReviewVerificationState::Stale),
            "now={now}"
        );
    }
}

#[test]
fn lifecycle_transition_is_not_effective_before_its_clock() {
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
        Some("2026-09-01T00:00:00Z".to_string()),
        2,
        vec![0; 64],
    )
    .expect("valid delayed attestation");
    let signature = signing_key.sign(&attestation.canonical_bytes());
    attestation
        .set_signature(signature.to_bytes().to_vec())
        .expect("signature is non-empty");

    let mut lifecycle = ReviewLifecycleRegistry::default();
    lifecycle
        .add_event(
            ReviewLifecycleEvent::new(
                "review:orders/reviewer-001",
                1,
                ReviewLifecycleState::Proposed,
                "2026-08-01T00:00:00Z",
                None,
            )
            .expect("valid proposed event"),
        )
        .expect("proposed event should be accepted");
    lifecycle
        .add_event(
            ReviewLifecycleEvent::new(
                "review:orders/reviewer-001",
                2,
                ReviewLifecycleState::Active,
                "2026-08-02T00:00:00Z",
                None,
            )
            .expect("valid future active event"),
        )
        .expect("future active event should be accepted");

    let store = trust_store(&signing_key);
    let verify = |now| {
        attestation.verify_against_at(
            &store,
            &lifecycle,
            "sha256:graph-001",
            "0123456789abcdef0123456789abcdef01234567",
            "sha256:review-001",
            now,
        )
    };

    assert_eq!(
        verify("2026-08-01T12:00:00Z"),
        Ok(ReviewVerificationState::Unverified)
    );
    assert_eq!(
        verify("2026-08-02T00:00:00Z"),
        Ok(ReviewVerificationState::Verified)
    );
}

#[test]
fn malformed_timestamp_and_invalid_window_fail_at_input_boundary() {
    let invalid_format = ReviewAttestation::new(
        "review:orders/reviewer-001",
        "sha256:graph-001",
        "0123456789abcdef0123456789abcdef01234567",
        "sha256:review-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00+00:00",
        None,
        1,
        vec![0; 64],
    );
    assert!(matches!(
        invalid_format,
        Err(AttestationError::InvalidTimestamp {
            field: "issued_at",
            ..
        })
    ));

    let invalid_window = ReviewAttestation::new(
        "review:orders/reviewer-001",
        "sha256:graph-001",
        "0123456789abcdef0123456789abcdef01234567",
        "sha256:review-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-09-01T00:00:00Z",
        Some("2026-09-01T00:00:00Z".to_string()),
        1,
        vec![0; 64],
    );
    assert!(matches!(
        invalid_window,
        Err(AttestationError::InvalidTimeWindow { .. })
    ));
}

#[test]
fn malformed_explicit_clock_is_a_verification_error() {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation =
        signed_expiring_attestation(&signing_key, "2026-08-01T00:00:00Z", "2026-09-01T00:00:00Z");
    assert!(matches!(
        attestation.verify_against_at(
            &trust_store(&signing_key),
            &lifecycle(ReviewLifecycleState::Active, 1),
            "sha256:graph-001",
            "0123456789abcdef0123456789abcdef01234567",
            "sha256:review-001",
            "2026-08-01T00:00:00+00:00",
        ),
        Err(AttestationVerificationError::InvalidTimestamp { field: "now", .. })
    ));
}

#[test]
fn legacy_binding_api_does_not_verify_expiring_attestation_without_clock() {
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let attestation =
        signed_expiring_attestation(&signing_key, "2026-08-01T00:00:00Z", "2026-09-01T00:00:00Z");
    assert_eq!(
        attestation.verify_with_lifecycle(
            &trust_store(&signing_key),
            &lifecycle(ReviewLifecycleState::Active, 1),
        ),
        Ok(ReviewVerificationState::Unverified)
    );
    assert_eq!(
        attestation.verify_against(
            &trust_store(&signing_key),
            &lifecycle(ReviewLifecycleState::Active, 1),
            "sha256:graph-001",
            "0123456789abcdef0123456789abcdef01234567",
            "sha256:review-001",
        ),
        Ok(ReviewVerificationState::Unverified)
    );
}
