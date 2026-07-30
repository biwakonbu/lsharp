//! selfhost attestation canonical bytes と Rust oracle の parity tests。

use super::harness::run_evidence_registry_runtime;
use lsharp_types::intent::review_attestation::{AttestationAlgorithm, ReviewAttestation};

fn expected_attestation(expires_at: Option<&str>) -> ReviewAttestation {
    ReviewAttestation::new(
        "review:checkout/reviewer-001",
        "sha256:グラフ",
        "0123456789abcdef0123456789abcdef01234567",
        "sha256:レビュー-001",
        "github-日本",
        "org/レビュー-2026",
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z",
        expires_at.map(str::to_owned),
        3,
        vec![1, 2, 3],
    )
    .expect("canonical parity fixture should be valid")
}

fn parse_vector<'a>(lines: &mut impl Iterator<Item = &'a str>, label: &str) -> Vec<u8> {
    assert_eq!(lines.next(), Some(label));
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

#[test]
fn selfhost_attestation_canonical_bytes_match_rust_for_utf8_and_optional_expiry() {
    let harness = r#"
(defn emit-bytes [bytes idx len]
  (if (>= idx len)
    0
    (do
      (print-string (int-to-string (vector-get bytes idx)))
      (print-string "\n")
      (emit-bytes bytes (+ idx 1) len))))
(defn emit-vector [label bytes]
    (do
      (print-string label)
      (print-string "\n")
      (print (vector-length bytes))
      (emit-bytes bytes 0 (vector-length bytes))))
(defn main []
  (let [with-expiry
          (source-review-attestation-record
            "review:checkout/reviewer-001"
            "sha256:グラフ"
            "0123456789abcdef0123456789abcdef01234567"
            "sha256:レビュー-001"
            "github-日本"
            "org/レビュー-2026"
            "ed25519"
            "AQID"
            "2026-08-01T00:00:00Z"
            "2026-09-01T00:00:00Z"
            3
            0
            0)
        without-expiry
          (source-review-attestation-record
            "review:checkout/reviewer-001"
            "sha256:グラフ"
            "0123456789abcdef0123456789abcdef01234567"
            "sha256:レビュー-001"
            "github-日本"
            "org/レビュー-2026"
            "ed25519"
            "AQID"
            "2026-08-01T00:00:00Z"
            ""
            3
            0
            0)]
    (do
      (emit-vector "with-expiry" (source-review-attestation-canonical-bytes with-expiry))
      (emit-vector "without-expiry" (source-review-attestation-canonical-bytes without-expiry))
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let mut lines = output.trim().lines();
    let with_expiry = parse_vector(&mut lines, "with-expiry");
    let without_expiry = parse_vector(&mut lines, "without-expiry");
    assert!(lines.next().is_none(), "unexpected extra selfhost output");

    assert_eq!(
        with_expiry,
        expected_attestation(Some("2026-09-01T00:00:00Z")).canonical_bytes()
    );
    assert_eq!(
        without_expiry,
        expected_attestation(None).canonical_bytes(),
        "an omitted expiry must be encoded as an empty length-prefixed field"
    );
}
