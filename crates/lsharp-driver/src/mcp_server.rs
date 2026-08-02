use crate::{api_doc, commands, config, error_codes};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

include!("mcp_protocol.rs");
include!("mcp_validation.rs");
include!("mcp_context.rs");
include!("mcp_compile.rs");
include!("mcp_language.rs");
include!("mcp_schema.rs");

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
            "lsharp_install",
            "package install は明示的な external provider adapter が必要です",
        ),
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
        "lsharp_install" => install_tool(arguments),
        "lsharp_search" => search_tool(arguments),
        _ => Err("tool not found".to_string()),
    }
}

fn install_tool(arguments: &Value) -> Result<Value, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "lsharp_install の arguments は object が必要です".to_string())?;
    if let Some(unknown) = object
        .keys()
        .find(|key| !matches!(key.as_str(), "name" | "project_dir"))
    {
        return Err(format!("lsharp_install の未知の引数: {unknown}"));
    }
    match arguments.get("name") {
        Some(Value::String(name)) if !name.trim().is_empty() => {}
        Some(_) => return Err("lsharp_install の name は空でない文字列が必要です".to_string()),
        None => return Err("lsharp_install の name は必須です".to_string()),
    }
    if let Some(project_dir) = arguments.get("project_dir") {
        match project_dir {
            Value::String(path) if !path.trim().is_empty() => {}
            _ => {
                return Err("lsharp_install の project_dir は空でない文字列が必要です".to_string());
            }
        }
    }
    Err(
        "native MCP package installation requires an explicit external provider adapter"
            .to_string(),
    )
}

fn search_tool(arguments: &Value) -> Result<Value, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "lsharp_search の arguments は object が必要です".to_string())?;
    if let Some(unknown) = object
        .keys()
        .find(|key| !matches!(key.as_str(), "project_dir" | "query"))
    {
        return Err(format!("lsharp_search の未知の引数: {unknown}"));
    }
    let project_dir = match arguments.get("project_dir") {
        None => project_dir_argument(arguments),
        Some(Value::String(project_dir)) if !project_dir.trim().is_empty() => {
            PathBuf::from(project_dir)
        }
        Some(_) => return Err("lsharp_search の project_dir は文字列が必要です".to_string()),
    };
    let query = match arguments.get("query") {
        None => "",
        Some(Value::String(query)) => query.as_str(),
        Some(_) => return Err("lsharp_search の query は文字列が必要です".to_string()),
    };

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
include!("mcp_tests.rs");
