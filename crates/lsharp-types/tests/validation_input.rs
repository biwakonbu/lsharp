use lsharp_types::evidence::{EvidenceValidationError, GraphError};
use lsharp_types::intent::NodeTextError;
use lsharp_types::validation::{IntentGraph, ValidationStatus};
use lsharp_types::validation_input::{ValidationInputError, parse_intent_graph_json};

fn complete_manifest() -> &'static str {
    r#"
    {
      "schema_version": 1,
      "nodes": [
        {
          "kind": "intent",
          "namespace": "checkout",
          "key": "safe-cancel",
          "text": "Users can cancel before shipment",
          "span": {"start": 0, "end": 10}
        },
        {
          "kind": "claim",
          "namespace": "checkout",
          "key": "cancel-rejects-shipped",
          "text": "The API rejects shipped orders",
          "span": {"start": 11, "end": 20}
        }
      ],
      "evidence": [
        {
          "namespace": "checkout",
          "key": "review-001",
          "method": "review",
          "subject": {"kind": "claim", "namespace": "checkout", "key": "cancel-rejects-shipped"},
          "outcome": "pass",
          "execution": {
            "runner": "validator-test",
            "target": "host",
            "source_commit": "commit-1",
            "artifact_digest": "sha256:artifact",
            "sampling": {
              "cases": 1,
              "seed": 0,
              "generator": "fixture",
              "shrinks": [],
              "coverage": {"all": 1}
            }
          },
          "provenance": {
            "producer": "validator-test",
            "tool_version": "0.2",
            "timestamp": "2026-07-23T00:00:00Z"
          },
          "independence": "independent-review"
        }
      ],
      "edges": [
        {
          "relation": "motivates",
          "intent": {"namespace": "checkout", "key": "safe-cancel"},
          "claim": {"namespace": "checkout", "key": "cancel-rejects-shipped"}
        },
        {
          "relation": "tested-by",
          "claim": {"namespace": "checkout", "key": "cancel-rejects-shipped"},
          "contract": {"namespace": "checkout", "key": "cancel-case"}
        },
        {
          "relation": "evaluates",
          "review": {"namespace": "checkout", "key": "reviewer-001"},
          "subject": {"kind": "evidence", "namespace": "checkout", "key": "review-001"}
        }
      ]
    }
    "#
}

#[test]
fn parse_manifest_builds_complete_graph_and_passes_validation() {
    let graph = parse_intent_graph_json(complete_manifest()).expect("manifest should parse");

    assert_eq!(graph.nodes().len(), 2);
    assert_eq!(graph.evidence().len(), 1);
    assert_eq!(graph.edges().len(), 3);
    assert_eq!(graph.validate().status(), ValidationStatus::Pass);
}

#[test]
fn parse_manifest_rejects_coverage_total_that_does_not_match_cases() {
    let manifest = complete_manifest().replace("\"cases\": 1", "\"cases\": 2");

    assert!(matches!(
        parse_intent_graph_json(&manifest),
        Err(ValidationInputError::Graph(GraphError::InvalidEvidence {
            source: EvidenceValidationError::CoverageCountMismatch {
                cases: 2,
                covered: 1
            }
        }))
    ));
}

#[test]
fn parse_manifest_rejects_duplicate_top_level_fields() {
    let cases = [
        (
            "schema_version",
            r#"{"schema_version":1,"schema_version":1,"nodes":[],"evidence":[],"edges":[]}"#,
        ),
        (
            "nodes",
            r#"{"schema_version":1,"nodes":[],"nodes":[],"evidence":[],"edges":[]}"#,
        ),
        (
            "reviews",
            r#"{"schema_version":1,"nodes":[],"reviews":[],"reviews":[],"evidence":[],"edges":[]}"#,
        ),
        (
            "evidence",
            r#"{"schema_version":1,"nodes":[],"evidence":[],"evidence":[],"edges":[]}"#,
        ),
        (
            "edges",
            r#"{"schema_version":1,"nodes":[],"evidence":[],"edges":[],"edges":[]}"#,
        ),
    ];

    for (field, manifest) in cases {
        assert!(
            matches!(
                parse_intent_graph_json(manifest),
                Err(ValidationInputError::Json(_))
            ),
            "duplicate top-level field must fail closed: {field}"
        );
    }
}

#[test]
fn parse_manifest_preserves_unknown_status_for_trace_gaps_and_open_questions() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [
        {"kind":"intent","namespace":"checkout","key":"safe-cancel","text":"Users can cancel"},
        {"kind":"claim","namespace":"checkout","key":"cancel-rejects-shipped","text":"API rejects shipped"},
        {"kind":"open-question","namespace":"checkout","key":"after-label","text":"Can cancellation happen after a label?"}
      ],
      "evidence": [],
      "edges": []
    }
    "#;

    let graph = parse_intent_graph_json(manifest).expect("manifest should parse");
    let report = graph.validate();
    assert_eq!(report.status(), ValidationStatus::Unknown);
    assert_eq!(report.open_questions(), 1);
    assert_eq!(report.trace_gaps().len(), 2);
}

#[test]
fn parse_manifest_wire_schema_uses_kebab_case_node_kind() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [
        {"kind":"open-question","namespace":"checkout","key":"after-label","text":"Can cancellation happen after a label?"}
      ],
      "evidence": [],
      "edges": []
    }
    "#;

    let graph = parse_intent_graph_json(manifest).expect("kebab-case node kind should parse");
    assert_eq!(graph.validate().open_questions(), 1);

    let invalid = manifest.replace("open-question", "open_question");
    assert!(matches!(
        parse_intent_graph_json(&invalid),
        Err(ValidationInputError::Json(_))
    ));
}

#[test]
fn parse_manifest_rejects_unknown_node_references() {
    let manifest = complete_manifest().replace(
        "\"key\": \"safe-cancel\"},\n          \"claim\"",
        "\"key\": \"missing-intent\"},\n          \"claim\"",
    );

    assert!(matches!(
        parse_intent_graph_json(&manifest),
        Err(ValidationInputError::MissingNodeReference { .. })
    ));
}

#[test]
fn parse_manifest_reports_missing_edge_endpoint_relation_and_id() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [
        {"kind":"claim","namespace":"checkout","key":"cancel-rejects-shipped","text":"API rejects shipped"}
      ],
      "evidence": [],
      "edges": [
        {
          "relation": "constrained-by",
          "claim": {"namespace":"checkout","key":"cancel-rejects-shipped"},
          "assumption": {"namespace":"checkout","key":"missing-assumption"}
        }
      ]
    }
    "#;

    assert!(matches!(
        parse_intent_graph_json(manifest),
        Err(ValidationInputError::MissingNodeReference {
            relation: "constrained-by.assumption",
            id,
        }) if id == "assumption:checkout/missing-assumption"
    ));
}

#[test]
fn parse_manifest_rejects_unsupported_schema_version() {
    let manifest = complete_manifest().replace("\"schema_version\": 1", "\"schema_version\": 2");

    assert!(matches!(
        parse_intent_graph_json(&manifest),
        Err(ValidationInputError::UnsupportedSchemaVersion { version: 2 })
    ));
}

#[test]
fn parse_manifest_rejects_duplicate_nodes() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [
        {"kind":"intent","namespace":"checkout","key":"safe-cancel","text":"first"},
        {"kind":"intent","namespace":"checkout","key":"safe-cancel","text":"duplicate"}
      ],
      "evidence": [],
      "edges": []
    }
    "#;

    assert!(matches!(
        parse_intent_graph_json(manifest),
        Err(ValidationInputError::Graph(_))
    ));
}

#[test]
fn parse_manifest_rejects_duplicate_evidence_ids() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [
        {"kind":"claim","namespace":"checkout","key":"cancel-rejects-shipped","text":"API rejects shipped"}
      ],
      "evidence": [
        {
          "namespace": "checkout",
          "key": "review-001",
          "method": "review",
          "subject": {"kind": "claim", "namespace": "checkout", "key": "cancel-rejects-shipped"},
          "outcome": "pass",
          "execution": {
            "runner": "validator-test",
            "target": "host",
            "source_commit": "commit-1",
            "artifact_digest": "sha256:artifact",
            "sampling": {"cases": 1, "seed": 0, "generator": "fixture"}
          },
          "provenance": {
            "producer": "validator-test",
            "tool_version": "0.2",
            "timestamp": "2026-07-23T00:00:00Z"
          },
          "independence": "independent-review"
        },
        {
          "namespace": "checkout",
          "key": "review-001",
          "method": "review",
          "subject": {"kind": "claim", "namespace": "checkout", "key": "cancel-rejects-shipped"},
          "outcome": "pass",
          "execution": {
            "runner": "validator-test",
            "target": "host",
            "source_commit": "commit-1",
            "artifact_digest": "sha256:artifact",
            "sampling": {"cases": 1, "seed": 0, "generator": "fixture"}
          },
          "provenance": {
            "producer": "validator-test",
            "tool_version": "0.2",
            "timestamp": "2026-07-23T00:00:00Z"
          },
          "independence": "independent-review"
        }
      ],
      "edges": []
    }
    "#;

    assert!(matches!(
        parse_intent_graph_json(manifest),
        Err(ValidationInputError::Graph(GraphError::DuplicateEvidence { id }))
            if id.as_str() == "evidence:checkout/review-001"
    ));
}

#[test]
fn parse_manifest_rejects_edges_that_reference_missing_evidence() {
    let manifest = complete_manifest().replace(
        "\"subject\": {\"kind\": \"evidence\", \"namespace\": \"checkout\", \"key\": \"review-001\"}",
        "\"subject\": {\"kind\": \"evidence\", \"namespace\": \"checkout\", \"key\": \"missing-evidence\"}",
    );

    assert!(matches!(
        parse_intent_graph_json(&manifest),
        Err(ValidationInputError::Graph(GraphError::MissingEvidence { id }))
            if id.as_str() == "evidence:checkout/missing-evidence"
    ));
}

#[test]
fn empty_manifest_is_a_valid_unknown_graph() {
    let graph: IntentGraph =
        parse_intent_graph_json(r#"{"schema_version":1,"nodes":[],"evidence":[],"edges":[]}"#)
            .expect("empty graph is still a valid manifest");

    assert_eq!(graph.validate().status(), ValidationStatus::Unknown);
}

#[test]
fn parse_manifest_rejects_unknown_fields_and_reversed_spans() {
    let unknown_field = complete_manifest().replace(
        "\"schema_version\": 1,",
        "\"schema_version\": 1, \"unexpected\": true,",
    );
    assert!(matches!(
        parse_intent_graph_json(&unknown_field),
        Err(ValidationInputError::Json(_))
    ));

    let reversed_span =
        complete_manifest().replace("\"start\": 0, \"end\": 10", "\"start\": 10, \"end\": 0");
    assert!(matches!(
        parse_intent_graph_json(&reversed_span),
        Err(ValidationInputError::InvalidSpan { start: 10, end: 0 })
    ));
}

#[test]
fn parse_manifest_rejects_unknown_fields_in_edge_payloads() {
    let extra_edges = [
        serde_json::json!({
            "relation": "constrained-by",
            "claim": {"namespace": "checkout", "key": "cancel-rejects-shipped"},
            "assumption": {"namespace": "checkout", "key": "state-authoritative"}
        }),
        serde_json::json!({
            "relation": "supports",
            "observation": {"namespace": "checkout", "key": "review-001"},
            "claim": {"namespace": "checkout", "key": "cancel-rejects-shipped"}
        }),
        serde_json::json!({
            "relation": "contradicts",
            "observation": {"namespace": "checkout", "key": "review-001"},
            "claim": {"namespace": "checkout", "key": "cancel-rejects-shipped"}
        }),
        serde_json::json!({
            "relation": "invalidates",
            "change": {"namespace": "checkout", "key": "api-v2"},
            "subject": {"kind": "evidence", "namespace": "checkout", "key": "review-001"}
        }),
    ];

    for edge_index in 0..7 {
        let mut value: serde_json::Value =
            serde_json::from_str(complete_manifest()).expect("complete fixture should be JSON");
        if let Some(nodes) = value["nodes"].as_array_mut() {
            nodes.push(serde_json::json!({
                "kind": "assumption",
                "namespace": "checkout",
                "key": "state-authoritative",
                "text": "Shipment state is authoritative"
            }));
        }
        if let Some(edges) = value["edges"].as_array_mut() {
            edges.extend(extra_edges.iter().cloned());
        }
        value["edges"][edge_index]["unexpected"] = serde_json::json!(true);
        let manifest = serde_json::to_string(&value).expect("manifest mutation should serialize");

        assert!(
            matches!(
                parse_intent_graph_json(&manifest),
                Err(ValidationInputError::Json(_))
            ),
            "unknown fields must be rejected for edge index {edge_index}"
        );
    }
}

#[test]
fn parse_manifest_rejects_missing_top_level_required_fields() {
    for field in ["schema_version", "nodes", "evidence", "edges"] {
        let mut value: serde_json::Value =
            serde_json::from_str(complete_manifest()).expect("complete fixture should be JSON");
        value
            .as_object_mut()
            .expect("manifest fixture should be an object")
            .remove(field);
        let manifest = serde_json::to_string(&value).expect("manifest mutation should serialize");

        assert!(
            matches!(
                parse_intent_graph_json(&manifest),
                Err(ValidationInputError::Json(_))
            ),
            "missing top-level {field} must fail during JSON decoding"
        );
    }
}

#[test]
fn parse_manifest_rejects_negative_unsigned_numeric_fields() {
    let cases = [
        (
            "span.start",
            complete_manifest().replace("\"start\": 0", "\"start\": -1"),
        ),
        (
            "span.end",
            complete_manifest().replace("\"end\": 10", "\"end\": -1"),
        ),
        (
            "sampling.cases",
            complete_manifest().replace("\"cases\": 1", "\"cases\": -1"),
        ),
        (
            "sampling.seed",
            complete_manifest().replace("\"seed\": 0", "\"seed\": -1"),
        ),
        (
            "sampling.shrinks",
            complete_manifest().replace("\"shrinks\": []", "\"shrinks\": [-1]"),
        ),
        (
            "sampling.coverage",
            complete_manifest()
                .replace("\"coverage\": {\"all\": 1}", "\"coverage\": {\"all\": -1}"),
        ),
    ];

    for (field, manifest) in cases {
        assert!(
            matches!(
                parse_intent_graph_json(&manifest),
                Err(ValidationInputError::Json(_))
            ),
            "negative {field} must fail during JSON decoding"
        );
    }
}

#[test]
fn parse_manifest_rejects_unsigned_numeric_overflow() {
    let overflow = "18446744073709551616";
    let cases = [
        (
            "span.start",
            complete_manifest().replace("\"start\": 0", &format!("\"start\": {overflow}")),
        ),
        (
            "span.end",
            complete_manifest().replace("\"end\": 10", &format!("\"end\": {overflow}")),
        ),
        (
            "sampling.cases",
            complete_manifest().replace("\"cases\": 1", &format!("\"cases\": {overflow}")),
        ),
        (
            "sampling.seed",
            complete_manifest().replace("\"seed\": 0", &format!("\"seed\": {overflow}")),
        ),
        (
            "sampling.shrinks",
            complete_manifest().replace("\"shrinks\": []", &format!("\"shrinks\": [{overflow}]")),
        ),
        (
            "sampling.coverage",
            complete_manifest().replace(
                "\"coverage\": {\"all\": 1}",
                &format!("\"coverage\": {{\"all\": {overflow}}}"),
            ),
        ),
    ];

    for (field, manifest) in cases {
        assert!(
            matches!(
                parse_intent_graph_json(&manifest),
                Err(ValidationInputError::Json(_))
            ),
            "overflow {field} must fail during JSON decoding"
        );
    }
}

#[test]
fn parse_manifest_rejects_fractional_unsigned_numeric_fields() {
    let cases = [
        (
            "span.start",
            complete_manifest().replace("\"start\": 0", "\"start\": 0.5"),
        ),
        (
            "span.end",
            complete_manifest().replace("\"end\": 10", "\"end\": 10.5"),
        ),
        (
            "sampling.cases",
            complete_manifest().replace("\"cases\": 1", "\"cases\": 1.5"),
        ),
        (
            "sampling.seed",
            complete_manifest().replace("\"seed\": 0", "\"seed\": 0.5"),
        ),
        (
            "sampling.shrinks",
            complete_manifest().replace("\"shrinks\": []", "\"shrinks\": [0.5]"),
        ),
        (
            "sampling.coverage",
            complete_manifest()
                .replace("\"coverage\": {\"all\": 1}", "\"coverage\": {\"all\": 1.5}"),
        ),
    ];

    for (field, manifest) in cases {
        assert!(
            matches!(
                parse_intent_graph_json(&manifest),
                Err(ValidationInputError::Json(_))
            ),
            "fractional {field} must fail during JSON decoding"
        );
    }
}

#[test]
fn parse_manifest_rejects_null_unsigned_numeric_fields() {
    let cases = [
        (
            "span.start",
            complete_manifest().replace("\"start\": 0", "\"start\": null"),
        ),
        (
            "span.end",
            complete_manifest().replace("\"end\": 10", "\"end\": null"),
        ),
        (
            "sampling.cases",
            complete_manifest().replace("\"cases\": 1", "\"cases\": null"),
        ),
        (
            "sampling.seed",
            complete_manifest().replace("\"seed\": 0", "\"seed\": null"),
        ),
        (
            "sampling.shrinks",
            complete_manifest().replace("\"shrinks\": []", "\"shrinks\": null"),
        ),
        (
            "sampling.coverage",
            complete_manifest().replace("\"coverage\": {\"all\": 1}", "\"coverage\": null"),
        ),
    ];

    for (field, manifest) in cases {
        assert!(
            matches!(
                parse_intent_graph_json(&manifest),
                Err(ValidationInputError::Json(_))
            ),
            "null {field} must fail during JSON decoding"
        );
    }
}

#[test]
fn parse_manifest_rejects_duplicate_coverage_bucket_keys() {
    let manifest = complete_manifest().replace(
        "\"coverage\": {\"all\": 1}",
        "\"coverage\": {\"all\": 1, \"all\": 2}",
    );

    assert!(
        matches!(
            parse_intent_graph_json(&manifest),
            Err(ValidationInputError::Json(_))
        ),
        "duplicate coverage keys must fail during manifest decoding"
    );
}

#[test]
fn parse_manifest_rejects_empty_required_evidence_fields() {
    let empty_runner =
        complete_manifest().replace("\"runner\": \"validator-test\"", "\"runner\": \"\"");

    assert!(matches!(
        parse_intent_graph_json(&empty_runner),
        Err(ValidationInputError::Graph(_))
    ));
}

#[test]
fn parse_manifest_rejects_unicode_whitespace_only_required_evidence_fields() {
    let cases = [
        ("runner", "validator-test"),
        ("target", "host"),
        ("source_commit", "commit-1"),
        ("artifact_digest", "sha256:artifact"),
        ("generator", "fixture"),
        ("producer", "validator-test"),
        ("tool_version", "0.2"),
        ("timestamp", "2026-07-23T00:00:00Z"),
    ];

    for (field, original_value) in cases {
        let manifest = complete_manifest().replace(
            &format!("\"{field}\": \"{original_value}\""),
            &format!("\"{field}\": \"\u{00A0}\""),
        );

        assert!(
            matches!(
                parse_intent_graph_json(&manifest),
                Err(ValidationInputError::Graph(GraphError::InvalidEvidence {
                    source: EvidenceValidationError::EmptyField { field: actual }
                })) if actual == field
            ),
            "manifest field {field} must reject Unicode whitespace-only values"
        );
    }
}

#[test]
fn parse_manifest_rejects_unicode_whitespace_only_node_text() {
    let manifest = complete_manifest().replace(
        "\"text\": \"Users can cancel before shipment\"",
        "\"text\": \"\u{00A0}\"",
    );

    assert!(matches!(
        parse_intent_graph_json(&manifest),
        Err(ValidationInputError::Node(NodeTextError::EmptyText))
    ));
}

#[test]
fn parse_manifest_rejects_whitespace_only_coverage_bucket_before_registration() {
    let manifest =
        complete_manifest().replace("\"coverage\": {\"all\": 1}", "\"coverage\": {\"  \": 1}");

    assert!(matches!(
        parse_intent_graph_json(&manifest),
        Err(ValidationInputError::Graph(GraphError::InvalidEvidence {
            source: EvidenceValidationError::EmptyField { field: "coverage" }
        }))
    ));
}

#[test]
fn parse_manifest_rejects_unicode_whitespace_only_coverage_bucket_before_registration() {
    let manifest = complete_manifest().replace(
        "\"coverage\": {\"all\": 1}",
        "\"coverage\": {\"\u{00A0}\": 1}",
    );

    assert!(matches!(
        parse_intent_graph_json(&manifest),
        Err(ValidationInputError::Graph(GraphError::InvalidEvidence {
            source: EvidenceValidationError::EmptyField { field: "coverage" }
        }))
    ));
}

#[test]
fn parse_manifest_rejects_invalid_evidence_subject_kind() {
    let invalid_subject = complete_manifest().replace(
        "\"subject\": {\"kind\": \"claim\", \"namespace\": \"checkout\", \"key\": \"cancel-rejects-shipped\"}",
        "\"subject\": {\"kind\": \"evidence\", \"namespace\": \"checkout\", \"key\": \"review-001\"}",
    );

    assert!(matches!(
        parse_intent_graph_json(&invalid_subject),
        Err(ValidationInputError::MissingNodeReference {
            relation: "evidence.subject",
            ..
        })
    ));
}

#[test]
fn parse_manifest_reports_invalid_evaluates_subject_kind() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [],
      "evidence": [],
      "edges": [
        {
          "relation": "evaluates",
          "review": {"namespace": "checkout", "key": "reviewer-001"},
          "subject": {"kind": "contract", "namespace": "checkout", "key": "cancel-case"}
        }
      ]
    }
    "#;

    assert!(matches!(
        parse_intent_graph_json(manifest),
        Err(ValidationInputError::InvalidSubjectKind {
            relation: "evaluates.subject",
            kind: "contract",
            id,
        }) if id == "contract:checkout/cancel-case"
    ));
}

#[test]
fn parse_manifest_reports_invalid_invalidates_subject_kind() {
    let manifest = r#"
    {
      "schema_version": 1,
      "nodes": [],
      "evidence": [],
      "edges": [
        {
          "relation": "invalidates",
          "change": {"namespace": "checkout", "key": "api-v2"},
          "subject": {"kind": "claim", "namespace": "checkout", "key": "cancel-rejects-shipped"}
        }
      ]
    }
    "#;

    assert!(matches!(
        parse_intent_graph_json(manifest),
        Err(ValidationInputError::InvalidSubjectKind {
            relation: "invalidates.subject",
            kind: "claim",
            id,
        }) if id == "claim:checkout/cancel-rejects-shipped"
    ));
}

#[test]
fn manifest_output_round_trips_through_input_parser() {
    let graph = parse_intent_graph_json(complete_manifest()).expect("manifest should parse");
    let output = graph
        .to_manifest_json_string()
        .expect("graph should serialize");
    let decoded = parse_intent_graph_json(&output).expect("output should parse");

    assert_eq!(decoded, graph);
    assert_eq!(decoded.validate(), graph.validate());
}
