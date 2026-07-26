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
