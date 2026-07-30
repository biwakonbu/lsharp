use lsharp_syntax::{ast::Decl, metadata::MetadataFormKind, parse};

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
fn review_attestation_named_fields_preserve_values_and_span() {
    let program = parse(SOURCE).expect("review attestation の named fields は parse できるべき");
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("metadata 付き defn を期待しました");
    };

    let MetadataFormKind::ReviewAttestation { attestation } = &metadata.forms[0].kind else {
        panic!("review attestation form を期待しました");
    };
    assert_eq!(attestation.review_id(), "review:checkout/reviewer-001");
    assert_eq!(attestation.subject_digest(), "sha256:subject-001");
    assert_eq!(attestation.source_commit(), "0123456789abcdef");
    assert_eq!(attestation.provenance_digest(), "sha256:review-001");
    assert_eq!(attestation.provider(), "github");
    assert_eq!(attestation.key_id(), "org/reviews-2026");
    assert_eq!(attestation.algorithm(), "ed25519");
    assert_eq!(attestation.signature(), "AAECAw");
    assert_eq!(attestation.issued_at(), "2026-08-01T00:00:00Z");
    assert_eq!(attestation.expires_at(), Some("2026-09-01T00:00:00Z"));
    assert_eq!(attestation.sequence(), 3);

    let form = &metadata.forms[0];
    assert_eq!(
        form.span().start,
        SOURCE.find(":review-attestation").unwrap()
    );
    assert_eq!(form.span().end, SOURCE.rfind("\n  true").unwrap());
}

#[test]
fn review_attestation_requires_named_fields() {
    let positional = parse(
        r#"(defn review [] :review-attestation "review:checkout/reviewer-001" "sha256:subject" true)"#,
    )
    .expect_err("review attestation の positional payload は拒否するべき");
    assert_eq!(positional.code(), "LS0101");

    let unknown = parse(
        r#"(defn review [] :review-attestation :review-id "review:checkout/reviewer-001" :unknown "x" true)"#,
    )
    .expect_err("review attestation の unknown named field は拒否するべき");
    assert_eq!(unknown.code(), "LS0101");
}

#[test]
fn review_attestation_rejects_duplicate_named_fields() {
    let error = parse(
        r#"(defn review [] :review-attestation :review-id "review:checkout/reviewer-001" :review-id "review:checkout/reviewer-002" true)"#,
    )
    .expect_err("review attestation の duplicate named field は拒否するべき");
    assert_eq!(error.code(), "LS0101");
}
