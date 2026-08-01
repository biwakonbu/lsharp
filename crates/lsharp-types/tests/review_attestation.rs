use lsharp_types::intent::review_attestation::{
    AttestationAlgorithm, AttestationError, ReviewAttestation,
};

fn attestation() -> ReviewAttestation {
    ReviewAttestation::new(
        "review:checkout/reviewer-001",
        "sha256:graph-001",
        "0123456789abcdef0123456789abcdef01234567",
        "sha256:review-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z",
        Some("2026-09-01T00:00:00Z".to_string()),
        3,
        vec![0x01, 0x02, 0x03],
    )
    .expect("valid attestation")
}

#[test]
fn canonical_bytes_are_deterministic_and_exclude_signature() {
    let first = attestation();
    let mut changed_signature = first.clone();
    changed_signature
        .set_signature(vec![0xaa, 0xbb])
        .expect("non-empty signature should be accepted");

    assert_eq!(first.canonical_bytes(), changed_signature.canonical_bytes());
    assert_eq!(first.canonical_bytes(), attestation().canonical_bytes());
    assert_eq!(first.algorithm().as_str(), "ed25519");
    assert_eq!(first.sequence(), 3);
}

#[test]
fn canonical_bytes_use_domain_separator_and_length_prefixed_utf8_fields() {
    let value = attestation().canonical_bytes();
    assert!(value.starts_with(b"lsharp.review-attestation.v1\0"));

    let mut offset = b"lsharp.review-attestation.v1\0".len();
    let first_length = u64::from_be_bytes(
        value[offset..offset + 8]
            .try_into()
            .expect("length prefix is eight bytes"),
    );
    offset += 8;
    assert_eq!(first_length, "review:checkout/reviewer-001".len() as u64);
    assert_eq!(
        &value[offset..offset + first_length as usize],
        b"review:checkout/reviewer-001"
    );
}

#[test]
fn optional_expiry_is_encoded_as_an_empty_field() {
    let mut value = attestation();
    value
        .set_expires_at(None)
        .expect("missing expiry is a valid optional field");
    let bytes = value.canonical_bytes();

    let mut fields = bytes[b"lsharp.review-attestation.v1\0".len()..].chunks_exact(8);
    assert!(fields.next().is_some());
    let mut expected_tail = vec![0; 8];
    expected_tail.extend_from_slice(&1u64.to_be_bytes());
    expected_tail.push(b'3');
    assert!(bytes.ends_with(&expected_tail));
}

#[test]
fn sequence_zero_is_rejected_at_the_attestation_boundary() {
    let error = ReviewAttestation::new(
        "review:checkout/reviewer-001",
        "sha256:graph-001",
        "0123456789abcdef0123456789abcdef01234567",
        "sha256:review-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z",
        None::<String>,
        0,
        vec![1, 2, 3],
    )
    .expect_err("attestation sequence は 1 以上でなければならない");

    assert_eq!(
        error,
        AttestationError::InvalidSequence { sequence: 0 }
    );
}

#[test]
fn required_fields_and_algorithm_are_fail_closed() {
    let error = ReviewAttestation::new(
        "",
        "sha256:graph-001",
        "0123456789abcdef0123456789abcdef01234567",
        "sha256:review-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z",
        None::<String>,
        1,
        vec![],
    )
    .expect_err("empty review ID must be rejected");
    assert!(matches!(
        error,
        AttestationError::EmptyField { field: "review_id" }
    ));

    assert!(matches!(
        AttestationAlgorithm::parse("rsa-sha256"),
        Err(AttestationError::UnsupportedAlgorithm { value }) if value == "rsa-sha256"
    ));
}
