use lsharp_syntax::parse;
use lsharp_types::intent::review_attestation::{
    AttestationAlgorithm, AttestationError, ReviewAttestation, ReviewVerificationState,
};
use lsharp_types::validation_source::{
    SourceGraphError, source_program_to_intent_graph, source_program_to_review_attestations,
};

const SOURCE: &str = r#"
(defn review []
  :review-attestation
    :review-id "review:checkout/reviewer-001"
    :subject-digest "sha256:subject-001"
    :source-commit "0123456789abcdef"
    :provenance-digest "sha256:review-001"
    :provider "github"
    :key-id "org/reviews-2026"
    :algorithm "ed25519"
    :signature "AAECAw"
    :issued-at "2026-08-01T00:00:00Z"
    :expires-at "2026-09-01T00:00:00Z"
    :sequence 3
  true)
"#;

#[test]
fn source_adapter_projects_attestation_with_canonical_bytes_and_unverified_state() {
    let program = parse(SOURCE).expect("source attestation fixture は parse できるべき");
    let attestations = source_program_to_review_attestations(&program)
        .expect("source adapter は named-field attestation を投影できるべき");
    assert_eq!(attestations.len(), 1);

    let record = &attestations[0];
    let attestation = record.attestation();
    assert_eq!(
        attestation.review_id().as_str(),
        "review:checkout/reviewer-001"
    );
    assert_eq!(attestation.subject_digest(), "sha256:subject-001");
    assert_eq!(attestation.source_commit(), "0123456789abcdef");
    assert_eq!(attestation.provenance_digest(), "sha256:review-001");
    assert_eq!(attestation.sequence(), 3);
    assert_eq!(
        record.verification_state(),
        ReviewVerificationState::Unverified
    );
    assert!(record.span().start < record.span().end);

    let canonical = attestation.canonical_bytes();
    assert!(canonical.starts_with(b"lsharp.review-attestation.v1\0"));
    assert!(
        canonical
            .windows(b"review:checkout/reviewer-001".len())
            .any(|window| { window == b"review:checkout/reviewer-001" })
    );
    assert!(source_program_to_intent_graph(&program).is_ok());
}

#[test]
fn source_adapter_preserves_absent_expiry_in_canonical_bytes_and_span() {
    let source = SOURCE.replace("    :expires-at \"2026-09-01T00:00:00Z\"\n", "");
    let program = parse(&source).expect("expires-at 省略 fixture は parse できるべき");
    let attestations = source_program_to_review_attestations(&program)
        .expect("expires-at 省略の attestation は投影できるべき");
    let record = &attestations[0];
    let attestation = record.attestation();
    assert_eq!(attestation.expires_at(), None);
    assert!(record.span().start < record.span().end);

    let expected = ReviewAttestation::new(
        "review:checkout/reviewer-001".to_string(),
        "sha256:subject-001".to_string(),
        "0123456789abcdef".to_string(),
        "sha256:review-001".to_string(),
        "github".to_string(),
        "org/reviews-2026".to_string(),
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z".to_string(),
        None,
        3,
        vec![0, 1, 2],
    )
    .expect("Rust の optional expires-at fixture は valid であるべき")
    .canonical_bytes();
    assert_eq!(attestation.canonical_bytes(), expected);
}

#[test]
fn source_adapter_preserves_utf8_fields_in_canonical_bytes() {
    let source = SOURCE
        .replace(
            ":subject-digest \"sha256:subject-001\"",
            ":subject-digest \"sha256:対象\"",
        )
        .replace(":provider \"github\"", ":provider \"レビュー\"")
        .replace(
            ":key-id \"org/reviews-2026\"",
            ":key-id \"org/reviews-2026-日本\"",
        );
    let program = parse(&source).expect("UTF-8 field fixture は parse できるべき");
    let record = &source_program_to_review_attestations(&program)
        .expect("UTF-8 field attestation は投影できるべき")[0];
    let expected = ReviewAttestation::new(
        "review:checkout/reviewer-001".to_string(),
        "sha256:対象".to_string(),
        "0123456789abcdef".to_string(),
        "sha256:review-001".to_string(),
        "レビュー".to_string(),
        "org/reviews-2026-日本".to_string(),
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z".to_string(),
        Some("2026-09-01T00:00:00Z".to_string()),
        3,
        vec![0, 1, 2],
    )
    .expect("Rust UTF-8 field fixture は valid であるべき")
    .canonical_bytes();
    assert_eq!(record.attestation().canonical_bytes(), expected);
}

#[test]
fn source_adapter_rejects_unknown_algorithm_with_attestation_span() {
    let source = SOURCE.replace(":algorithm \"ed25519\"", ":algorithm \"rsa-sha256\"");
    let program = parse(&source).expect("algorithm boundary fixture は parse できるべき");
    let error = source_program_to_review_attestations(&program).expect_err("unknown algorithm");
    match error {
        SourceGraphError::ReviewAttestationAt {
            span,
            source: AttestationError::UnsupportedAlgorithm { value },
        } => {
            assert_eq!(value, "rsa-sha256");
            assert!(span.start < span.end);
        }
        other => panic!("unexpected source attestation error: {other:?}"),
    }
}

#[test]
fn source_adapter_rejects_invalid_signature_encoding_with_attestation_span() {
    let source = SOURCE.replace(":signature \"AAECAw\"", ":signature \"A===\"");
    let program = parse(&source).expect("signature boundary fixture は parse できるべき");
    let error = source_program_to_review_attestations(&program).expect_err("invalid signature");
    match error {
        SourceGraphError::ReviewAttestationAt {
            span,
            source: AttestationError::InvalidSignatureEncoding { value },
        } => {
            assert_eq!(value, "A===");
            assert!(span.start < span.end);
        }
        other => panic!("unexpected source attestation error: {other:?}"),
    }
}

#[test]
fn source_adapter_rejects_invalid_timestamp_and_time_window_with_attestation_span() {
    let invalid_timestamp = SOURCE.replace(
        ":issued-at \"2026-08-01T00:00:00Z\"",
        ":issued-at \"2026-02-30T00:00:00Z\"",
    );
    let program =
        parse(&invalid_timestamp).expect("timestamp boundary fixture は parse できるべき");
    let error =
        source_program_to_review_attestations(&program).expect_err("invalid calendar timestamp");
    match error {
        SourceGraphError::ReviewAttestationAt {
            span,
            source: AttestationError::InvalidTimestamp { field, value },
        } => {
            assert_eq!(field, "issued_at");
            assert_eq!(value, "2026-02-30T00:00:00Z");
            assert!(span.start < span.end);
        }
        other => panic!("unexpected timestamp error: {other:?}"),
    }

    let invalid_window = SOURCE.replace(
        ":expires-at \"2026-09-01T00:00:00Z\"",
        ":expires-at \"2026-07-01T00:00:00Z\"",
    );
    let program = parse(&invalid_window).expect("time window fixture は parse できるべき");
    let error = source_program_to_review_attestations(&program)
        .expect_err("expires_at must be later than issued_at");
    match error {
        SourceGraphError::ReviewAttestationAt {
            span,
            source:
                AttestationError::InvalidTimeWindow {
                    issued_at,
                    expires_at,
                },
        } => {
            assert_eq!(issued_at, "2026-08-01T00:00:00Z");
            assert_eq!(expires_at, "2026-07-01T00:00:00Z");
            assert!(span.start < span.end);
        }
        other => panic!("unexpected time window error: {other:?}"),
    }
}
