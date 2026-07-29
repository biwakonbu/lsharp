use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static COMPILE_RUN_TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

struct CompileRunTempDir {
    path: PathBuf,
}

impl Drop for CompileRunTempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn new_compile_run_temp_dir() -> Result<CompileRunTempDir, String> {
    let sequence = COMPILE_RUN_TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let path = std::env::temp_dir().join(format!(
        "lsharp_mcp_compile_run_{}_{}_{}",
        std::process::id(),
        timestamp,
        sequence
    ));

    std::fs::create_dir(&path).map_err(|error| mcp_io_error(path.display(), error))?;
    Ok(CompileRunTempDir { path })
}

fn compile_run_tool(arguments: &Value) -> Result<Value, String> {
    let temp_dir = new_compile_run_temp_dir()?;
    let input_path = temp_dir.path.join("Main.ls");
    let output_path = temp_dir.path.join("Main.wasm");

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
