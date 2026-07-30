//! selfhost source attestation producer と Rust source adapter の契約 tests。

use super::harness::run_evidence_registry_runtime;
use lsharp_types::intent::review_attestation::{AttestationAlgorithm, ReviewAttestation};

const VALID_SOURCE: &str = r#"(defn review [] :review-attestation :review-id "review:checkout/reviewer-001" :subject-digest "sha256:subject-001" :source-commit "0123456789abcdef" :provenance-digest "sha256:review-001" :provider "github" :key-id "org/reviews-2026" :algorithm "ed25519" :signature "AAECAw" :issued-at "2026-08-01T00:00:00Z" :expires-at "2026-09-01T00:00:00Z" :sequence 3 true)"#;

fn quote_source(source: &str) -> String {
    source.replace('"', "\\\"")
}

fn parse_bytes<'a>(lines: &mut impl Iterator<Item = &'a str>) -> Vec<u8> {
    assert_eq!(lines.next(), Some("canonical"));
    let length = lines
        .next()
        .expect("selfhost canonical bytes length is required")
        .parse::<usize>()
        .expect("selfhost canonical bytes length should be numeric");
    (0..length)
        .map(|_| {
            lines
                .next()
                .expect("selfhost canonical byte is required")
                .parse::<u8>()
                .expect("selfhost canonical byte should be numeric")
        })
        .collect()
}

fn assert_source_attestation_error(source: &str) {
    let source = quote_source(source);
    let harness = format!(
        r#"
(defn main []
  (let [program (parse-program "{source}")
        result (source-review-attestations-from-program program)
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-graph-error-code error))
      (print (source-graph-error-kind error))
      (print-string (source-graph-error-id error))
      (print-string "\n")
      (print (if (< (source-graph-error-start error) (source-graph-error-end error)) 1 0))
      0)))
"#,
        source = source
    );

    let output = run_evidence_registry_runtime(&harness);
    assert_eq!(
        output.trim().lines().collect::<Vec<_>>(),
        ["0", "8", "20", "review:checkout/reviewer-001", "1"]
    );
}

#[test]
fn selfhost_source_attestation_projects_named_fields_and_unverified_state() {
    let source = quote_source(VALID_SOURCE);
    let harness = format!(
        r#"
(defn emit-bytes [bytes idx len]
  (if (>= idx len)
    0
    (do
      (print (vector-get bytes idx))
      (emit-bytes bytes (+ idx 1) len))))
(defn main []
  (let [program (parse-program "{source}")
        result (source-review-attestations-from-program program)]
    (if (= (source-result-status result) 1)
      (let [attestations (source-result-value result)
            attestation (vector-get attestations 0)]
        (do
          (print (vector-length attestations))
          (print-string (source-review-attestation-id attestation))
          (print-string "\n")
          (print-string (source-review-attestation-subject-digest attestation))
          (print-string "\n")
          (print-string (source-review-attestation-source-commit attestation))
          (print-string "\n")
          (print-string (source-review-attestation-provider attestation))
          (print-string "\n")
          (print-string (source-review-attestation-key-id attestation))
          (print-string "\n")
          (print-string (source-review-attestation-algorithm attestation))
          (print-string "\n")
          (print-string (source-review-attestation-expires-at attestation))
          (print-string "\n")
          (print (source-review-attestation-sequence attestation))
          (print (if (string-eq (source-review-attestation-state attestation) "unverified") 1 0))
          (print (if (< (source-review-attestation-start attestation) (source-review-attestation-end attestation)) 1 0))
          (print-string "canonical\n")
          (let [bytes (source-review-attestation-canonical-bytes attestation)]
            (do
              (print (vector-length bytes))
              (emit-bytes bytes 0 (vector-length bytes))))
          0))
      (do
        (print -1)
        (print (source-graph-error-code (source-result-error result)))
        0))))
"#,
        source = source
    );

    let output = run_evidence_registry_runtime(&harness);
    let mut lines = output.trim().lines();
    assert_eq!(
        lines.by_ref().take(11).collect::<Vec<_>>(),
        [
            "1",
            "review:checkout/reviewer-001",
            "sha256:subject-001",
            "0123456789abcdef",
            "github",
            "org/reviews-2026",
            "ed25519",
            "2026-09-01T00:00:00Z",
            "3",
            "1",
            "1",
        ]
    );
    let canonical = parse_bytes(&mut lines);
    let expected = ReviewAttestation::new(
        "review:checkout/reviewer-001",
        "sha256:subject-001",
        "0123456789abcdef",
        "sha256:review-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z",
        Some("2026-09-01T00:00:00Z".to_owned()),
        3,
        vec![0, 1, 2],
    )
    .expect("Rust source parity fixture should be valid")
    .canonical_bytes();
    assert_eq!(canonical, expected);
    assert!(lines.next().is_none(), "unexpected extra selfhost output");
}

#[test]
fn selfhost_source_attestation_rejects_invalid_algorithm_and_preserves_span() {
    assert_source_attestation_error(&VALID_SOURCE.replace("ed25519", "rsa-sha256"));
}

#[test]
fn selfhost_source_attestation_rejects_invalid_signature_encoding_and_preserves_span() {
    assert_source_attestation_error(&VALID_SOURCE.replace("AAECAw", "A==="));
}

#[test]
fn selfhost_source_attestation_rejects_invalid_issued_at_date_and_preserves_span() {
    assert_source_attestation_error(
        &VALID_SOURCE.replace("2026-08-01T00:00:00Z", "2026-02-30T00:00:00Z"),
    );
}

#[test]
fn selfhost_source_attestation_rejects_non_forward_expiry_and_preserves_span() {
    assert_source_attestation_error(
        &VALID_SOURCE.replace("2026-09-01T00:00:00Z", "2026-07-01T00:00:00Z"),
    );
}
