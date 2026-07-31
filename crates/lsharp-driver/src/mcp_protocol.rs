const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

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
                        if matches!(
                            tool.name.as_str(),
                            "lsharp_check"
                                | "lsharp_validate"
                                | "lsharp_errors"
                                | "lsharp_search"
                                | "lsharp_project_context"
                                | "lsharp_package_api"
                                | "lsharp_stdlib_api"
                        ) {
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
