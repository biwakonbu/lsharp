mod review_registry_tests {
    use super::*;

    #[test]
    fn test_validate_tool_manifest_schema_declares_redacted_review_registry() {
        let response = handle_jsonrpc_message(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        }));
        let tool = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "lsharp_validate")
            .expect("lsharp_validate が tools/list に必要");
        let review_schema =
            &tool["outputSchema"]["properties"]["manifest"]["properties"]["reviews"];
        let input_review_schema =
            &tool["inputSchema"]["properties"]["manifest"]["oneOf"][0]["properties"]["reviews"];

        assert_eq!(review_schema["type"], "array");
        assert_eq!(input_review_schema["type"], "array");
        assert_eq!(
            review_schema["items"]["required"],
            json!(["namespace", "key", "provenance_digest", "visibility"])
        );
        assert_eq!(
            input_review_schema["items"]["required"],
            json!(["namespace", "key", "provenance_digest", "visibility"])
        );
        assert_eq!(review_schema["items"]["additionalProperties"], false);
        assert_eq!(input_review_schema["items"]["additionalProperties"], false);
        assert_eq!(
            review_schema["items"]["properties"]["visibility"]["enum"],
            json!(["public", "redacted"])
        );
        assert!(review_schema["items"]["properties"].get("author").is_none());
        assert!(review_schema["items"]["properties"].get("email").is_none());
        assert!(review_schema["items"]["properties"].get("body").is_none());
        assert!(input_review_schema["items"]["properties"]
            .get("author")
            .is_none());
        assert!(input_review_schema["items"]["properties"]
            .get("email")
            .is_none());
        assert!(input_review_schema["items"]["properties"]
            .get("body")
            .is_none());
    }

    #[test]
    fn test_validate_tool_manifest_input_schema_declares_versioned_graph_fields() {
        let response = handle_jsonrpc_message(&json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }));
        let tool = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "lsharp_validate")
            .expect("lsharp_validate が tools/list に必要");
        let manifest_schema = &tool["inputSchema"]["properties"]["manifest"]["oneOf"][0];

        assert_eq!(
            manifest_schema["required"],
            json!(["schema_version", "nodes", "evidence", "edges"])
        );
        assert_eq!(manifest_schema["additionalProperties"], false);
        assert_eq!(manifest_schema["properties"]["schema_version"]["const"], 1);
        assert_eq!(manifest_schema["properties"]["nodes"]["type"], "array");
        assert_eq!(manifest_schema["properties"]["evidence"]["type"], "array");
        assert_eq!(manifest_schema["properties"]["edges"]["type"], "array");
        assert_eq!(manifest_schema["properties"]["reviews"]["type"], "array");
    }

    #[test]
    fn test_validate_tool_manifest_schema_declares_typed_edges_and_subjects() {
        let response = handle_jsonrpc_message(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/list"
        }));
        let tool = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "lsharp_validate")
            .expect("lsharp_validate が tools/list に必要");
        let input_manifest = &tool["inputSchema"]["properties"]["manifest"]["oneOf"][0];
        let output_manifest = &tool["outputSchema"]["properties"]["manifest"];

        assert_eq!(input_manifest, output_manifest);
        let edge_variants = input_manifest["properties"]["edges"]["items"]["oneOf"]
            .as_array()
            .expect("edges は relation-specific oneOf を公開するべき");
        assert_eq!(edge_variants.len(), 6);
        assert_eq!(
            edge_variants[0]["properties"]["relation"]["const"],
            "motivates"
        );
        assert_eq!(
            edge_variants[1]["properties"]["relation"]["const"],
            "constrained-by"
        );
        assert_eq!(
            edge_variants[2]["properties"]["relation"]["const"],
            "tested-by"
        );
        assert_eq!(
            edge_variants[3]["properties"]["relation"]["enum"],
            json!(["supports", "contradicts"])
        );
        assert_eq!(
            edge_variants[4]["properties"]["relation"]["const"],
            "evaluates"
        );
        assert_eq!(
            edge_variants[5]["properties"]["relation"]["const"],
            "invalidates"
        );

        let id_schema = &edge_variants[0]["properties"]["intent"];
        assert_eq!(id_schema["required"], json!(["namespace", "key"]));
        assert_eq!(
            id_schema["properties"]["namespace"]["pattern"],
            "^[A-Za-z0-9_.-]+$"
        );
        assert_eq!(
            id_schema["properties"]["key"]["pattern"],
            "^[A-Za-z0-9_.-]+$"
        );

        assert_eq!(
            input_manifest["properties"]["evidence"]["items"]["properties"]["subject"]
                ["properties"]["kind"]["enum"],
            json!(["intent", "claim", "contract"])
        );
        assert_eq!(
            edge_variants[4]["properties"]["subject"]["properties"]["kind"]["enum"],
            json!(["intent", "claim", "evidence"])
        );
        assert_eq!(
            edge_variants[5]["properties"]["subject"]["properties"]["kind"]["enum"],
            json!(["evidence", "review"])
        );
    }

    #[test]
    fn test_manifest_schemas_use_draft202012_validator_for_valid_and_invalid_fixtures() {
        let response = handle_jsonrpc_message(&json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/list"
        }));
        let tool = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "lsharp_validate")
            .expect("lsharp_validate が tools/list に必要");
        let input_schema = &tool["inputSchema"];
        let output_schema = &tool["outputSchema"];
        let canonical_schema: Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/intent-graph.schema.json"
        ))
        .expect("canonical intent graph schema は JSON として読めるべき");
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/validation/ec-m3-canonical-manifest.json"
        ))
        .expect("canonical manifest fixture は JSON として読めるべき");

        jsonschema::draft202012::meta::validate(&canonical_schema)
            .expect("canonical schema は Draft 2020-12 meta-schema に適合するべき");
        let canonical_validator = jsonschema::draft202012::new(&canonical_schema)
            .expect("canonical schema の validator を構築できるべき");
        let input_validator = jsonschema::draft202012::new(input_schema)
            .expect("MCP input schema の validator を構築できるべき");
        let output_validator = jsonschema::draft202012::new(output_schema)
            .expect("MCP output schema の validator を構築できるべき");

        assert!(
            canonical_validator.is_valid(&fixture),
            "canonical fixture は canonical schema に適合するべき"
        );
        let valid_input = json!({ "manifest": fixture.clone() });
        let valid_output = json!({
            "status": "pass",
            "trace_gaps": [],
            "open_questions": 0,
            "independent_reviews": 1,
            "contradicting_observations": 0,
            "stale_reviews": 0,
            "stale_evidence": 0,
            "manifest": fixture.clone()
        });
        assert!(
            input_validator.is_valid(&valid_input),
            "canonical fixture は MCP input schema に適合するべき"
        );
        assert!(
            output_validator.is_valid(&valid_output),
            "canonical fixture は MCP output schema に適合するべき"
        );

        let mut fractional = fixture.clone();
        fractional["nodes"][0]["span"]["start"] = json!(0.5);
        let mut null = fixture.clone();
        null["evidence"][0]["execution"]["sampling"]["seed"] = Value::Null;
        let mut overflow = fixture.clone();
        overflow["evidence"][0]["execution"]["sampling"]["cases"] =
            serde_json::from_str("18446744073709551616")
                .expect("overflow number は JSON として読めるべき");
        let mut invalid_subject = fixture.clone();
        invalid_subject["edges"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "relation": "evaluates",
                "review": { "namespace": "checkout", "key": "reviewer-001" },
                "subject": {
                    "kind": "review",
                    "namespace": "checkout",
                    "key": "reviewer-001"
                }
            }));

        for (label, manifest) in [
            ("fractional", fractional),
            ("null", null),
            ("overflow", overflow),
            ("invalid-subject-kind", invalid_subject),
        ] {
            assert!(
                !canonical_validator.is_valid(&manifest),
                "{label}: canonical schema は不正 manifest を拒否するべき"
            );
            assert!(
                !input_validator.is_valid(&json!({ "manifest": manifest.clone() })),
                "{label}: MCP input schema は不正 manifest を拒否するべき"
            );
            assert!(
                !output_validator.is_valid(&json!({
                    "status": "pass",
                    "trace_gaps": [],
                    "open_questions": 0,
                    "independent_reviews": 1,
                    "contradicting_observations": 0,
                    "stale_reviews": 0,
                    "stale_evidence": 0,
                    "manifest": manifest
                })),
                "{label}: MCP output schema は不正 manifest を拒否するべき"
            );
        }

        for field in ["producer", "tool_version", "timestamp"] {
            let mut blank_provenance = fixture.clone();
            blank_provenance["evidence"][0]["provenance"][field] = json!("");
            assert!(
                !canonical_validator.is_valid(&blank_provenance),
                "blank {field}: canonical schema は空 provenance field を拒否するべき"
            );
            assert!(
                !input_validator.is_valid(&json!({ "manifest": blank_provenance.clone() })),
                "blank {field}: MCP input schema は空 provenance field を拒否するべき"
            );
            assert!(
                !output_validator.is_valid(&json!({
                    "status": "pass",
                    "trace_gaps": [],
                    "open_questions": 0,
                    "independent_reviews": 1,
                    "contradicting_observations": 0,
                    "stale_reviews": 0,
                    "stale_evidence": 0,
                    "manifest": blank_provenance
                })),
                "blank {field}: MCP output schema は空 provenance field を拒否するべき"
            );
        }
    }

    #[test]
    fn test_mcp_validation_report_conforms_to_ref_resolved_intent_validation_schema() {
        let report_schema: Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/intent-validation.schema.json"
        ))
        .expect("intent validation schema は JSON として読めるべき");
        let manifest_schema: Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/intent-graph.schema.json"
        ))
        .expect("intent graph schema は JSON として読めるべき");
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/validation/ec-m3-canonical-manifest.json"
        ))
        .expect("canonical manifest fixture は JSON として読めるべき");
        jsonschema::draft202012::meta::validate(&report_schema)
            .expect("intent validation schema は Draft 2020-12 meta-schema に適合するべき");

        let manifest_resource = jsonschema::Resource::from_contents(manifest_schema)
            .expect("intent graph schema を resource として登録できるべき");
        let validator = jsonschema::draft202012::options()
            .with_resource(
                "https://lsharp.dev/schemas/intent-graph.schema.json",
                manifest_resource,
            )
            .build(&report_schema)
            .expect("intent validation schema の $ref を解決できるべき");
        let report = call_tool(
            "lsharp_validate",
            &json!({
                "manifest": fixture,
                "include_manifest": true
            }),
        )
        .expect("MCP validation report を生成できるべき");

        validator.validate(&report).unwrap_or_else(|error| {
            panic!("MCP validation report が schema に適合しない: {error}")
        });
        let mut invalid_report = report;
        invalid_report["status"] = json!("invalid-status");
        assert!(
            !validator.is_valid(&invalid_report),
            "未知の report status は schema validator で拒否するべき"
        );
    }

    #[test]
    fn test_mcp_validation_output_schema_matches_report_boundaries() {
        let report_schema: Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/intent-validation.schema.json"
        ))
        .expect("intent validation schema は JSON として読めるべき");
        let manifest_schema: Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/intent-graph.schema.json"
        ))
        .expect("intent graph schema は JSON として読めるべき");
        let manifest_resource =
            jsonschema::Resource::from_contents(manifest_schema).expect("manifest schema");
        let canonical_validator = jsonschema::draft202012::options()
            .with_resource(
                "https://lsharp.dev/schemas/intent-graph.schema.json",
                manifest_resource,
            )
            .build(&report_schema)
            .expect("intent validation schema の $ref を解決できるべき");
        let response = handle_jsonrpc_message(&json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/list"
        }));
        let output_schema = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "lsharp_validate")
            .expect("lsharp_validate が tools/list に必要")["outputSchema"]
            .clone();
        jsonschema::draft202012::meta::validate(&output_schema)
            .expect("MCP output schema は Draft 2020-12 meta-schema に適合するべき");
        let output_validator = jsonschema::draft202012::new(&output_schema)
            .expect("MCP output schema の validator を構築できるべき");
        let report = call_tool(
            "lsharp_validate",
            &json!({
                "manifest": {
                    "schema_version": 1,
                    "nodes": [],
                    "evidence": [],
                    "edges": []
                },
                "include_manifest": true
            }),
        )
        .expect("MCP validation report を生成できるべき");
        assert!(
            canonical_validator.is_valid(&report),
            "canonical report schema は valid report を受理するべき"
        );
        assert!(
            output_validator.is_valid(&report),
            "MCP output schema は valid report を受理するべき"
        );

        let mut unknown_field = report.clone();
        unknown_field["unexpected"] = json!(true);
        let mut invalid_gap_code = report.clone();
        invalid_gap_code["trace_gaps"] = json!([{
            "code": "trace-gap.unknown",
            "subject_id": "intent:checkout/payments"
        }]);
        let mut empty_gap_subject = report.clone();
        empty_gap_subject["trace_gaps"] = json!([{
            "code": "trace-gap.intent-without-claim",
            "subject_id": ""
        }]);
        let mut overflow_count = report.clone();
        overflow_count["open_questions"] = serde_json::from_str("18446744073709551616")
            .expect("overflow report count は JSON として読めるべき");

        for (label, invalid_report) in [
            ("unknown-field", unknown_field),
            ("invalid-gap-code", invalid_gap_code),
            ("empty-gap-subject", empty_gap_subject),
            ("overflow-count", overflow_count),
        ] {
            assert!(
                !canonical_validator.is_valid(&invalid_report),
                "{label}: canonical report schema は不正 report を拒否するべき"
            );
            assert!(
                !output_validator.is_valid(&invalid_report),
                "{label}: MCP output schema は canonical report boundary と同じ reject をするべき"
            );
        }
    }

    #[test]
    fn test_validate_tool_manifest_schema_declares_unsigned_integer_boundaries() {
        let response = handle_jsonrpc_message(&json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list"
        }));
        let tool = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "lsharp_validate")
            .expect("lsharp_validate が tools/list に必要");
        let manifest_schema = &tool["inputSchema"]["properties"]["manifest"]["oneOf"][0];
        let node_span = &manifest_schema["properties"]["nodes"]["items"]["properties"]["span"];
        let sampling = &manifest_schema["properties"]["evidence"]["items"]["properties"]
            ["execution"]["properties"]["sampling"];

        for field in ["start", "end"] {
            assert_eq!(node_span["properties"][field]["type"], "integer");
            assert_eq!(node_span["properties"][field]["minimum"], 0);
            assert_eq!(node_span["properties"][field]["maximum"], u64::MAX);
        }
        for field in ["cases", "seed"] {
            assert_eq!(sampling["properties"][field]["type"], "integer");
            assert_eq!(sampling["properties"][field]["minimum"], 0);
            assert_eq!(sampling["properties"][field]["maximum"], u64::MAX);
        }
        assert_eq!(
            sampling["properties"]["shrinks"]["items"]["type"],
            "integer"
        );
        assert_eq!(sampling["properties"]["shrinks"]["items"]["minimum"], 0);
        assert_eq!(
            sampling["properties"]["shrinks"]["items"]["maximum"],
            u64::MAX
        );
        assert_eq!(
            sampling["properties"]["coverage"]["additionalProperties"]["type"],
            "integer"
        );
        assert_eq!(
            sampling["properties"]["coverage"]["additionalProperties"]["minimum"],
            0
        );
        assert_eq!(
            sampling["properties"]["coverage"]["additionalProperties"]["maximum"],
            u64::MAX
        );
    }

    #[test]
    fn test_validate_tool_rejects_unsigned_numeric_manifest_boundaries() {
        let fields = [
            ("span.start", "__SPAN_START__"),
            ("span.end", "__SPAN_END__"),
            ("sampling.cases", "__SAMPLING_CASES__"),
            ("sampling.seed", "__SAMPLING_SEED__"),
            ("sampling.shrinks[0]", "__SAMPLING_SHRINK__"),
            ("sampling.coverage.ok", "__SAMPLING_COVERAGE__"),
        ];
        let values = [
            ("fractional", "0.5"),
            ("null", "null"),
            ("overflow", "18446744073709551616"),
        ];

        for (field, marker) in fields {
            for (label, literal) in values {
                let manifest = numeric_manifest_with_literal(marker, literal);
                let response = handle_jsonrpc_message(&json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "tools/call",
                    "params": {
                        "name": "lsharp_validate",
                        "arguments": { "manifest": manifest }
                    }
                }));
                let error = response["result"]["content"][0]["text"]
                    .as_str()
                    .expect("MCP numeric boundary は text error を返すべき");

                assert_eq!(response["result"]["isError"], true);
                assert!(response["result"].get("structuredContent").is_none());
                assert!(
                    error.contains("validation manifest の parse に失敗しました:"),
                    "{field}={label}: unexpected error: {error}"
                );
            }
        }
    }

    #[test]
    fn test_validate_tool_rejects_unsigned_numeric_manifest_file_boundaries() {
        let fields = [
            ("span.start", "__SPAN_START__"),
            ("span.end", "__SPAN_END__"),
            ("sampling.cases", "__SAMPLING_CASES__"),
            ("sampling.seed", "__SAMPLING_SEED__"),
            ("sampling.shrinks[0]", "__SAMPLING_SHRINK__"),
            ("sampling.coverage.ok", "__SAMPLING_COVERAGE__"),
        ];
        let values = [
            ("fractional", "0.5"),
            ("null", "null"),
            ("overflow", "18446744073709551616"),
        ];

        for (field, marker) in fields {
            for (label, literal) in values {
                let manifest = numeric_manifest_with_literal(marker, literal);
                let path = std::env::temp_dir().join(format!(
                    "lsharp_mcp_manifest_file_numeric_{}_{}_{}.json",
                    std::process::id(),
                    field.replace(['.', '[', ']'], "-"),
                    label
                ));
                std::fs::write(&path, manifest).expect("manifest_file fixtureを書き込めるべき");
                let response = handle_jsonrpc_message(&json!({
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "tools/call",
                    "params": {
                        "name": "lsharp_validate",
                        "arguments": { "manifest_file": path.display().to_string() }
                    }
                }));
                std::fs::remove_file(&path).expect("manifest_file fixtureを削除できるべき");

                let error = response["result"]["content"][0]["text"]
                    .as_str()
                    .expect("MCP manifest_file numeric boundary は text error を返すべき");
                assert_eq!(response["result"]["isError"], true);
                assert!(response["result"].get("structuredContent").is_none());
                assert!(
                    error.contains("validation manifest の parse に失敗しました:"),
                    "{field}={label}: unexpected error: {error}"
                );
            }
        }
    }

    #[test]
    fn test_validate_tool_rejects_blank_manifest_provenance_fields() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/validation/ec-m3-canonical-manifest.json"
        ))
        .expect("canonical manifest fixture は JSON として読めるべき");
        let assert_error = |arguments: Value, route: &str, field: &str| {
            let response = handle_jsonrpc_message(&json!({
                "jsonrpc": "2.0",
                "id": 3,
                "method": "tools/call",
                "params": { "name": "lsharp_validate", "arguments": arguments }
            }));
            let error = response["result"]["content"][0]["text"]
                .as_str()
                .expect("manifest error は text を返すべき");
            assert_eq!(response["result"]["isError"], true, "{route}");
            assert!(response["result"].get("structuredContent").is_none());
            assert!(
                error.contains(field),
                "{route}/{field}: unexpected error: {error}"
            );
        };
        for field in ["producer", "tool_version", "timestamp"] {
            let mut manifest = fixture.clone();
            manifest["evidence"][0]["provenance"][field] = json!("");
            assert_error(json!({ "manifest": manifest.clone() }), "manifest", field);
            let path = std::env::temp_dir().join(format!(
                "lsharp_mcp_blank_provenance_{}_{}.json",
                std::process::id(),
                field
            ));
            std::fs::write(&path, serde_json::to_string(&manifest).unwrap())
                .expect("manifest_file fixtureを書き込めるべき");
            assert_error(
                json!({ "manifest_file": path.display().to_string() }),
                "manifest_file",
                field,
            );
            std::fs::remove_file(&path).expect("manifest_file fixtureを削除できるべき");
        }
    }

    #[test]
    fn test_validate_tool_rejects_duplicate_top_level_manifest_fields() {
        let duplicate = r#"{
            "schema_version": 1,
            "schema_version": 1,
            "nodes": [],
            "evidence": [],
            "edges": []
        }"#;
        let assert_error = |arguments: Value, route: &str| {
            let response = handle_jsonrpc_message(&json!({
                "jsonrpc": "2.0",
                "id": 4,
                "method": "tools/call",
                "params": { "name": "lsharp_validate", "arguments": arguments }
            }));
            let error = response["result"]["content"][0]["text"]
                .as_str()
                .expect("duplicate manifest key error は text を返すべき");
            assert_eq!(response["result"]["isError"], true, "{route}");
            assert!(response["result"].get("structuredContent").is_none());
            assert!(
                error.contains("validation manifest の parse に失敗しました:"),
                "{route}: unexpected error: {error}"
            );
            assert!(
                error.contains("duplicate"),
                "{route}: unexpected error: {error}"
            );
            assert!(
                error.contains("schema_version"),
                "{route}: unexpected error: {error}"
            );
        };

        assert_error(json!({ "manifest": duplicate }), "manifest");

        let path = std::env::temp_dir().join(format!(
            "lsharp_mcp_duplicate_top_level_{}.json",
            std::process::id()
        ));
        std::fs::write(&path, duplicate).expect("duplicate manifest_file fixtureを書き込めるべき");
        assert_error(
            json!({ "manifest_file": path.display().to_string() }),
            "manifest_file",
        );
        std::fs::remove_file(&path).expect("duplicate manifest_file fixtureを削除できるべき");
    }

    #[test]
    fn test_validate_tool_rejects_unknown_top_level_manifest_fields() {
        let object = json!({
            "schema_version": 1,
            "nodes": [],
            "evidence": [],
            "edges": [],
            "unexpected": true
        });
        let string = r#"{
            "schema_version": 1,
            "nodes": [],
            "evidence": [],
            "edges": [],
            "unexpected": true
        }"#;
        let assert_error = |arguments: Value, route: &str| {
            let response = handle_jsonrpc_message(&json!({
                "jsonrpc": "2.0",
                "id": 5,
                "method": "tools/call",
                "params": { "name": "lsharp_validate", "arguments": arguments }
            }));
            let error = response["result"]["content"][0]["text"]
                .as_str()
                .expect("unknown manifest field error は text を返すべき");
            assert_eq!(response["result"]["isError"], true, "{route}");
            assert!(response["result"].get("structuredContent").is_none());
            assert!(
                error.contains("validation manifest の parse に失敗しました:"),
                "{route}: unexpected error: {error}"
            );
            assert!(
                error.contains("unexpected"),
                "{route}: unexpected error: {error}"
            );
        };

        assert_error(json!({ "manifest": object }), "manifest-object");
        assert_error(json!({ "manifest": string }), "manifest-string");

        let path = std::env::temp_dir().join(format!(
            "lsharp_mcp_unknown_top_level_{}.json",
            std::process::id()
        ));
        std::fs::write(&path, string).expect("unknown manifest_file fixtureを書き込めるべき");
        assert_error(
            json!({ "manifest_file": path.display().to_string() }),
            "manifest-file",
        );
        std::fs::remove_file(&path).expect("unknown manifest_file fixtureを削除できるべき");
    }

    fn numeric_manifest_with_literal(marker: &str, literal: &str) -> String {
        let template = r#"{
            "schema_version": 1,
            "nodes": [{
                "kind": "intent",
                "namespace": "checkout",
                "key": "safe-cancel",
                "text": "Users can cancel",
                "span": {"start": "__SPAN_START__", "end": "__SPAN_END__"}
            }],
            "evidence": [{
                "namespace": "checkout",
                "key": "cancel-example",
                "method": "example",
                "subject": {"kind": "intent", "namespace": "checkout", "key": "safe-cancel"},
                "outcome": "pass",
                "execution": {
                    "runner": "mcp-test",
                    "target": "aarch64-apple-darwin",
                    "source_commit": "source-commit",
                    "artifact_digest": "sha256:artifact",
                    "sampling": {
                        "cases": "__SAMPLING_CASES__",
                        "seed": "__SAMPLING_SEED__",
                        "generator": "fixed",
                        "shrinks": ["__SAMPLING_SHRINK__"],
                        "coverage": {"ok": "__SAMPLING_COVERAGE__"}
                    }
                },
                "provenance": {
                    "producer": "mcp-test",
                    "tool_version": "0.1.0",
                    "timestamp": "2026-07-29T00:00:00Z"
                },
                "independence": "same-author"
            }],
            "edges": []
        }"#;
        [
            "__SPAN_START__",
            "__SPAN_END__",
            "__SAMPLING_CASES__",
            "__SAMPLING_SEED__",
            "__SAMPLING_SHRINK__",
            "__SAMPLING_COVERAGE__",
        ]
        .into_iter()
        .fold(template.to_string(), |manifest, candidate| {
            let replacement = if candidate == marker { literal } else { "0" };
            manifest.replace(&format!("\"{candidate}\""), replacement)
        })
    }

    #[test]
    fn test_validate_tool_includes_redacted_review_registry_without_private_fields() {
        let result = call_tool(
            "lsharp_validate",
            &json!({
                "manifest": {
                    "schema_version": 1,
                    "nodes": [],
                    "reviews": [{
                        "namespace": "checkout",
                        "key": "reviewer-001",
                        "provenance_digest": "sha256:review-provenance-001",
                        "visibility": "redacted"
                    }],
                    "evidence": [],
                    "edges": []
                },
                "include_manifest": true
            }),
        )
        .expect("MCP validation は redacted review registry を返すべき");

        let review = &result["manifest"]["reviews"][0];
        assert_eq!(review["namespace"], "checkout");
        assert_eq!(review["key"], "reviewer-001");
        assert_eq!(review["provenance_digest"], "sha256:review-provenance-001");
        assert_eq!(review["visibility"], "redacted");
        assert!(review.get("author").is_none());
        assert!(review.get("email").is_none());
        assert!(review.get("body").is_none());
    }

    #[test]
    fn test_validate_tool_rejects_unregistered_review_edge() {
        let error = call_tool(
            "lsharp_validate",
            &json!({
                "manifest": {
                    "schema_version": 1,
                    "nodes": [],
                    "reviews": [{
                        "namespace": "checkout",
                        "key": "reviewer-001",
                        "provenance_digest": "sha256:review-provenance-001",
                        "visibility": "redacted"
                    }],
                    "evidence": [],
                    "edges": [{
                        "relation": "invalidates",
                        "change": {"namespace": "checkout", "key": "api-v2"},
                        "subject": {
                            "kind": "review",
                            "namespace": "checkout",
                            "key": "missing-review"
                        }
                    }]
                },
                "include_manifest": true
            }),
        )
        .expect_err("未登録 review edge は MCP で拒否するべき");

        assert!(error.contains("review ID"), "unexpected error: {error}");
        assert!(
            error.contains("missing-review"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn test_validate_tool_rejects_unregistered_review_edge_with_explicit_empty_registry() {
        let error = call_tool(
            "lsharp_validate",
            &json!({
                "manifest": {
                    "schema_version": 1,
                    "nodes": [{
                        "kind": "intent",
                        "namespace": "checkout",
                        "key": "safe-cancel",
                        "text": "Users can cancel"
                    }],
                    "reviews": [],
                    "evidence": [],
                    "edges": [{
                        "relation": "evaluates",
                        "review": {"namespace": "checkout", "key": "reviewer-001"},
                        "subject": {
                            "kind": "intent",
                            "namespace": "checkout",
                            "key": "safe-cancel"
                        }
                    }]
                }
            }),
        )
        .expect_err("明示 empty review registry は未登録 edge を MCP で拒否するべき");

        assert!(error.contains("review ID"), "unexpected error: {error}");
        assert!(error.contains("reviewer-001"), "unexpected error: {error}");
    }

    #[test]
    fn test_validate_tool_allows_opaque_review_edge_when_registry_is_omitted() {
        let result = call_tool(
            "lsharp_validate",
            &json!({
                "manifest": {
                    "schema_version": 1,
                    "nodes": [{
                        "kind": "intent",
                        "namespace": "checkout",
                        "key": "safe-cancel",
                        "text": "Users can cancel"
                    }],
                    "evidence": [],
                    "edges": [{
                        "relation": "evaluates",
                        "review": {"namespace": "checkout", "key": "reviewer-001"},
                        "subject": {
                            "kind": "intent",
                            "namespace": "checkout",
                            "key": "safe-cancel"
                        }
                    }]
                },
                "include_manifest": true
            }),
        )
        .expect("registry 省略時は opaque review endpoint を MCP で受理するべき");

        assert_eq!(result["status"], "unknown");
        assert!(result["manifest"].get("reviews").is_none());
    }
}
