fn validate_tool(arguments: &Value) -> Result<Value, String> {
    let review_inputs = review_input_arguments(arguments)?;
    let _ = review_inputs.explicit_count();
    let graph = validation_graph(arguments)?;
    let include_manifest = include_manifest_argument(arguments)?;
    let mut report = graph.validate().to_json_value();
    if include_manifest {
        report["manifest"] = graph.to_manifest_json_value();
    }
    Ok(report)
}

fn review_input_arguments(arguments: &Value) -> Result<crate::review_input::ReviewInputs, String> {
    let trust_store = path_argument(arguments, "trust_store")?;
    let lifecycle = path_argument(arguments, "review_lifecycle")?;
    let project_root = review_input_project_root(arguments)?;
    crate::review_input::load_review_inputs(
        &project_root,
        trust_store.as_deref(),
        lifecycle.as_deref(),
    )
    .map_err(|error| error.to_string())
}

fn path_argument(arguments: &Value, name: &str) -> Result<Option<std::path::PathBuf>, String> {
    match arguments.get(name) {
        None => Ok(None),
        Some(Value::String(path)) if !path.trim().is_empty() => Ok(Some(path.into())),
        Some(Value::String(_)) => Err(format!("{name} は空にできません")),
        Some(_) => Err(format!("{name} は文字列 path が必要です")),
    }
}

fn review_input_project_root(arguments: &Value) -> Result<std::path::PathBuf, String> {
    let start = ["manifest_file", "file"]
        .into_iter()
        .find_map(|name| arguments.get(name).and_then(Value::as_str))
        .map(|path| {
            let path = std::path::Path::new(path);
            if path.is_dir() {
                path.to_path_buf()
            } else {
                path.parent().unwrap_or(path).to_path_buf()
            }
        })
        .unwrap_or(std::env::current_dir().map_err(|error| error.to_string())?);
    let mut current = start;
    loop {
        if current.join("lsharp.toml").is_file() {
            return Ok(current);
        }
        let Some(parent) = current.parent() else {
            return Ok(current);
        };
        if parent == current {
            return Ok(current);
        }
        current = parent.to_path_buf();
    }
}

fn validation_graph(arguments: &Value) -> Result<lsharp_types::validation::IntentGraph, String> {
    let input_names = ["source", "file", "manifest", "manifest_file"]
        .into_iter()
        .filter(|name| arguments.get(*name).is_some())
        .collect::<Vec<_>>();
    if input_names.len() != 1 {
        return Err(
            "lsharp_validate は source、file、manifest、manifest_file のいずれか一つが必要です"
                .to_string(),
        );
    }

    match input_names[0] {
        "source" | "file" => source_graph_input(arguments),
        "manifest" => {
            let manifest = manifest_input_argument(
                arguments
                    .get("manifest")
                    .expect("manifest input name was collected"),
            )?;
            manifest_graph_input(&manifest)
        }
        "manifest_file" => {
            let file = arguments
                .get("manifest_file")
                .and_then(Value::as_str)
                .ok_or_else(|| "manifest_file は文字列 path が必要です".to_string())?;
            let manifest =
                std::fs::read_to_string(file).map_err(|error| mcp_io_error(file, error))?;
            manifest_graph_input(&manifest)
        }
        _ => unreachable!("input name is restricted to the validation schema"),
    }
}

fn source_graph_input(arguments: &Value) -> Result<lsharp_types::validation::IntentGraph, String> {
    let source = source_argument(arguments)?;
    let program = lsharp_syntax::parse(&source)
        .map_err(|error| format!("validation source の parse に失敗しました: {error}"))?;
    let graph = lsharp_types::validation_source::source_program_to_intent_graph(&program)
        .map_err(|error| format!("validation source graph の構築に失敗しました: {error}"))?;
    Ok(graph)
}

fn manifest_input_argument(value: &Value) -> Result<String, String> {
    match value {
        Value::Object(_) => serde_json::to_string(value)
            .map_err(|error| format!("validation manifest の JSON 化に失敗しました: {error}")),
        Value::String(source) => Ok(source.clone()),
        _ => Err("manifest は JSON object または JSON string が必要です".to_string()),
    }
}

fn manifest_graph_input(manifest: &str) -> Result<lsharp_types::validation::IntentGraph, String> {
    lsharp_types::validation_input::parse_intent_graph_json(manifest)
        .map_err(|error| format!("validation manifest の parse に失敗しました: {error}"))
}

fn include_manifest_argument(arguments: &Value) -> Result<bool, String> {
    match arguments.get("include_manifest") {
        None => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err("include_manifest は boolean が必要です".to_string()),
    }
}
