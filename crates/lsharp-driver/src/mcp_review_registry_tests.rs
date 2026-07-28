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
        }
        for field in ["cases", "seed"] {
            assert_eq!(sampling["properties"][field]["type"], "integer");
            assert_eq!(sampling["properties"][field]["minimum"], 0);
        }
        assert_eq!(
            sampling["properties"]["shrinks"]["items"]["type"],
            "integer"
        );
        assert_eq!(sampling["properties"]["shrinks"]["items"]["minimum"], 0);
        assert_eq!(
            sampling["properties"]["coverage"]["additionalProperties"]["type"],
            "integer"
        );
        assert_eq!(
            sampling["properties"]["coverage"]["additionalProperties"]["minimum"],
            0
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
