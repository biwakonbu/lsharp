//! source evidence record、sampling、registry closure の contract tests。

use lsharp_syntax::parse;
use lsharp_types::evidence::{
    EvidenceMethod, EvidenceOutcome, EvidenceValidationError, GraphError, Independence,
};
use lsharp_types::validation_input::parse_intent_graph_json;
use lsharp_types::validation_source::{source_program_to_intent_graph, SourceGraphError};

fn source_evidence_form(key: &str, method: &str, outcome: &str, independence: &str) -> String {
    format!(
        r#"
          :evidence "evidence:matrix/{key}"
            :subject "claim:checkout/cancel"
            :method "{method}"
            :outcome "{outcome}"
            :runner "source-enum-matrix"
            :target "aarch64-apple-darwin"
            :source-commit "source-commit-enum-matrix"
            :artifact-digest "sha256:source-enum-matrix"
            :cases 1
            :seed 0
            :generator "source-enum-matrix-generator"
            :producer "source-enum-matrix-producer"
            :tool-version "0.2.0-dev"
            :timestamp "2026-07-26T00:00:00Z"
            :independence "{independence}"
        "#,
    )
}
#[test]
fn source_adapter_projects_record_definition_evidence_and_support_edges() {
    let program = parse(
        r#"
        (type Point
          (record (: x Int))
          :claim "claim:geometry/point-typed" "The point coordinate is an integer"
          :evidence "evidence:geometry/point-proof"
            :subject "claim:geometry/point-typed"
            :method "case"
            :outcome "pass"
            :runner "source-record-test"
            :target "aarch64-apple-darwin"
            :source-commit "source-record-commit"
            :artifact-digest "sha256:source-record"
            :cases 1
            :seed 0
            :generator "source-record-generator"
            :producer "source-record-producer"
            :tool-version "0.2.0-dev"
            :timestamp "2026-07-26T00:00:00Z"
            :independence "same-author"
          :supports "evidence:geometry/point-proof" "claim:geometry/point-typed")
        "#,
    )
    .expect("record definition evidence fixture は parse できるべき");

    let graph = source_program_to_intent_graph(&program)
        .expect("record definition evidence graph が構築できるべき");
    assert_eq!(graph.evidence().len(), 1);
    assert_eq!(graph.edges().len(), 1);
    assert!(matches!(
        &graph.edges()[0],
        lsharp_types::evidence::Edge::Supports { observation, claim }
            if observation.as_str() == "evidence:geometry/point-proof"
                && claim.as_str() == "claim:geometry/point-typed"
    ));
}

#[test]
fn source_adapter_preserves_evidence_sampling_projection_through_public_seam() {
    let program = parse(
        r#"
        (defn claim []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :evidence "evidence:checkout/cancel-case"
            :subject "claim:checkout/cancel"
            :method "case"
            :outcome "pass"
            :runner "source-sampling-test"
            :target "aarch64-apple-darwin"
            :source-commit "source-sampling-commit"
            :artifact-digest "sha256:source-sampling"
            :cases 3
            :seed 42
            :generator "checkout-cancel-fixture"
            :shrinks [8 3 1]
            :coverage [("negative" 2) ("positive" 1)]
            :producer "lsharp-test"
            :tool-version "0.2.0"
            :timestamp "2026-07-26T00:00:00Z"
            :independence "same-author"
          true)
        "#,
    )
    .expect("sampling evidence fixture は parse できるべき");

    let graph =
        source_program_to_intent_graph(&program).expect("sampling evidence graph が構築できるべき");
    let evidence = &graph.evidence()[0];
    assert_eq!(evidence.method(), EvidenceMethod::Case);
    assert_eq!(evidence.outcome(), EvidenceOutcome::Pass);
    assert_eq!(evidence.execution().cases(), 3);
    assert_eq!(evidence.execution().seed(), 42);
    assert_eq!(evidence.execution().shrinks(), &[8, 3, 1]);
    assert_eq!(evidence.execution().coverage().get("negative"), Some(&2));
    assert_eq!(evidence.execution().coverage().get("positive"), Some(&1));
    assert_eq!(evidence.provenance().producer(), "lsharp-test");
    assert_eq!(evidence.independence(), Independence::SameAuthor);
}

#[test]
fn source_adapter_reports_duplicate_evidence_with_both_source_spans() {
    const SOURCE: &str = r#"
        (defn claim []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          true)
        (defn first []
          :evidence "evidence:checkout/same"
            :subject "claim:checkout/cancel"
            :method "case"
            :outcome "pass"
            :runner "first-runner"
            :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef"
            :artifact-digest "sha256:first"
            :cases 1
            :seed 42
            :generator "checkout-cancel-fixture"
            :producer "lsharp-test"
            :tool-version "0.2.0"
            :timestamp "2026-07-25T00:00:00Z"
            :independence "same-author"
          true)
        (defn second []
          :evidence "evidence:checkout/same"
            :subject "claim:checkout/cancel"
            :method "case"
            :outcome "pass"
            :runner "second-runner"
            :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef"
            :artifact-digest "sha256:second"
            :cases 1
            :seed 42
            :generator "checkout-cancel-fixture"
            :producer "lsharp-test"
            :tool-version "0.2.0"
            :timestamp "2026-07-25T00:00:00Z"
            :independence "same-author"
          true)
        "#;
    let program = parse(SOURCE).expect("duplicate evidence fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("duplicate source evidence は span 付きで拒否するべき");
    let SourceGraphError::DuplicateEvidence {
        id,
        first_span,
        duplicate_span,
    } = error
    else {
        panic!("duplicate evidence の source diagnostic を期待しました: {error:?}");
    };

    assert_eq!(id, "evidence:checkout/same");
    assert!(first_span.start < duplicate_span.start);
    assert!(first_span.end <= duplicate_span.start);
    assert!(SOURCE[first_span.start..first_span.end].contains("first-runner"));
    assert!(SOURCE[duplicate_span.start..duplicate_span.end].contains("second-runner"));
}

#[test]
fn source_adapter_reports_invalid_evidence_enum_with_directive_span() {
    let cases = [
        (
            "method",
            "not-a-method",
            "claim:checkout/cancel",
            "not-a-method",
            "pass",
            "same-author",
        ),
        (
            "outcome",
            "not-an-outcome",
            "claim:checkout/cancel",
            "case",
            "not-an-outcome",
            "same-author",
        ),
        (
            "independence",
            "not-an-independence",
            "claim:checkout/cancel",
            "case",
            "pass",
            "not-an-independence",
        ),
        (
            "subject",
            "evidence:checkout/wrong-kind",
            "evidence:checkout/wrong-kind",
            "case",
            "pass",
            "same-author",
        ),
    ];

    for (index, (field, expected_value, subject, method, outcome, independence)) in
        cases.iter().enumerate()
    {
        let source = format!(
            r#"
            (defn cancel []
              :claim "claim:checkout/cancel" "The API rejects shipped orders"
              :evidence "evidence:checkout/invalid-{index}"
                :subject "{subject}"
                :method "{method}"
                :outcome "{outcome}"
                :runner "source-enum-test"
                :target "aarch64-apple-darwin"
                :source-commit "source-commit-enum-test"
                :artifact-digest "sha256:source-enum-test"
                :cases 1
                :seed 0
                :generator "source-enum-test-generator"
                :producer "source-enum-test-producer"
                :tool-version "0.2.0"
                :timestamp "2026-07-26T00:00:00Z"
                :independence "{independence}"
              true)
            "#,
        );
        let program = parse(&source).expect("invalid evidence enum fixture は parse できるべき");
        let error = source_program_to_intent_graph(&program)
            .expect_err("invalid evidence enum は source diagnostic になるべき");
        let SourceGraphError::InvalidEvidenceField {
            field: actual_field,
            value,
            span,
        } = error
        else {
            panic!("invalid evidence enum の span 付き診断を期待しました: {error:?}");
        };

        assert_eq!(actual_field, *field);
        assert_eq!(value, *expected_value);
        assert!(span.start < span.end);
        let diagnostic_source = &source[span.start..span.end];
        assert!(diagnostic_source.contains(":evidence"));
        assert!(diagnostic_source.contains(&format!(":{field}")));
    }
}

#[test]
fn source_adapter_rejects_empty_required_sampling_generator() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :evidence "evidence:checkout/empty-generator"
            :subject "claim:checkout/cancel"
            :method "case"
            :outcome "pass"
            :runner "source-empty-generator"
            :target "aarch64-apple-darwin"
            :source-commit "source-empty-generator-commit"
            :artifact-digest "sha256:source-empty-generator"
            :cases 1
            :seed 0
            :generator ""
            :producer "source-empty-generator-producer"
            :tool-version "0.2.0-dev"
            :timestamp "2026-07-28T00:00:00Z"
            :independence "same-author"
          true)
        "#,
    )
    .expect("empty generator fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("empty generator は source graph 登録時に拒否するべき");
    assert!(matches!(
        error,
        SourceGraphError::Graph(GraphError::InvalidEvidence {
            source: EvidenceValidationError::EmptyField { field: "generator" }
        })
    ));
}

#[test]
fn source_adapter_rejects_empty_required_execution_runner() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :evidence "evidence:checkout/empty-runner"
            :subject "claim:checkout/cancel"
            :method "case"
            :outcome "pass"
            :runner ""
            :target "aarch64-apple-darwin"
            :source-commit "source-empty-runner-commit"
            :artifact-digest "sha256:source-empty-runner"
            :cases 1
            :seed 0
            :generator "source-empty-runner-generator"
            :producer "source-empty-runner-producer"
            :tool-version "0.2.0-dev"
            :timestamp "2026-07-28T00:00:00Z"
            :independence "same-author"
          true)
        "#,
    )
    .expect("empty runner fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("empty runner は source graph 登録時に拒否するべき");
    assert!(matches!(
        error,
        SourceGraphError::Graph(GraphError::InvalidEvidence {
            source: EvidenceValidationError::EmptyField { field: "runner" }
        })
    ));
}

#[test]
fn source_adapter_rejects_empty_required_execution_target() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :evidence "evidence:checkout/empty-target"
            :subject "claim:checkout/cancel"
            :method "case"
            :outcome "pass"
            :runner "source-empty-target-runner"
            :target ""
            :source-commit "source-empty-target-commit"
            :artifact-digest "sha256:source-empty-target"
            :cases 1
            :seed 0
            :generator "source-empty-target-generator"
            :producer "source-empty-target-producer"
            :tool-version "0.2.0-dev"
            :timestamp "2026-07-28T00:00:00Z"
            :independence "same-author"
          true)
        "#,
    )
    .expect("empty target fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("empty target は source graph 登録時に拒否するべき");
    assert!(matches!(
        error,
        SourceGraphError::Graph(GraphError::InvalidEvidence {
            source: EvidenceValidationError::EmptyField { field: "target" }
        })
    ));
}

#[test]
fn source_adapter_rejects_empty_required_execution_source_commit() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :evidence "evidence:checkout/empty-source-commit"
            :subject "claim:checkout/cancel"
            :method "case"
            :outcome "pass"
            :runner "source-empty-source-commit-runner"
            :target "aarch64-apple-darwin"
            :source-commit ""
            :artifact-digest "sha256:source-empty-source-commit"
            :cases 1
            :seed 0
            :generator "source-empty-source-commit-generator"
            :producer "source-empty-source-commit-producer"
            :tool-version "0.2.0-dev"
            :timestamp "2026-07-28T00:00:00Z"
            :independence "same-author"
          true)
        "#,
    )
    .expect("empty source commit fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("empty source commit は source graph 登録時に拒否するべき");
    assert!(matches!(
        error,
        SourceGraphError::Graph(GraphError::InvalidEvidence {
            source: EvidenceValidationError::EmptyField {
                field: "source_commit"
            }
        })
    ));
}

#[test]
fn source_adapter_rejects_empty_required_execution_artifact_digest() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :evidence "evidence:checkout/empty-artifact-digest"
            :subject "claim:checkout/cancel"
            :method "case"
            :outcome "pass"
            :runner "source-empty-artifact-digest-runner"
            :target "aarch64-apple-darwin"
            :source-commit "source-empty-artifact-digest-commit"
            :artifact-digest ""
            :cases 1
            :seed 0
            :generator "source-empty-artifact-digest-generator"
            :producer "source-empty-artifact-digest-producer"
            :tool-version "0.2.0-dev"
            :timestamp "2026-07-28T00:00:00Z"
            :independence "same-author"
          true)
        "#,
    )
    .expect("empty artifact digest fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("empty artifact digest は source graph 登録時に拒否するべき");
    assert!(matches!(
        error,
        SourceGraphError::Graph(GraphError::InvalidEvidence {
            source: EvidenceValidationError::EmptyField {
                field: "artifact_digest"
            }
        })
    ));
}

#[test]
fn source_adapter_rejects_empty_required_provenance_producer() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :evidence "evidence:checkout/empty-producer"
            :subject "claim:checkout/cancel"
            :method "case"
            :outcome "pass"
            :runner "source-empty-producer-runner"
            :target "aarch64-apple-darwin"
            :source-commit "source-empty-producer-commit"
            :artifact-digest "sha256:source-empty-producer"
            :cases 1
            :seed 0
            :generator "source-empty-producer-generator"
            :producer ""
            :tool-version "0.2.0-dev"
            :timestamp "2026-07-28T00:00:00Z"
            :independence "same-author"
          true)
        "#,
    )
    .expect("empty producer fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("empty producer は source graph 登録時に拒否するべき");
    assert!(matches!(
        error,
        SourceGraphError::Graph(GraphError::InvalidEvidence {
            source: EvidenceValidationError::EmptyField { field: "producer" }
        })
    ));
}

#[test]
fn source_adapter_rejects_empty_required_provenance_tool_version() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :evidence "evidence:checkout/empty-tool-version"
            :subject "claim:checkout/cancel"
            :method "case"
            :outcome "pass"
            :runner "source-empty-tool-version-runner"
            :target "aarch64-apple-darwin"
            :source-commit "source-empty-tool-version-commit"
            :artifact-digest "sha256:source-empty-tool-version"
            :cases 1
            :seed 0
            :generator "source-empty-tool-version-generator"
            :producer "source-empty-tool-version-producer"
            :tool-version ""
            :timestamp "2026-07-28T00:00:00Z"
            :independence "same-author"
          true)
        "#,
    )
    .expect("empty tool version fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("empty tool version は source graph 登録時に拒否するべき");
    assert!(matches!(
        error,
        SourceGraphError::Graph(GraphError::InvalidEvidence {
            source: EvidenceValidationError::EmptyField {
                field: "tool_version"
            }
        })
    ));
}

#[test]
fn source_adapter_rejects_empty_required_provenance_timestamp() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :evidence "evidence:checkout/empty-timestamp"
            :subject "claim:checkout/cancel"
            :method "case"
            :outcome "pass"
            :runner "source-empty-timestamp-runner"
            :target "aarch64-apple-darwin"
            :source-commit "source-empty-timestamp-commit"
            :artifact-digest "sha256:source-empty-timestamp"
            :cases 1
            :seed 0
            :generator "source-empty-timestamp-generator"
            :producer "source-empty-timestamp-producer"
            :tool-version "0.2.0-dev"
            :timestamp ""
            :independence "same-author"
          true)
        "#,
    )
    .expect("empty timestamp fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("empty timestamp は source graph 登録時に拒否するべき");
    assert!(matches!(
        error,
        SourceGraphError::Graph(GraphError::InvalidEvidence {
            source: EvidenceValidationError::EmptyField { field: "timestamp" }
        })
    ));
}

#[test]
fn source_adapter_registers_evidence_records_before_support_edges() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :evidence "evidence:checkout/cancel-observation"
            :subject "claim:checkout/cancel-rejects-shipped"
            :method "case"
            :outcome "pass"
            :runner "cargo-test"
            :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef"
            :artifact-digest "sha256:abc123"
            :cases 1
            :seed 42
            :generator "checkout-cancel-fixture"
            :producer "lsharp-test"
            :tool-version "0.2.0"
            :timestamp "2026-07-25T00:00:00Z"
            :independence "same-author"
          :supports "evidence:checkout/cancel-observation" "claim:checkout/cancel-rejects-shipped"
          true)
        "#,
    )
    .expect("evidence source fixture は parse できるべき");

    let graph = source_program_to_intent_graph(&program)
        .expect("evidence record と supports edge が構築できるべき");
    assert_eq!(graph.evidence().len(), 1);
    assert_eq!(
        graph.evidence()[0].id().as_str(),
        "evidence:checkout/cancel-observation"
    );
    assert!(matches!(
        graph.evidence()[0].subject(),
        lsharp_types::evidence::EvidenceSubject::Claim(claim)
            if claim.as_str() == "claim:checkout/cancel-rejects-shipped"
    ));
    assert!(matches!(
        &graph.edges()[0],
        lsharp_types::evidence::Edge::Supports { observation, claim }
            if observation.as_str() == "evidence:checkout/cancel-observation"
                && claim.as_str() == "claim:checkout/cancel-rejects-shipped"
    ));
}

#[test]
fn source_adapter_registers_contradicts_evidence_edge() {
    let program = parse(&format!(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          {}
          :contradicts "evidence:matrix/cancel-counterexample" "claim:checkout/cancel"
          true)
        "#,
        source_evidence_form(
            "cancel-counterexample",
            "case",
            "contradicted",
            "same-author"
        )
    ))
    .expect("contradicts source fixture は parse できるべき");

    let graph = source_program_to_intent_graph(&program)
        .expect("contradicts evidence graph が構築できるべき");
    assert_eq!(graph.evidence().len(), 1);
    assert!(matches!(
        &graph.edges()[0],
        lsharp_types::evidence::Edge::Contradicts { observation, claim }
            if observation.as_str() == "evidence:matrix/cancel-counterexample"
                && claim.as_str() == "claim:checkout/cancel"
    ));
}

#[test]
fn source_adapter_reports_unregistered_evidence_edge_with_directive_span() {
    const SOURCE: &str = r#"
        (defn cancel []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :supports "evidence:checkout/missing" "claim:checkout/cancel"
          true)
        "#;
    let program = parse(SOURCE).expect("unregistered evidence span fixture は parse できるべき");
    let error = source_program_to_intent_graph(&program)
        .expect_err("unregistered evidence edge は directive span 付きで拒否するべき");
    let SourceGraphError::EvidenceRegistryRequired {
        relation,
        evidence_id,
        span,
    } = error
    else {
        panic!("unregistered evidence edge の source diagnostic を期待しました: {error:?}");
    };

    assert_eq!(relation, "supports");
    assert_eq!(evidence_id, "evidence:checkout/missing");
    assert!(span.start < span.end);
    assert!(SOURCE[span.start..span.end].contains(":supports"));
}

#[test]
fn source_adapter_projects_optional_sampling_fields() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :evidence "evidence:checkout/cancel-observation"
            :subject "claim:checkout/cancel-rejects-shipped"
            :method "property"
            :outcome "pass"
            :runner "cargo-test"
            :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef"
            :artifact-digest "sha256:abc123"
            :cases 3
            :seed 42
            :generator "checkout-cancel-fixture"
            :shrinks [8 3 1]
            :coverage [("negative" 2) ("positive" 1)]
            :producer "lsharp-test"
            :tool-version "0.2.0"
            :timestamp "2026-07-25T00:00:00Z"
            :independence "same-author"
          true)
        "#,
    )
    .expect("optional sampling source fixture は parse できるべき");

    let graph = source_program_to_intent_graph(&program)
        .expect("optional sampling fields は canonical evidence に投影されるべき");
    let execution = graph.evidence()[0].execution();
    assert_eq!(execution.shrinks(), &[8, 3, 1]);
    assert_eq!(
        execution.coverage().get("negative"),
        Some(&2),
        "coverage は bucket 名を保持するべき"
    );
    assert_eq!(execution.coverage().get("positive"), Some(&1));

    let sampling = graph.to_manifest_json_value()["evidence"][0]["execution"]["sampling"].clone();
    assert_eq!(sampling["shrinks"], serde_json::json!([8, 3, 1]));
    assert_eq!(
        sampling["coverage"],
        serde_json::json!({"negative": 2, "positive": 1})
    );
}

#[test]
fn source_adapter_preserves_every_evidence_enum_variant() {
    let methods = [
        ("example", EvidenceMethod::Example),
        ("case", EvidenceMethod::Case),
        ("assert", EvidenceMethod::Assert),
        ("property", EvidenceMethod::Property),
        ("production", EvidenceMethod::Production),
        ("reference", EvidenceMethod::Reference),
        ("proof", EvidenceMethod::Proof),
        ("review", EvidenceMethod::Review),
    ];
    let outcomes = [
        ("pass", EvidenceOutcome::Pass),
        ("fail", EvidenceOutcome::Fail),
        ("contradicted", EvidenceOutcome::Contradicted),
        ("unknown", EvidenceOutcome::Unknown),
        ("stale", EvidenceOutcome::Stale),
    ];
    let independences = [
        ("same-author", Independence::SameAuthor),
        ("independent-review", Independence::IndependentReview),
        ("external-observation", Independence::ExternalObservation),
    ];

    let mut evidence_forms = String::new();
    for (index, (wire, _)) in methods.iter().enumerate() {
        evidence_forms.push_str(&source_evidence_form(
            &format!("method-{index}"),
            wire,
            "pass",
            "same-author",
        ));
    }
    for (index, (wire, _)) in outcomes.iter().enumerate() {
        evidence_forms.push_str(&source_evidence_form(
            &format!("outcome-{index}"),
            "case",
            wire,
            "same-author",
        ));
    }
    for (index, (wire, _)) in independences.iter().enumerate() {
        evidence_forms.push_str(&source_evidence_form(
            &format!("independence-{index}"),
            "case",
            "pass",
            wire,
        ));
    }

    let source = format!(
        r#"(defn cancel []
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          {evidence_forms}
          true)"#
    );
    let program = parse(&source).expect("全 Evidence enum source fixture は parse できるべき");
    let graph = source_program_to_intent_graph(&program)
        .expect("source adapter は全 Evidence enum variant を保持するべき");

    assert_eq!(
        graph.evidence().len(),
        methods.len() + outcomes.len() + independences.len()
    );
    assert_eq!(
        graph
            .evidence()
            .iter()
            .take(methods.len())
            .map(|evidence| evidence.method())
            .collect::<Vec<_>>(),
        methods.iter().map(|(_, value)| *value).collect::<Vec<_>>()
    );
    assert_eq!(
        graph
            .evidence()
            .iter()
            .skip(methods.len())
            .take(outcomes.len())
            .map(|evidence| evidence.outcome())
            .collect::<Vec<_>>(),
        outcomes.iter().map(|(_, value)| *value).collect::<Vec<_>>()
    );
    assert_eq!(
        graph
            .evidence()
            .iter()
            .skip(methods.len() + outcomes.len())
            .map(|evidence| evidence.independence())
            .collect::<Vec<_>>(),
        independences
            .iter()
            .map(|(_, value)| *value)
            .collect::<Vec<_>>()
    );

    let manifest = graph
        .to_manifest_json_string()
        .expect("source evidence graph は manifest 化できるべき");
    let decoded = parse_intent_graph_json(&manifest)
        .expect("source evidence manifest は input parser で復元できるべき");
    assert_eq!(decoded, graph);
}
