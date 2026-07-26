fn compile_run_tool(arguments: &Value) -> Result<Value, String> {
    let temp_dir = std::env::temp_dir().join("lsharp_mcp_compile_run");
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).map_err(|e| mcp_io_error(temp_dir.display(), e))?;
    let input_path = temp_dir.join("Main.ls");
    let output_path = temp_dir.join("Main.wasm");

    if let Some(source) = arguments.get("source").and_then(Value::as_str) {
        std::fs::write(&input_path, source).map_err(|e| mcp_io_error(input_path.display(), e))?;
    } else if let Some(file) = arguments.get("file").and_then(Value::as_str) {
        let content = std::fs::read_to_string(file).map_err(|e| mcp_io_error(file, e))?;
        std::fs::write(&input_path, content).map_err(|e| mcp_io_error(input_path.display(), e))?;
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
    let formatted =
        std::fs::read_to_string(&input_path).map_err(|e| mcp_io_error(input_path.display(), e))?;
    let wasm_bytes = std::fs::read(&artifacts.output_path)
        .map_err(|e| mcp_io_error(artifacts.output_path.display(), e))?;
    let stdout = lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes)
        .map_err(|e| format!("実行失敗: {e}"))?;

    Ok(json!({
        "ok": true,
        "formatted": formatted,
        "stdout": stdout,
        "exit_code": 0,
    }))
}
