use lsharp_syntax::parse;
use lsharp_types::evidence::{EvidenceMethod, EvidenceOutcome, Independence};
use lsharp_types::metadata_contract::inventory_contract_suites;
use lsharp_types::validation_input::parse_intent_graph_json;
use lsharp_types::validation_source::{SourceGraphError, source_program_to_intent_graph};

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
fn source_adapter_registers_typed_nodes_without_deriving_ids_from_span_or_order() {
    let program = parse(
        r#"
        (module Checkout
          (defn cancel []
            :intent "intent:checkout/safe-cancel" "Users can cancel an order"
            :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
            true))
        "#,
    )
    .expect("source fixture は parse できるべき");

    let graph = source_program_to_intent_graph(&program).expect("source graph が構築できるべき");
    assert_eq!(
        graph
            .nodes()
            .iter()
            .map(|node| (node.stable_id().as_str(), node.text()))
            .collect::<Vec<_>>(),
        vec![
            ("intent:checkout/safe-cancel", "Users can cancel an order"),
            (
                "claim:checkout/cancel-rejects-shipped",
                "The API rejects shipped orders"
            ),
        ]
    );
    assert!(graph.nodes().iter().all(|node| node.source_span().end > 0));
}

#[test]
fn source_adapter_projects_type_definition_metadata_nodes() {
    let program = parse(
        r#"
        (type (Result e)
          (Ok Int)
          (Err e)
          :intent "intent:checkout/result" "The result models checkout completion"
          :claim "claim:checkout/result-total" "Every checkout returns a result")
        "#,
    )
    .expect("type definition metadata fixture は parse できるべき");

    let graph =
        source_program_to_intent_graph(&program).expect("type definition nodes が投影できるべき");
    assert_eq!(
        graph
            .nodes()
            .iter()
            .map(|node| (node.stable_id().as_str(), node.text()))
            .collect::<Vec<_>>(),
        vec![
            (
                "intent:checkout/result",
                "The result models checkout completion"
            ),
            (
                "claim:checkout/result-total",
                "Every checkout returns a result"
            ),
        ]
    );
    assert!(graph.nodes().iter().all(|node| node.source_span().end > 0));
}

#[test]
fn source_adapter_projects_record_definition_metadata_nodes() {
    let program = parse(
        r#"
        (type (Point a)
          (record
            (: x Int)
            (: y a))
          :intent "intent:geometry/point" "A point has two coordinates"
          :claim "claim:geometry/point-typed" "Each coordinate follows the declared type")
        "#,
    )
    .expect("record definition metadata fixture は parse できるべき");

    let graph =
        source_program_to_intent_graph(&program).expect("record definition nodes が投影できるべき");
    assert_eq!(
        graph
            .nodes()
            .iter()
            .map(|node| (node.stable_id().as_str(), node.text()))
            .collect::<Vec<_>>(),
        vec![
            ("intent:geometry/point", "A point has two coordinates"),
            (
                "claim:geometry/point-typed",
                "Each coordinate follows the declared type"
            ),
        ]
    );
    assert!(graph.nodes().iter().all(|node| node.source_span().end > 0));
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
fn source_adapter_projects_nested_module_private_and_impl_metadata_in_declaration_order() {
    const SOURCE: &str = r#"
        (module Checkout
          (defn top []
            :intent "intent:checkout/top" "Top declaration"
            true)
          (private
            (defn hidden []
              :claim "claim:checkout/hidden" "Private declaration"
              true))
          (impl (Show Int)
            (defn render []
              :assumption "assumption:checkout/render" "Render is deterministic"
              :open-question "open-question:checkout/render" "Need external review"
              true)))
        "#;
    let program = parse(SOURCE).expect("nested declaration metadata fixture は parse できるべき");

    let graph = source_program_to_intent_graph(&program)
        .expect("nested module/private/impl metadata が投影できるべき");
    assert_eq!(
        graph
            .nodes()
            .iter()
            .map(|node| (node.stable_id().as_str(), node.text()))
            .collect::<Vec<_>>(),
        vec![
            ("intent:checkout/top", "Top declaration"),
            ("claim:checkout/hidden", "Private declaration"),
            ("assumption:checkout/render", "Render is deterministic"),
            ("open-question:checkout/render", "Need external review"),
        ]
    );

    let spans = graph
        .nodes()
        .iter()
        .map(|node| node.source_span())
        .collect::<Vec<_>>();
    assert!(spans.windows(2).all(|pair| pair[0].start < pair[1].start));
    assert!(SOURCE[spans[0].start..spans[0].end].contains("Top declaration"));
    assert!(SOURCE[spans[1].start..spans[1].end].contains("Private declaration"));
    assert!(SOURCE[spans[2].start..spans[2].end].contains("Render is deterministic"));
    assert!(SOURCE[spans[3].start..spans[3].end].contains("Need external review"));
}

#[test]
fn source_adapter_rejects_duplicate_ids_and_typed_kind_mismatch() {
    let duplicate = parse(
        r#"
        (defn first [] :intent "intent:checkout/same" "first" true)
        (defn second [] :intent "intent:checkout/same" "second" true)
        "#,
    )
    .expect("duplicate fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&duplicate),
        Err(SourceGraphError::DuplicateNode { .. })
    ));

    let mismatch = parse(r#"(defn cancel [] :claim "intent:checkout/wrong-kind" "claim" true)"#)
        .expect("kind mismatch fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&mismatch),
        Err(SourceGraphError::KindMismatch { .. })
    ));

    let empty = parse(r#"(defn cancel [] :intent "" "" true)"#)
        .expect("empty node fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&empty),
        Err(SourceGraphError::Node(_))
    ));
}

#[test]
fn source_adapter_reports_duplicate_node_with_both_source_spans() {
    const SOURCE: &str = r#"
        (module Checkout
          (private
            (defn first []
              :intent "intent:checkout/same" "first declaration"
              true))
          (impl (Show Int)
            (defn second []
              :intent "intent:checkout/same" "second declaration"
              true)))
        "#;
    let program = parse(SOURCE).expect("nested duplicate fixture は parse できるべき");

    let error = source_program_to_intent_graph(&program)
        .expect_err("duplicate source node は span 付きで拒否するべき");
    let SourceGraphError::DuplicateNode {
        id,
        first_span,
        duplicate_span,
    } = error
    else {
        panic!("duplicate node の source diagnostic を期待しました: {error:?}");
    };

    assert_eq!(id, "intent:checkout/same");
    assert!(first_span.start < duplicate_span.start);
    assert!(first_span.end <= duplicate_span.start);
    assert!(SOURCE[first_span.start..first_span.end].contains("first declaration"));
    assert!(SOURCE[duplicate_span.start..duplicate_span.end].contains("second declaration"));
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
fn source_only_metadata_does_not_create_an_empty_contract_suite() {
    let program =
        parse(r#"(defn cancel [] :intent "intent:checkout/safe-cancel" "Users can cancel" true)"#)
            .expect("source intent fixture は parse できるべき");

    let suites =
        inventory_contract_suites(&program).expect("source metadata は inventory できるべき");
    assert!(suites.is_empty());
}

#[test]
fn source_adapter_registers_node_edges_after_collecting_all_declarations() {
    let program = parse(
        r#"
        (module Checkout
          (defn cancel []
            :motivates "intent:checkout/safe-cancel" "claim:checkout/cancel-rejects-shipped"
            :constrained-by "claim:checkout/cancel-rejects-shipped" "assumption:checkout/state-authoritative"
            true)
          (defn intent []
            :intent "intent:checkout/safe-cancel" "Users can cancel an order"
            :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
            :assumption "assumption:checkout/state-authoritative" "Shipment state is authoritative"
            true))
        "#,
    )
    .expect("source edge fixture は parse できるべき");

    let graph =
        source_program_to_intent_graph(&program).expect("source edge graph が構築できるべき");
    assert_eq!(graph.edges().len(), 2);
    assert!(matches!(
        &graph.edges()[0],
        lsharp_types::evidence::Edge::Motivates { intent, claim }
            if intent.as_str() == "intent:checkout/safe-cancel"
                && claim.as_str() == "claim:checkout/cancel-rejects-shipped"
    ));
    assert!(matches!(
        &graph.edges()[1],
        lsharp_types::evidence::Edge::ConstrainedBy { claim, assumption }
            if claim.as_str() == "claim:checkout/cancel-rejects-shipped"
                && assumption.as_str() == "assumption:checkout/state-authoritative"
    ));
}

#[test]
fn source_adapter_rejects_orphan_and_mismatched_edge_endpoints() {
    let orphan = parse(
        r#"(defn cancel [] :motivates "intent:checkout/missing" "claim:checkout/cancel" true)"#,
    )
    .expect("orphan fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&orphan),
        Err(SourceGraphError::MissingNodeReference {
            relation: "motivates.intent",
            ..
        })
    ));

    let mismatch = parse(
        r#"
        (defn cancel []
          :intent "intent:checkout/safe-cancel" "Users can cancel"
          :claim "claim:checkout/cancel" "The API rejects shipped orders"
          :motivates "claim:checkout/cancel" "intent:checkout/safe-cancel"
          true)
        "#,
    )
    .expect("mismatch fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&mismatch),
        Err(SourceGraphError::EdgeIdAt { .. })
    ));
}

#[test]
fn source_adapter_reports_orphan_edge_with_directive_span() {
    const SOURCE: &str =
        r#"(defn cancel [] :motivates "intent:checkout/missing" "claim:checkout/cancel" true)"#;
    let program = parse(SOURCE).expect("orphan span fixture は parse できるべき");
    let error = source_program_to_intent_graph(&program)
        .expect_err("orphan edge は directive span 付きで拒否するべき");
    let SourceGraphError::MissingNodeReference { relation, span, .. } = error else {
        panic!("orphan edge の source diagnostic を期待しました: {error:?}");
    };

    assert_eq!(relation, "motivates.intent");
    assert!(span.start < span.end);
    assert!(SOURCE[span.start..span.end].starts_with(":motivates"));
}

#[test]
fn source_adapter_reports_malformed_edge_id_with_directive_span() {
    const SOURCE: &str =
        r#"(defn cancel [] :motivates "intent:checkout" "claim:checkout/cancel" true)"#;
    let program = parse(SOURCE).expect("malformed edge ID fixture は parse できるべき");
    let error = source_program_to_intent_graph(&program)
        .expect_err("malformed edge ID は directive span 付きで拒否するべき");
    let SourceGraphError::EdgeIdAt { relation, span, .. } = error else {
        panic!("malformed edge ID の source diagnostic を期待しました: {error:?}");
    };

    assert_eq!(relation, "motivates.intent");
    assert!(span.start < span.end);
    assert!(SOURCE[span.start..span.end].starts_with(":motivates"));
}

#[test]
fn source_adapter_registers_tested_by_claim_contract_edges() {
    let program = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :tested-by "claim:checkout/cancel-rejects-shipped" "contract:checkout/cancel-case"
          true)
        "#,
    )
    .expect("tested-by source fixture は parse できるべき");

    let graph =
        source_program_to_intent_graph(&program).expect("tested-by source graph が構築できるべき");
    assert!(matches!(
        &graph.edges()[0],
        lsharp_types::evidence::Edge::TestedBy { claim, contract }
            if claim.as_str() == "claim:checkout/cancel-rejects-shipped"
                && contract.as_str() == "contract:checkout/cancel-case"
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
fn source_adapter_rejects_orphan_or_mismatched_tested_by_claims() {
    let orphan = parse(
        r#"(defn cancel [] :tested-by "claim:checkout/missing" "contract:checkout/case" true)"#,
    )
    .expect("orphan tested-by fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&orphan),
        Err(SourceGraphError::MissingNodeReference {
            relation: "tested-by.claim",
            ..
        })
    ));

    let mismatch = parse(
        r#"(defn cancel [] :tested-by "intent:checkout/wrong-kind" "contract:checkout/case" true)"#,
    )
    .expect("kind mismatch tested-by fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&mismatch),
        Err(SourceGraphError::EdgeIdAt { .. })
    ));
}

#[test]
fn source_adapter_rejects_evidence_edges_without_registry_entries() {
    let supports = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :supports "evidence:checkout/cancel-observation" "claim:checkout/cancel-rejects-shipped"
          true)
        "#,
    )
    .expect("supports source fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&supports),
        Err(SourceGraphError::EvidenceRegistryRequired {
            relation: "supports",
            evidence_id,
            ..
        }) if evidence_id == "evidence:checkout/cancel-observation"
    ));

    let contradicts = parse(
        r#"
        (defn cancel []
          :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
          :contradicts "evidence:checkout/cancel-counterexample" "claim:checkout/cancel-rejects-shipped"
          true)
        "#,
    )
    .expect("contradicts source fixture は parse できるべき");
    assert!(matches!(
        source_program_to_intent_graph(&contradicts),
        Err(SourceGraphError::EvidenceRegistryRequired {
            relation: "contradicts",
            evidence_id,
            ..
        }) if evidence_id == "evidence:checkout/cancel-counterexample"
    ));
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
