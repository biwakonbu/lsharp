use lsharp_types::validation::{IntentGraph, ReviewEvidenceIdentity};

#[test]
fn validation_report_projects_review_evidence_identity_in_stable_order() {
    let identity = ReviewEvidenceIdentity::new(
        "sha256:graph",
        "commit-1",
        "sha256:artifact",
        "2026-08-15T00:00:00Z",
        Some("sha256:trust".to_string()),
        Some("sha256:lifecycle".to_string()),
    )
    .expect("review evidence identity should accept complete values");
    let report = IntentGraph::default()
        .validate()
        .with_review_evidence_identity(identity);

    assert_eq!(
        report.to_json_value()["review_evidence_identity"],
        serde_json::json!({
            "subject_digest": "sha256:graph",
            "source_commit": "commit-1",
            "artifact_digest": "sha256:artifact",
            "trust_store_digest": "sha256:trust",
            "lifecycle_digest": "sha256:lifecycle",
            "now": "2026-08-15T00:00:00Z"
        })
    );
    assert_eq!(
        report.to_text(),
        "status: unknown\nopen-questions: 0\nindependent-reviews: 0\ncontradicting-observations: 0\nstale-reviews: 0\nstale-evidence: 0\nreview-evidence-identity: subject=sha256:graph source=commit-1 artifact=sha256:artifact trust-store=sha256:trust lifecycle=sha256:lifecycle now=2026-08-15T00:00:00Z\n"
    );
}

#[test]
fn review_evidence_identity_rejects_blank_required_fields() {
    let error = ReviewEvidenceIdentity::new(
        " ",
        "commit-1",
        "sha256:artifact",
        "2026-08-15T00:00:00Z",
        None,
        None,
    )
    .expect_err("blank subject digest must fail closed");
    assert_eq!(
        error.to_string(),
        "review evidence identity の必須 field が空です: subject_digest"
    );
}
