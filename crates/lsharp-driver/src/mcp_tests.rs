#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_tools_contains_phase12_mvp_tools() {
        let tools = list_tools();
        let names: Vec<&str> = tools.iter().map(|tool| tool.name.as_str()).collect();

        assert!(names.contains(&"lsharp_check"));
        assert!(names.contains(&"lsharp_hover"));
        assert!(names.contains(&"lsharp_completion"));
        assert!(names.contains(&"lsharp_package_api"));
        assert!(names.contains(&"lsharp_stdlib_api"));
    }

    #[test]
    fn test_error_tool_input_schema_requires_error_code() {
        let schema = tool_input_schema("lsharp_errors");

        assert_eq!(schema["required"], json!(["error_code"]));
    }

    #[test]
    fn test_initialize_response_advertises_mcp_protocol_and_tools_capability() {
        let response = handle_jsonrpc_message(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize"
        }));

        assert_eq!(response["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);
        assert_eq!(
            response["result"]["capabilities"]["tools"]["listChanged"],
            false
        );
        assert_eq!(response["result"]["serverInfo"]["name"], "lsharp");
    }

    #[test]
    fn test_call_hover_tool_returns_doc_and_type() {
        let source = r#"
(defn add
  [x y]
  :doc "加算"
  (+ x y))

(defn main []
  (add 1 2))
"#;
        let result = call_tool(
            "lsharp_hover",
            &json!({
                "source": source,
                "line": 7,
                "character": 3
            }),
        )
        .expect("hover tool should succeed");

        assert_eq!(result["name"], "add");
        assert_eq!(result["type"], "Int -> Int -> Int");
        assert_eq!(result["doc"], "加算");
    }

    #[test]
    fn test_handle_jsonrpc_tools_list_request_returns_mcp_result() {
        let response = handle_jsonrpc_message(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        }));

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response["result"]["tools"].is_array());
    }

    #[test]
    fn test_validate_tool_declares_source_input_and_report_output_schema() {
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

        assert_eq!(tool["inputSchema"]["type"], "object");
        assert_eq!(
            tool["inputSchema"]["oneOf"],
            json!([
                { "required": ["source"] },
                { "required": ["file"] },
                { "required": ["manifest"] },
                { "required": ["manifest_file"] }
            ])
        );
        assert_eq!(
            tool["outputSchema"]["required"],
            json!([
                "status",
                "trace_gaps",
                "open_questions",
                "independent_reviews",
                "contradicting_observations",
                "stale_reviews",
                "stale_evidence"
            ])
        );
        assert_eq!(
            tool["outputSchema"]["properties"]["status"]["enum"],
            json!(["pass", "fail", "unknown"])
        );
        assert_eq!(
            tool["outputSchema"]["properties"]["review_verifications"]["items"]["properties"]["state"]
                ["enum"],
            json!(["verified", "unverified", "stale", "revoked"])
        );
        assert_eq!(
            tool["outputSchema"]["properties"]["manifest"]["type"],
            "object"
        );
    }

    #[test]
    fn test_validate_tool_projects_source_to_fact_oriented_report() {
        let result = call_tool(
            "lsharp_validate",
            &json!({
                "source": r#"
                    (defn cancel []
                      :intent "intent:checkout/safe-cancel" "Users can cancel an order"
                      :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
                      true)
                "#,
            }),
        )
        .expect("lsharp_validate は source graph report を返すべき");

        assert_eq!(result["status"], "unknown");
        assert!(result["trace_gaps"].is_array());
        assert_eq!(result["open_questions"], 0);
        assert_eq!(result["independent_reviews"], 0);
        assert_eq!(result["contradicting_observations"], 0);
        assert_eq!(result["stale_reviews"], 0);
        assert_eq!(result["stale_evidence"], 0);
        assert!(result.get("verified").is_none());
    }

    #[test]
    fn test_validate_tool_is_available_through_jsonrpc_tools_call() {
        let response = handle_jsonrpc_message(&json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "lsharp_validate",
                "arguments": {
                    "source": "(defn main [] true)"
                }
            }
        }));

        assert_eq!(response["result"]["isError"], false);
        assert_eq!(response["result"]["structuredContent"]["status"], "unknown");
    }

    #[test]
    fn test_validate_tool_accepts_file_input() {
        let path = std::env::temp_dir().join("lsharp_mcp_validate_file.ls");
        std::fs::write(&path, "(defn main [] true)").unwrap();

        let result = call_tool(
            "lsharp_validate",
            &json!({ "file": path.display().to_string() }),
        )
        .expect("lsharp_validate は file 入力を受け付けるべき");

        assert_eq!(result["status"], "unknown");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_validate_tool_reports_source_parse_errors() {
        let error = call_tool("lsharp_validate", &json!({ "source": "(" }))
            .expect_err("不正な source は MCP エラーになるべき");

        assert!(error.starts_with("validation source の parse に失敗しました:"));
    }

    #[test]
    fn test_validate_tool_declares_manifest_inputs() {
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

        assert_eq!(
            tool["inputSchema"]["oneOf"],
            json!([
                { "required": ["source"] },
                { "required": ["file"] },
                { "required": ["manifest"] },
                { "required": ["manifest_file"] }
            ])
        );
        assert_eq!(
            tool["inputSchema"]["properties"]["manifest"]["oneOf"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            tool["inputSchema"]["properties"]["manifest_file"]["type"],
            "string"
        );
        assert_eq!(
            tool["inputSchema"]["properties"]["include_manifest"]["type"],
            "boolean"
        );
        assert_eq!(
            tool["inputSchema"]["properties"]["trust_store"]["type"],
            "string"
        );
        assert_eq!(
            tool["inputSchema"]["properties"]["review_lifecycle"]["type"],
            "string"
        );
    }

    #[test]
    fn test_validate_tool_rejects_review_input_outside_project_root() {
        let error = call_tool(
            "lsharp_validate",
            &json!({
                "manifest": {
                    "schema_version": 1,
                    "nodes": [],
                    "evidence": [],
                    "edges": []
                },
                "trust_store": "../outside-review-wire.json"
            }),
        )
        .expect_err("project root 外の trust store は拒否するべき");

        assert!(error.contains("project root"), "unexpected error: {error}");
        assert!(error.contains("trust store"), "unexpected error: {error}");
    }

    #[test]
    fn test_validate_tool_accepts_manifest_input() {
        let manifest = json!({
            "schema_version": 1,
            "nodes": [],
            "evidence": [],
            "edges": []
        });
        let result = call_tool("lsharp_validate", &json!({ "manifest": manifest }))
            .expect("lsharp_validate は manifest object を受け付けるべき");

        assert_eq!(result["status"], "unknown");
        assert!(result["trace_gaps"].is_array());
        assert!(result.get("verified").is_none());

        let manifest_text = r#"{"schema_version":1,"nodes":[],"evidence":[],"edges":[]}"#;
        let string_result = call_tool("lsharp_validate", &json!({ "manifest": manifest_text }))
            .expect("lsharp_validate は manifest JSON string も受け付けるべき");
        assert_eq!(string_result["status"], "unknown");
    }

    #[test]
    fn test_validate_tool_accepts_manifest_file_input() {
        let path = std::env::temp_dir().join("lsharp_mcp_validate_manifest.json");
        std::fs::write(
            &path,
            r#"{"schema_version":1,"nodes":[],"evidence":[],"edges":[]}"#,
        )
        .unwrap();

        let result = call_tool(
            "lsharp_validate",
            &json!({ "manifest_file": path.display().to_string() }),
        )
        .expect("lsharp_validate は manifest_file 入力を受け付けるべき");

        assert_eq!(result["status"], "unknown");
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn test_validate_tool_reports_manifest_input_errors() {
        let error = call_tool(
            "lsharp_validate",
            &json!({
                "manifest": {
                    "schema_version": 99,
                    "nodes": [],
                    "evidence": [],
                    "edges": []
                }
            }),
        )
        .expect_err("未対応 schema_version は MCP エラーになるべき");

        assert!(error.contains("validation manifest の parse に失敗しました:"));
        assert!(error.contains("schema_version 99"));
    }

    #[test]
    fn test_validate_tool_rejects_multiple_input_kinds() {
        let error = call_tool(
            "lsharp_validate",
            &json!({
                "source": "(defn main [] true)",
                "manifest": {
                    "schema_version": 1,
                    "nodes": [],
                    "evidence": [],
                    "edges": []
                }
            }),
        )
        .expect_err("複数の validation input は拒否するべき");

        assert!(error.contains("いずれか一つが必要です"));
    }

    #[test]
    fn test_validation_graph_rejects_missing_input_before_parsing() {
        let error = validation_graph(&json!({}))
            .expect_err("validation input がない場合は parse 前に拒否するべき");

        assert_eq!(
            error,
            "lsharp_validate は source、file、manifest、manifest_file のいずれか一つが必要です"
        );
    }

    #[test]
    fn test_validate_tool_manifest_is_available_through_jsonrpc_tools_call() {
        let response = handle_jsonrpc_message(&json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "lsharp_validate",
                "arguments": {
                    "manifest": {
                        "schema_version": 1,
                        "nodes": [],
                        "evidence": [],
                        "edges": []
                    }
                }
            }
        }));

        assert_eq!(response["result"]["isError"], false);
        assert_eq!(response["result"]["structuredContent"]["status"], "unknown");
    }

    #[test]
    fn test_validate_tool_can_include_canonical_manifest() {
        let result = call_tool(
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
        .expect("include_manifest は canonical manifest を返すべき");

        assert_eq!(result["status"], "unknown");
        assert_eq!(result["manifest"]["schema_version"], 1);
        assert!(result["manifest"]["nodes"].is_array());
        assert!(result["manifest"]["evidence"].is_array());
        assert!(result["manifest"]["edges"].is_array());
    }

    #[test]
    fn test_validate_tool_projects_source_into_canonical_manifest() {
        let result = call_tool(
            "lsharp_validate",
            &json!({
                "source": r#"
                    (defn cancel []
                      :intent "intent:checkout/safe-cancel" "Users can cancel an order"
                      :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
                      true)
                "#,
                "include_manifest": true
            }),
        )
        .expect("source validation は canonical manifest を返すべき");

        assert_eq!(result["manifest"]["schema_version"], 1);
        assert_eq!(result["manifest"]["nodes"].as_array().unwrap().len(), 2);
        assert!(result["manifest"]["edges"].is_array());
    }

    #[test]
    fn test_validate_tool_rejects_invalid_include_manifest_option() {
        let error = call_tool(
            "lsharp_validate",
            &json!({
                "source": "(defn main [] true)",
                "include_manifest": "yes"
            }),
        )
        .expect_err("include_manifest は boolean 以外を拒否するべき");

        assert!(error.contains("include_manifest は boolean が必要です"));
    }

    #[test]
    fn test_project_context_tool_honors_explicit_project_dir() {
        let dir = std::env::temp_dir().join("lsharp_mcp_project_context");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("lsharp.toml"),
            r#"
[project]
name = "context-demo"
version = "1.2.3"
description = "context fixture"
"#,
        )
        .unwrap();

        let result = call_tool(
            "lsharp_project_context",
            &json!({ "project_dir": dir.display().to_string() }),
        )
        .expect("project_context が明示 project_dir を読むべき");

        assert_eq!(result["project"]["name"], "context-demo");
        assert_eq!(result["project"]["version"], "1.2.3");
        assert_eq!(result["project"]["description"], "context fixture");
        assert!(result["dependencies"].is_array());
        assert!(result["installedPackages"].is_array());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_package_api_tool_reads_installed_api_json() {
        let dir = std::env::temp_dir().join("lsharp_mcp_package_api");
        let _ = std::fs::remove_dir_all(&dir);
        let package_dir = dir.join(".lsharp/packages/demo-12345678");
        std::fs::create_dir_all(package_dir.join("docs")).unwrap();
        std::fs::write(
            package_dir.join("docs/api.json"),
            r#"{
  "package": "demo",
  "version": "0.1.0",
  "modules": [
    {
      "name": "Geometry",
      "doc": null,
      "functions": [],
      "types": []
    }
  ]
}"#,
        )
        .unwrap();

        let result = call_tool(
            "lsharp_package_api",
            &json!({
                "project_dir": dir.display().to_string(),
                "name": "demo"
            }),
        )
        .expect("package_api が成功するべき");

        assert_eq!(result["package"], "demo");
        assert_eq!(result["modules"][0]["name"], "Geometry");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_stdlib_api_tool_returns_stdlib_modules_with_metadata() {
        let result = call_tool("lsharp_stdlib_api", &json!({})).expect("stdlib_api が成功するべき");
        let modules = result["modules"]
            .as_array()
            .expect("modules は配列であるべき");

        assert!(modules.len() >= 11, "stdlib module 数が不足している");

        let core = modules
            .iter()
            .find(|module| module["name"] == "Core")
            .expect("Core module が必要");
        let functions = core["functions"]
            .as_array()
            .expect("functions は配列であるべき");
        let abs = functions
            .iter()
            .find(|function| function["name"] == "abs")
            .expect("Core.abs が必要");

        assert!(core["doc"].is_string());
        assert_eq!(abs["doc"], "整数の絶対値を返す。");
        assert_eq!(abs["params"][0]["doc"], "対象の整数");
        assert_eq!(abs["returns"]["doc"], "x の絶対値");
    }

    #[test]
    fn test_errors_tool_returns_ls_error_code_reference_and_legacy_alias() {
        let result = call_tool("lsharp_errors", &json!({"error_code": "LS1001"}))
            .expect("lsharp_errors は LS#### を返すべき");

        assert_eq!(result["code"], "LS1001");
        assert_eq!(result["name"], "undefined-variable");
        assert_eq!(result["legacy_code"], "E0001");
        assert!(
            result["doc"]
                .as_str()
                .is_some_and(|doc| doc.contains("docs/guides/error-reference.md")),
            "error reference docs への導線が必要"
        );
    }

    #[test]
    fn test_errors_tool_requires_error_code() {
        let error = errors_tool(&json!({})).expect_err("error_code なしでは診断を返せない");

        assert_eq!(error, "error_code が必要です");
    }

    #[test]
    fn test_errors_tool_accepts_legacy_error_code_alias() {
        let result = call_tool("lsharp_errors", &json!({"error_code": "E0001"}))
            .expect("legacy E0001 は LS1001 へ解決するべき");

        assert_eq!(result["code"], "LS1001");
        assert_eq!(result["legacy_code"], "E0001");

        let branch_mismatch = call_tool("lsharp_errors", &json!({"error_code": "E0003"}))
            .expect("legacy E0003 は LS1002 へ解決するべき");
        assert_eq!(branch_mismatch["code"], "LS1002");
    }

    #[test]
    fn test_errors_tool_returns_macro_expansion_code_reference() {
        let result = call_tool("lsharp_errors", &json!({"error_code": "LS0201"}))
            .expect("LS0201 は macro expansion error として解決するべき");

        assert_eq!(result["code"], "LS0201");
        assert_eq!(result["name"], "macro-expansion-error");
    }

    #[test]
    fn test_errors_tool_returns_empty_executable_contract_code() {
        let result = call_tool("lsharp_errors", &json!({"error_code": "LS2004"}))
            .expect("LS2004 は空の executable contract として解決するべき");

        assert_eq!(result["code"], "LS2004");
        assert_eq!(result["name"], "empty-executable-contract");
    }

    #[test]
    fn test_errors_tool_returns_vacuous_contract_code() {
        let result = call_tool("lsharp_errors", &json!({"error_code": "LS2005"}))
            .expect("LS2005 は vacuous contract として解決するべき");

        assert_eq!(result["code"], "LS2005");
        assert_eq!(result["name"], "vacuous-contract");
    }

    #[test]
    fn test_check_tool_returns_diagnostic_code_and_source_range() {
        let result = call_tool("lsharp_check", &json!({"source": "(unknown-form)"}))
            .expect("lsharp_check は syntax diagnostics を返すべき");
        let diagnostic = &result["diagnostics"][0];

        assert_eq!(diagnostic["code"], "LS0103");
        assert_eq!(diagnostic["range"]["start"]["line"], 0);
        assert_eq!(diagnostic["range"]["start"]["character"], 1);
        assert_eq!(diagnostic["range"]["end"]["character"], 13);
    }

    #[test]
    fn test_mcp_file_input_preserves_driver_io_error_code() {
        let path = std::env::temp_dir().join("lsharp_mcp_missing_source.ls");
        let _ = std::fs::remove_file(&path);

        let error = call_tool("lsharp_check", &json!({"file": path.display().to_string()}))
            .expect_err("存在しない MCP source file は失敗するべき");

        assert!(
            error.starts_with("[LS5001]"),
            "MCP の driver I/O 診断コードを保持するべき: {error}"
        );
    }

    #[test]
    fn test_check_tool_reports_legacy_migration_enum_strings() {
        let source = "(defn succ [x] :example [(succ 0) (= (succ 1) 2)] :invariant (= result (+ x 1)) (+ x 1))";
        let result = call_tool("lsharp_check", &json!({"source": source}))
            .expect("lsharp_check は legacy migration report を返すべき");

        assert_eq!(result["ok"], true);
        assert_eq!(result["diagnostics"], json!([]));
        let migration = result["migrationDiagnostics"]
            .as_array()
            .expect("migrationDiagnostics は配列であるべき");
        assert_eq!(migration.len(), 3);
        assert_eq!(migration[0]["code"], "LS2001");
        assert_eq!(migration[0]["owner"], "succ");
        assert_eq!(
            migration[0]["selectedSemantics"],
            "legacy-example-truthiness"
        );
        assert_eq!(migration[0]["disposition"], "docs-only-example");
        assert_eq!(migration[0]["range"]["start"]["line"], 0);
        assert_eq!(migration[0]["range"]["start"]["character"], 25);
        assert_eq!(migration[0]["range"]["end"]["character"], 33);
        assert_eq!(migration[1]["disposition"], "assertion");
        assert_eq!(migration[2]["code"], "LS2002");
        assert_eq!(
            migration[2]["selectedSemantics"],
            "legacy-invariant-deterministic-smoke"
        );
        assert_eq!(migration[2]["disposition"], "property-postcondition");
        assert_eq!(migration[2]["range"]["start"]["character"], 61);
        assert_eq!(migration[2]["range"]["end"]["character"], 79);
    }

    #[test]
    fn test_check_tool_declares_legacy_migration_output_schema() {
        let response = handle_jsonrpc_message(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        }));
        let tool = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "lsharp_check")
            .expect("lsharp_check が tools/list に必要");
        let migration = &tool["outputSchema"]["properties"]["migrationDiagnostics"];

        assert_eq!(migration["type"], "array");
        assert_eq!(
            migration["items"]["properties"]["selectedSemantics"]["enum"],
            json!([
                "legacy-example-truthiness",
                "legacy-invariant-deterministic-smoke"
            ])
        );
        assert_eq!(
            migration["items"]["properties"]["disposition"]["enum"],
            json!([
                "docs-only-example",
                "assertion",
                "property-postcondition",
                "manual-review"
            ])
        );
    }

    #[test]
    fn test_error_reference_doc_mentions_all_mcp_error_codes() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let doc =
            std::fs::read_to_string(repo_root.join("docs/guides/error-reference.md")).unwrap();

        for entry in crate::error_codes::ERROR_CODES {
            assert!(
                doc.contains(entry.code),
                "error-reference.md に {} が必要",
                entry.code
            );
        }
    }

    #[test]
    fn test_compile_run_tool_requires_source_or_file() {
        let error = compile_run_tool(&json!({}))
            .expect_err("compile_run は source または file なしでは実行できない");

        assert_eq!(error, "source または file が必要です");
    }

    #[test]
    fn test_compile_run_temp_dirs_are_unique_and_cleaned_up() {
        let first =
            new_compile_run_temp_dir().expect("最初の compile_run 一時ディレクトリを作れる");
        let first_path = first.path.clone();
        let second =
            new_compile_run_temp_dir().expect("2つ目の compile_run 一時ディレクトリを作れる");

        assert_ne!(first.path, second.path);
        assert!(first.path.is_dir());
        assert!(second.path.is_dir());

        drop(first);
        assert!(!first_path.exists());
    }

    #[test]
    fn test_compile_run_tool_uses_wasi_default_for_wasm_output() {
        let result = compile_run_tool(&json!({
            "source": "(defn main [] (print 42))\n",
        }))
        .expect("compile_run が成功するべき");

        assert_eq!(result["ok"], true);
        assert_eq!(result["exit_code"], 0);
        assert!(
            result["stdout"]
                .as_str()
                .is_some_and(|stdout| stdout.contains("42")),
            "WASI 実行結果に 42 が含まれるべき"
        );
    }

    #[test]
    fn test_mcp_test_module_keeps_private_protocol_helper_access() {
        let response = jsonrpc_result(json!(7), json!({"ok": true}));

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 7);
        assert_eq!(response["result"]["ok"], true);
    }

    include!("mcp_review_registry_tests.rs");
}
