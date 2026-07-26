use crate::{api_doc, commands, config, error_codes};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

include!("mcp_protocol.rs");
include!("mcp_validation.rs");
include!("mcp_context.rs");
include!("mcp_compile.rs");

fn mcp_io_error(path: impl std::fmt::Display, error: impl std::fmt::Display) -> String {
    format!(
        "[{}] {}: {}",
        error_codes::DRIVER_IO_ERROR_CODE,
        path,
        error
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpTool {
    pub name: String,
    pub description: String,
}

pub fn list_tools() -> Vec<McpTool> {
    vec![
        tool("lsharp_check", "L# source を型チェックする"),
        tool(
            "lsharp_validate",
            "L# source の intent/evidence graph を fact-oriented に検証する",
        ),
        tool("lsharp_hover", "カーソル位置の型と :doc を返す"),
        tool("lsharp_completion", "補完候補を返す"),
        tool("lsharp_format", "L# source を整形する"),
        tool("lsharp_definition", "定義位置を返す"),
        tool("lsharp_references", "参照位置一覧を返す"),
        tool("lsharp_project_context", "lsharp.toml と依存関係を返す"),
        tool("lsharp_package_api", "ローカル package api.json を返す"),
        tool("lsharp_stdlib_api", "stdlib API を返す"),
        tool("lsharp_compile_run", "compile と実行結果を返す"),
        tool("lsharp_errors", "エラーコードの説明を返す"),
        tool(
            "lsharp_search",
            "ローカルインストール済みパッケージを検索する",
        ),
    ]
}

pub fn call_tool(name: &str, arguments: &Value) -> Result<Value, String> {
    match name {
        "lsharp_hover" => hover_tool(arguments),
        "lsharp_check" => check_tool(arguments),
        "lsharp_validate" => validate_tool(arguments),
        "lsharp_completion" => completion_tool(arguments),
        "lsharp_format" => format_tool(arguments),
        "lsharp_definition" => definition_tool(arguments),
        "lsharp_references" => references_tool(arguments),
        "lsharp_project_context" => project_context_tool(arguments),
        "lsharp_package_api" => package_api_tool(arguments),
        "lsharp_stdlib_api" => stdlib_api_tool(arguments),
        "lsharp_compile_run" => compile_run_tool(arguments),
        "lsharp_errors" => errors_tool(arguments),
        "lsharp_search" => search_tool(arguments),
        _ => Err("tool not found".to_string()),
    }
}

fn tool(name: &str, description: &str) -> McpTool {
    McpTool {
        name: name.to_string(),
        description: description.to_string(),
    }
}

fn tool_input_schema(name: &str) -> Value {
    match name {
        "lsharp_check" => json_schema(&["source"], &["file"]),
        "lsharp_validate" => validate_input_schema(),
        "lsharp_hover" | "lsharp_completion" | "lsharp_definition" | "lsharp_references" => {
            json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "file": { "type": "string" },
                    "line": { "type": "integer", "minimum": 0 },
                    "character": { "type": "integer", "minimum": 0 },
                    "col": { "type": "integer", "minimum": 0 }
                },
                "required": ["line", "character"]
            })
        }
        "lsharp_format" | "lsharp_compile_run" => json_schema(&["source"], &["file"]),
        "lsharp_errors" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "error_code": { "type": "string" }
            },
            "required": ["error_code"]
        }),
        "lsharp_package_api" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "project_dir": { "type": "string" }
            },
            "required": ["name"]
        }),
        "lsharp_stdlib_api" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "module": { "type": "string" }
            }
        }),
        "lsharp_project_context" | "lsharp_search" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "project_dir": { "type": "string" },
                "query": { "type": "string" }
            }
        }),
        _ => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object"
        }),
    }
}

fn tool_output_schema(name: &str) -> Value {
    match name {
        "lsharp_check" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["ok", "diagnostics", "migrationDiagnostics"],
            "properties": {
                "ok": { "type": "boolean" },
                "diagnostics": { "type": "array" },
                "migrationDiagnostics": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["code", "owner", "selectedSemantics", "disposition", "range"],
                        "properties": {
                            "code": {
                                "type": "string",
                                "enum": ["LS2001", "LS2002", "LS2003"]
                            },
                            "owner": { "type": "string" },
                            "selectedSemantics": {
                                "type": "string",
                                "enum": [
                                    "legacy-example-truthiness",
                                    "legacy-invariant-deterministic-smoke"
                                ]
                            },
                            "disposition": {
                                "type": "string",
                                "enum": [
                                    "docs-only-example",
                                    "assertion",
                                    "property-postcondition",
                                    "manual-review"
                                ]
                            },
                            "range": {
                                "type": "object",
                                "required": ["start", "end"],
                                "properties": {
                                    "start": { "$ref": "#/$defs/position" },
                                    "end": { "$ref": "#/$defs/position" }
                                }
                            },
                            "message": { "type": "string" }
                        }
                    }
                }
            },
            "$defs": {
                "position": {
                    "type": "object",
                    "required": ["line", "character"],
                    "properties": {
                        "line": { "type": "integer", "minimum": 0 },
                        "character": { "type": "integer", "minimum": 0 }
                    }
                }
            }
        }),
        "lsharp_validate" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": [
                "status",
                "trace_gaps",
                "open_questions",
                "independent_reviews",
                "contradicting_observations"
            ],
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["pass", "fail", "unknown"]
                },
                "trace_gaps": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["code", "subject_id"],
                        "properties": {
                            "code": { "type": "string" },
                            "subject_id": { "type": "string" }
                        }
                    }
                },
                "open_questions": { "type": "integer", "minimum": 0 },
                "independent_reviews": { "type": "integer", "minimum": 0 },
                "contradicting_observations": { "type": "integer", "minimum": 0 },
                "manifest": {
                    "type": "object",
                    "required": ["schema_version", "nodes", "evidence", "edges"],
                    "properties": {
                        "schema_version": { "type": "integer", "const": 1 },
                        "nodes": { "type": "array" },
                        "evidence": { "type": "array" },
                        "edges": { "type": "array" }
                    }
                }
            }
        }),
        _ => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object"
        }),
    }
}

fn json_schema(required_primary: &[&str], required_secondary: &[&str]) -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": {
            "source": { "type": "string" },
            "file": { "type": "string" }
        },
        "anyOf": [
            { "required": required_primary },
            { "required": required_secondary }
        ]
    })
}

fn validate_input_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": {
            "source": { "type": "string" },
            "file": { "type": "string" },
            "manifest": {
                "oneOf": [
                    { "type": "object" },
                    { "type": "string" }
                ]
            },
            "manifest_file": { "type": "string" },
            "include_manifest": { "type": "boolean" }
        },
        "oneOf": [
            { "required": ["source"] },
            { "required": ["file"] },
            { "required": ["manifest"] },
            { "required": ["manifest_file"] }
        ]
    })
}

fn hover_tool(arguments: &Value) -> Result<Value, String> {
    let source = source_argument(arguments)?;
    let position = position_argument(arguments)?;
    let hover = lsharp_lsp::analyze_hover(&source, position)
        .ok_or_else(|| "hover を解決できませんでした".to_string())?;
    let text = match hover.contents {
        lsharp_lsp::HoverContents::Scalar(lsharp_lsp::MarkedString::String(text)) => text,
        _ => return Err("hover contents を文字列へ変換できませんでした".to_string()),
    };
    let (name, ty, doc) = split_hover_text(&text);
    Ok(json!({
        "name": name,
        "type": ty,
        "doc": doc,
    }))
}

fn check_tool(arguments: &Value) -> Result<Value, String> {
    let source = source_argument(arguments)?;
    let diagnostics = lsharp_lsp::parse_and_check(&source);
    let migration_diagnostics = legacy_migration_diagnostics_json(&source);
    Ok(json!({
        "ok": diagnostics.is_empty(),
        "diagnostics": diagnostics.iter().map(|diag| json!({
            "message": diag.message,
            "severity": diag.severity.map(|severity| format!("{severity:?}")),
            "code": diag
                .code
                .as_ref()
                .and_then(|code| serde_json::to_value(code).ok()),
            "range": {
                "start": {
                    "line": diag.range.start.line,
                    "character": diag.range.start.character,
                },
                "end": {
                    "line": diag.range.end.line,
                    "character": diag.range.end.character,
                },
            },
        })).collect::<Vec<_>>(),
        "migrationDiagnostics": migration_diagnostics,
    }))
}

fn legacy_migration_diagnostics_json(source: &str) -> Vec<Value> {
    let Ok(program) = lsharp_syntax::parse(source) else {
        return Vec::new();
    };
    let Ok(diagnostics) = lsharp_types::metadata_migration::classify_legacy_contracts(&program)
    else {
        return Vec::new();
    };

    diagnostics
        .iter()
        .map(|diagnostic| {
            json!({
                "code": diagnostic.code(),
                "owner": diagnostic.owner(),
                "selectedSemantics": diagnostic.selected_semantics().as_str(),
                "disposition": diagnostic.disposition().as_str(),
                "range": source_span_range(source, diagnostic.span()),
                "message": diagnostic.message(),
            })
        })
        .collect()
}

fn source_span_range(source: &str, span: lsharp_syntax::span::Span) -> Value {
    let (start_line, start_character) = source_offset_position(source, span.start);
    let (end_line, end_character) = source_offset_position(source, span.end);
    json!({
        "start": {
            "line": start_line,
            "character": start_character,
        },
        "end": {
            "line": end_line,
            "character": end_character,
        },
    })
}

fn source_offset_position(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0;
    let mut character = 0;
    for (index, ch) in source.char_indices() {
        if index >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }
    (line, character)
}

fn completion_tool(arguments: &Value) -> Result<Value, String> {
    let source = source_argument(arguments)?;
    let position = position_argument(arguments)?;
    let project_dir = project_dir_argument(arguments);
    let module_candidates = list_module_candidates(&project_dir);
    let items = lsharp_lsp::analyze_completion(&source, position, &module_candidates);
    Ok(json!({
        "items": items.iter().map(|item| json!({
            "label": item.label,
            "kind": item.kind.map(|kind| format!("{kind:?}")),
            "insertText": item.insert_text,
        })).collect::<Vec<_>>()
    }))
}

fn format_tool(arguments: &Value) -> Result<Value, String> {
    let source = source_argument(arguments)?;
    Ok(json!({ "formatted": lsharp_lsp::format_source(&source) }))
}

fn definition_tool(arguments: &Value) -> Result<Value, String> {
    let source = source_argument(arguments)?;
    let position = position_argument(arguments)?;
    let location = lsharp_lsp::find_definition(&source, position)
        .ok_or_else(|| "definition が見つかりません".to_string())?;
    Ok(json!({
        "start": {
            "line": location.start.line,
            "character": location.start.character,
        },
        "end": {
            "line": location.end.line,
            "character": location.end.character,
        }
    }))
}

fn references_tool(arguments: &Value) -> Result<Value, String> {
    let source = source_argument(arguments)?;
    let position = position_argument(arguments)?;
    let references = lsharp_lsp::find_references(&source, position, true);
    Ok(json!({
        "count": references.len(),
        "ranges": references.iter().map(|range| json!({
            "start": {
                "line": range.start.line,
                "character": range.start.character,
            },
            "end": {
                "line": range.end.line,
                "character": range.end.character,
            }
        })).collect::<Vec<_>>()
    }))
}

fn errors_tool(arguments: &Value) -> Result<Value, String> {
    let code = arguments
        .get("error_code")
        .and_then(Value::as_str)
        .ok_or_else(|| "error_code が必要です".to_string())?;
    let Some(entry) = error_codes::find_error_code(code) else {
        return Ok(json!({
            "code": code,
            "name": "unknown",
            "description": "未知のエラーコードです",
            "fix": "最新版ドキュメントを確認してください",
            "doc": error_codes::ERROR_REFERENCE_DOC,
        }));
    };
    Ok(json!({
        "code": entry.code,
        "legacy_code": entry.legacy_code,
        "name": entry.name,
        "description": entry.summary,
        "detail": entry.detail,
        "fix": entry.fix,
        "doc": error_codes::ERROR_REFERENCE_DOC,
    }))
}

fn search_tool(arguments: &Value) -> Result<Value, String> {
    let project_dir = project_dir_argument(arguments);
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default();

    let packages = installed_packages(&project_dir)
        .into_iter()
        .filter(|pkg| {
            query.is_empty()
                || pkg["name"]
                    .as_str()
                    .is_some_and(|name| name.contains(query))
        })
        .collect::<Vec<_>>();
    Ok(json!({ "packages": packages }))
}

fn source_argument(arguments: &Value) -> Result<String, String> {
    if let Some(source) = arguments.get("source").and_then(Value::as_str) {
        return Ok(source.to_string());
    }
    if let Some(file) = arguments.get("file").and_then(Value::as_str) {
        return std::fs::read_to_string(file).map_err(|e| mcp_io_error(file, e));
    }
    Err("source または file が必要です".to_string())
}

fn position_argument(arguments: &Value) -> Result<lsharp_lsp::Position, String> {
    let line = arguments
        .get("line")
        .and_then(Value::as_u64)
        .ok_or_else(|| "line が必要です".to_string())?;
    let character = arguments
        .get("character")
        .or_else(|| arguments.get("col"))
        .and_then(Value::as_u64)
        .ok_or_else(|| "character が必要です".to_string())?;
    Ok(lsharp_lsp::Position::new(line as u32, character as u32))
}

fn split_hover_text(text: &str) -> (String, String, String) {
    let mut lines = text.lines();
    let signature_line = lines.next().unwrap_or_default();
    let mut signature_parts = signature_line.splitn(2, " : ");
    let name = signature_parts.next().unwrap_or_default().to_string();
    let ty = signature_parts.next().unwrap_or_default().to_string();
    let doc = lines
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (name, ty, doc)
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn jsonrpc_error(id: Value, code: i32, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

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
                "contradicting_observations"
            ])
        );
        assert_eq!(
            tool["outputSchema"]["properties"]["status"]["enum"],
            json!(["pass", "fail", "unknown"])
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
}
