use crate::{api_doc, commands, config, error_codes};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpTool {
    pub name: String,
    pub description: String,
}

pub fn run_stdio_server() -> miette::Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line.map_err(|e| miette::miette!("stdin 読み込み失敗: {e}"))?;
        if line.trim().is_empty() {
            continue;
        }

        let request: Value =
            serde_json::from_str(&line).map_err(|e| miette::miette!("JSON パース失敗: {e}"))?;
        let response = handle_jsonrpc_message(&request);
        if response.is_null() {
            continue;
        }

        let payload = serde_json::to_string(&response)
            .map_err(|e| miette::miette!("JSON 直列化失敗: {e}"))?;
        writeln!(writer, "{payload}").map_err(|e| miette::miette!("stdout 書き込み失敗: {e}"))?;
        writer
            .flush()
            .map_err(|e| miette::miette!("stdout flush 失敗: {e}"))?;
    }

    Ok(())
}

pub fn handle_jsonrpc_message(request: &Value) -> Value {
    let id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or_else(|| json!({}));

    let Some(id) = id else {
        return Value::Null;
    };

    match method {
        "initialize" => jsonrpc_result(
            id,
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": "lsharp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        ),
        "ping" => jsonrpc_result(id, json!({})),
        "tools/list" => jsonrpc_result(
            id,
            json!({
                "tools": list_tools()
                    .into_iter()
                    .map(|tool| {
                        let mut descriptor = json!({
                            "name": tool.name,
                            "description": tool.description,
                            "inputSchema": tool_input_schema(&tool.name),
                        });
                        if tool.name == "lsharp_check" {
                            descriptor["outputSchema"] = tool_output_schema(&tool.name);
                        }
                        descriptor
                    })
                    .collect::<Vec<_>>()
            }),
        ),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let arguments = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match call_tool(name, &arguments) {
                Ok(value) => jsonrpc_result(
                    id,
                    json!({
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
                            }
                        ],
                        "structuredContent": value,
                        "isError": false
                    }),
                ),
                Err(error) => jsonrpc_result(
                    id,
                    json!({
                        "content": [
                            {
                                "type": "text",
                                "text": error,
                            }
                        ],
                        "isError": true
                    }),
                ),
            }
        }
        _ => jsonrpc_error(id, -32601, format!("Method not found: {method}")),
    }
}

pub fn list_tools() -> Vec<McpTool> {
    vec![
        tool("lsharp_check", "L# source を型チェックする"),
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
                        "required": [
                            "code",
                            "owner",
                            "selectedSemantics",
                            "disposition",
                            "range",
                            "message"
                        ],
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
                                },
                                "additionalProperties": false
                            },
                            "message": { "type": "string" }
                        },
                        "additionalProperties": false
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
                    },
                    "additionalProperties": false
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

fn project_context_tool(arguments: &Value) -> Result<Value, String> {
    let project_dir = project_dir_argument(arguments);
    let cfg = config::load_config(&project_dir);
    let dependencies = cfg
        .dependencies
        .iter()
        .map(|(name, spec)| dependency_summary(name, spec, &project_dir))
        .collect::<Vec<_>>();

    Ok(json!({
        "project": {
            "name": cfg.project.name,
            "version": cfg.project.version,
            "description": cfg.project.description,
            "exports": cfg.project.exports.modules,
        },
        "dependencies": dependencies,
        "installedPackages": installed_packages(&project_dir),
    }))
}

fn package_api_tool(arguments: &Value) -> Result<Value, String> {
    let name = arguments
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "name が必要です".to_string())?;
    let project_dir = project_dir_argument(arguments);
    let package_dir = find_installed_package_dir(&project_dir, name)
        .ok_or_else(|| format!("インストール済みパッケージ '{name}' が見つかりません"))?;
    read_or_generate_package_api(&package_dir)
}

fn stdlib_api_tool(arguments: &Value) -> Result<Value, String> {
    let stdlib_root = stdlib_root().ok_or_else(|| "stdlib が見つかりません".to_string())?;
    let package = "stdlib";
    let version = env!("CARGO_PKG_VERSION");
    let mut modules = Vec::new();
    let target_module = arguments.get("module").and_then(Value::as_str);

    let entries =
        std::fs::read_dir(&stdlib_root).map_err(|e| format!("{}: {e}", stdlib_root.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("ls") {
            continue;
        }
        let doc =
            api_doc::build_api_doc_for_file(package, version, &path).map_err(|e| e.to_string())?;
        let mut doc_modules = doc.modules;
        if let Some(module) = doc_modules.pop()
            && target_module.is_none_or(|target| target == module.name)
        {
            modules.push(module);
        }
    }
    modules.sort_by(|left, right| left.name.cmp(&right.name));

    Ok(json!({
        "package": package,
        "version": version,
        "modules": modules,
    }))
}

fn compile_run_tool(arguments: &Value) -> Result<Value, String> {
    let temp_dir = std::env::temp_dir().join("lsharp_mcp_compile_run");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).map_err(|e| format!("{}: {e}", temp_dir.display()))?;
    let input_path = temp_dir.join("Main.ls");
    let output_path = temp_dir.join("Main.wasm");

    if let Some(source) = arguments.get("source").and_then(Value::as_str) {
        std::fs::write(&input_path, source)
            .map_err(|e| format!("{}: {e}", input_path.display()))?;
    } else if let Some(file) = arguments.get("file").and_then(Value::as_str) {
        let content = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
        std::fs::write(&input_path, content)
            .map_err(|e| format!("{}: {e}", input_path.display()))?;
    } else {
        return Err("source または file が必要です".to_string());
    }

    let artifacts = commands::compile::compile_file(
        &input_path,
        Some(&output_path),
        false,
        Some(commands::compile::CompileTarget::WasiPreview1),
    )
    .map_err(|e| e.to_string())?;
    let formatted = std::fs::read_to_string(&input_path)
        .map_err(|e| format!("{}: {e}", input_path.display()))?;
    let wasm_bytes = std::fs::read(&artifacts.output_path)
        .map_err(|e| format!("{}: {e}", artifacts.output_path.display()))?;
    let stdout = lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes)
        .map_err(|e| format!("実行失敗: {e}"))?;

    Ok(json!({
        "ok": true,
        "formatted": formatted,
        "stdout": stdout,
        "exit_code": 0,
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
        return std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"));
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

fn project_dir_argument(arguments: &Value) -> PathBuf {
    arguments
        .get("project_dir")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn dependency_summary(name: &str, spec: &config::DependencySpec, project_dir: &Path) -> Value {
    match spec {
        config::DependencySpec::Version(version) => json!({
            "name": name,
            "version": version,
            "source": "registry"
        }),
        config::DependencySpec::Path { path } => json!({
            "name": name,
            "source": "path",
            "path": project_dir.join(path).display().to_string()
        }),
        config::DependencySpec::Git { git, branch, tag } => json!({
            "name": name,
            "source": "git",
            "git": git,
            "branch": branch,
            "tag": tag
        }),
    }
}

fn installed_packages(project_dir: &Path) -> Vec<Value> {
    let packages_dir = project_dir.join(".lsharp").join("packages");
    let Ok(entries) = std::fs::read_dir(packages_dir) else {
        return Vec::new();
    };
    let mut packages = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() && path.symlink_metadata().is_err() {
            continue;
        }
        let cfg = config::load_config(&path);
        let name = if cfg.project.name.is_empty() {
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("package")
                .to_string()
        } else {
            cfg.project.name
        };
        packages.push(json!({
            "name": name,
            "version": cfg.project.version,
            "path": path.display().to_string()
        }));
    }
    packages.sort_by(|left, right| left["name"].as_str().cmp(&right["name"].as_str()));
    packages
}

fn list_module_candidates(project_dir: &Path) -> Vec<String> {
    let mut modules = Vec::new();
    for package_dir in installed_package_dirs(project_dir) {
        let api_path = package_dir.join("docs").join("api.json");
        if let Ok(content) = std::fs::read_to_string(&api_path)
            && let Ok(value) = serde_json::from_str::<Value>(&content)
            && let Some(items) = value.get("modules").and_then(Value::as_array)
        {
            for item in items {
                if let Some(name) = item.get("name").and_then(Value::as_str) {
                    modules.push(name.to_string());
                }
            }
        }
    }
    modules.sort();
    modules.dedup();
    modules
}

fn installed_package_dirs(project_dir: &Path) -> Vec<PathBuf> {
    let packages_dir = project_dir.join(".lsharp").join("packages");
    let Ok(entries) = std::fs::read_dir(packages_dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();
    paths
}

fn find_installed_package_dir(project_dir: &Path, name: &str) -> Option<PathBuf> {
    installed_package_dirs(project_dir)
        .into_iter()
        .find(|path| {
            path.file_name()
                .and_then(|entry| entry.to_str())
                .is_some_and(|entry| entry.starts_with(&format!("{name}-")))
        })
}

fn read_or_generate_package_api(package_dir: &Path) -> Result<Value, String> {
    let api_path = package_dir.join("docs").join("api.json");
    if api_path.exists() {
        let content = std::fs::read_to_string(&api_path)
            .map_err(|e| format!("{}: {e}", api_path.display()))?;
        return serde_json::from_str(&content).map_err(|e| format!("{}: {e}", api_path.display()));
    }

    let cfg = config::load_config(package_dir);
    let package = if cfg.project.name.is_empty() {
        package_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package")
            .to_string()
    } else {
        cfg.project.name
    };
    let version = if cfg.project.version.is_empty() {
        "0.1.0".to_string()
    } else {
        cfg.project.version
    };
    let api = api_doc::build_api_doc_for_package(package_dir, &package, &version)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(api).map_err(|e| e.to_string())
}

fn stdlib_root() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("LSHARP_STDLIB_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib");
    if path.exists() { Some(path) } else { None }
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

    fn legacy_migration_schema() -> Value {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let path = repo_root.join("docs/schemas/legacy-migration.schema.json");
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} の読み込みに失敗: {error}", path.display()));
        serde_json::from_str(&source)
            .unwrap_or_else(|error| panic!("{} は valid JSON であるべき: {error}", path.display()))
    }

    #[test]
    fn test_legacy_migration_schema_declares_stable_enum_strings() {
        let schema = legacy_migration_schema();

        assert_eq!(
            schema["$defs"]["migrationCode"]["enum"],
            json!(["LS2001", "LS2002", "LS2003"])
        );
        assert_eq!(
            schema["$defs"]["selectedSemantics"]["enum"],
            json!([
                "legacy-example-truthiness",
                "legacy-invariant-deterministic-smoke"
            ])
        );
        assert_eq!(
            schema["$defs"]["migrationDisposition"]["enum"],
            json!([
                "docs-only-example",
                "assertion",
                "property-postcondition",
                "manual-review"
            ])
        );
    }

    #[test]
    fn test_legacy_migration_schema_keeps_selfhost_and_mcp_shapes_distinct() {
        let schema = legacy_migration_schema();
        let alternatives = schema["oneOf"].as_array().unwrap();
        assert_eq!(alternatives.len(), 2);
        assert_eq!(alternatives[0]["$ref"], "#/$defs/selfhostMigrationRow");
        assert_eq!(alternatives[1]["$ref"], "#/$defs/mcpMigrationDiagnostic");

        let selfhost_required = schema["$defs"]["selfhostMigrationRow"]["required"]
            .as_array()
            .unwrap();
        assert!(selfhost_required.contains(&json!("ownerHash")));
        assert!(selfhost_required.contains(&json!("span")));

        let mcp_required = schema["$defs"]["mcpMigrationDiagnostic"]["required"]
            .as_array()
            .unwrap();
        assert!(mcp_required.contains(&json!("owner")));
        assert!(mcp_required.contains(&json!("range")));
    }

    #[test]
    fn test_mcp_output_schema_matches_shared_legacy_migration_schema() {
        let schema = legacy_migration_schema();
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
        let migration_item = &tool["outputSchema"]["properties"]["migrationDiagnostics"]["items"];
        let properties = &migration_item["properties"];

        assert_eq!(
            migration_item["required"],
            schema["$defs"]["mcpMigrationDiagnostic"]["required"]
        );

        assert_eq!(
            properties["code"]["enum"],
            schema["$defs"]["migrationCode"]["enum"]
        );
        assert_eq!(
            properties["selectedSemantics"]["enum"],
            schema["$defs"]["selectedSemantics"]["enum"]
        );
        assert_eq!(
            properties["disposition"]["enum"],
            schema["$defs"]["migrationDisposition"]["enum"]
        );
    }

    #[test]
    fn test_mcp_output_schema_keeps_legacy_rows_strictly_closed() {
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
        let migration_item = &tool["outputSchema"]["properties"]["migrationDiagnostics"]["items"];
        let range = &migration_item["properties"]["range"];
        let position = &tool["outputSchema"]["$defs"]["position"];

        assert_eq!(
            migration_item["additionalProperties"],
            json!(false),
            "MCP migration row は共有 schema と同じく未知キーを拒否するべき"
        );
        assert_eq!(
            range["additionalProperties"],
            json!(false),
            "MCP migration range は未知キーを拒否するべき"
        );
        assert_eq!(
            position["additionalProperties"],
            json!(false),
            "MCP migration position は未知キーを拒否するべき"
        );
        assert_eq!(
            position["properties"]["line"]["minimum"],
            json!(0),
            "LSP line は非負整数として固定するべき"
        );
        assert_eq!(
            position["properties"]["character"]["minimum"],
            json!(0),
            "LSP character は非負整数として固定するべき"
        );
    }

    #[test]
    fn test_mcp_migration_rows_stay_inside_shared_legacy_migration_schema() {
        let schema = legacy_migration_schema();
        let source = "(defn succ [x] :example [(succ 0) (= (succ 1) 2)] :invariant (= result (+ x 1)) (+ x 1))";
        let result = call_tool("lsharp_check", &json!({"source": source}))
            .expect("lsharp_check は legacy migration report を返すべき");
        let rows = result["migrationDiagnostics"]
            .as_array()
            .expect("migrationDiagnostics は配列であるべき");

        assert_eq!(rows.len(), 3);
        for row in rows {
            assert!(
                schema["$defs"]["migrationCode"]["enum"]
                    .as_array()
                    .unwrap()
                    .contains(&row["code"])
            );
            assert!(
                schema["$defs"]["selectedSemantics"]["enum"]
                    .as_array()
                    .unwrap()
                    .contains(&row["selectedSemantics"])
            );
            assert!(
                schema["$defs"]["migrationDisposition"]["enum"]
                    .as_array()
                    .unwrap()
                    .contains(&row["disposition"])
            );
        }
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
