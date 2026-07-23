//! WasmGC core module の実行境界。
//!
//! Preview1/Preview2 の WASI runner と分離し、WasmGC の `main` export と
//! `env.print-string` host import だけを接続する。WasmGC backend はまだ WASI/component
//! module を生成しないため、ここで暗黙の WASI fallback を行わない。

use std::io::Write;
use std::sync::{Arc, Mutex};

use wasmtime::{Config, Engine, ExternType, Instance, Module, Store};

use crate::wasi_runner::ExecutionOutput;
use crate::wasmgc_host::create_component_output_import;
use crate::wasmgc_host::create_print_string_import;

const DEFAULT_MAX_WASM_STACK: usize = 64 * 1024 * 1024;

/// WasmGC core module を実行し、`print-string` の各 chunk を sink へ渡す。
///
/// sink は一回の `print-string` 呼び出しで渡された bytes 全体を消費する契約であり、
/// `Err` を返した場合は再試行せず Wasm 実行を trap として終了する。返り値は exported
/// `main` の i64 result を i32 へ検証変換した exit code である。
pub fn run_wasm_wasmgc_with_stdout_sink<F>(wasm_bytes: &[u8], sink: F) -> Result<i32, String>
where
    F: Fn(&[u8]) -> Result<(), String> + Send + Sync + 'static,
{
    let engine = configured_engine()?;
    let module = Module::new(&engine, wasm_bytes)
        .map_err(|error| format!("WasmGC module の読み込みに失敗: {error}"))?;
    let mut store = Store::new(&engine, ());
    let mut imports = Vec::with_capacity(module.imports().len());
    let mut sink = Some(sink);

    for import in module.imports() {
        if import.module() != "env" || import.name() != "print-string" {
            return Err(format!(
                "WasmGC runner は import {}.{} を未対応のまま実行しません",
                import.module(),
                import.name()
            ));
        }
        let ExternType::Func(func_type) = import.ty() else {
            return Err("env.print-string import は function である必要があります".into());
        };
        let sink = sink
            .take()
            .ok_or_else(|| "env.print-string import が重複しています".to_string())?;
        let host = create_print_string_import(&mut store, func_type, sink)
            .map_err(|error| format!("env.print-string host import の作成に失敗: {error}"))?;
        imports.push(host.into());
    }

    let instance = Instance::new(&mut store, &module, &imports)
        .map_err(|error| format!("WasmGC module のインスタンス化に失敗: {error}"))?;
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .map_err(|error| format!("WasmGC main export の取得に失敗: {error}"))?;
    let result = main
        .call(&mut store, ())
        .map_err(|error| format!("WasmGC 実行に失敗: {error:#}"))?;
    i32::try_from(result).map_err(|error| format!("WasmGC exit code が i32 範囲外です: {error}"))
}

/// WasmGC core module の canonical `list<u8>` output import を sink へ接続して実行する。
///
/// `lsharp:wasmgc-output/stdout@0.1.0::write` だけを解決し、WASI や GC reference import へ
/// 暗黙に fallback しない。core module は `memory` export を持ち、host callback は `(ptr, len)`
/// の範囲を一回の write として消費する。
pub fn run_wasm_wasmgc_component_output_with_stdout_sink<F>(
    wasm_bytes: &[u8],
    sink: F,
) -> Result<i32, String>
where
    F: Fn(&[u8]) -> Result<(), String> + Send + Sync + 'static,
{
    let engine = configured_engine()?;
    let module = Module::new(&engine, wasm_bytes)
        .map_err(|error| format!("WasmGC component output module の読み込みに失敗: {error}"))?;
    let mut store = Store::new(&engine, ());
    let mut imports = Vec::with_capacity(module.imports().len());
    let mut sink = Some(sink);

    for import in module.imports() {
        if import.module() != "lsharp:wasmgc-output/stdout@0.1.0" || import.name() != "write" {
            return Err(format!(
                "WasmGC component output runner は import {}.{} を未対応のまま実行しません",
                import.module(),
                import.name()
            ));
        }
        let ExternType::Func(func_type) = import.ty() else {
            return Err("component output write import は function である必要があります".into());
        };
        let sink = sink
            .take()
            .ok_or_else(|| "component output write import が重複しています".to_string())?;
        let host = create_component_output_import(&mut store, func_type, sink)
            .map_err(|error| format!("component output host import の作成に失敗: {error}"))?;
        imports.push(host.into());
    }

    let instance = Instance::new(&mut store, &module, &imports).map_err(|error| {
        format!("WasmGC component output module のインスタンス化に失敗: {error}")
    })?;
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .map_err(|error| format!("WasmGC component output main export の取得に失敗: {error}"))?;
    let result = main
        .call(&mut store, ())
        .map_err(|error| format!("WasmGC component output の実行に失敗: {error:#}"))?;
    i32::try_from(result)
        .map_err(|error| format!("WasmGC component output exit code が i32 範囲外です: {error}"))
}

/// WasmGC core module の canonical output bytes と exit code を capture する。
pub fn run_wasm_wasmgc_component_output_capture(
    wasm_bytes: &[u8],
) -> Result<ExecutionOutput, String> {
    let stdout = Arc::new(Mutex::new(Vec::<u8>::new()));
    let stdout_for_sink = Arc::clone(&stdout);
    let exit_code = run_wasm_wasmgc_component_output_with_stdout_sink(wasm_bytes, move |bytes| {
        stdout_for_sink
            .lock()
            .map_err(|_| "WasmGC component output stdout mutex が poisoned です".to_string())?
            .extend_from_slice(bytes);
        Ok(())
    })?;
    let stdout = stdout
        .lock()
        .map_err(|_| "WasmGC component output stdout mutex が poisoned です".to_string())?;
    let stdout = String::from_utf8(stdout.clone()).map_err(|error| {
        format!(
            "WasmGC component output stdout の UTF-8 変換に失敗: {error}; stdout_lossy={:?}",
            String::from_utf8_lossy(&stdout)
        )
    })?;
    Ok(ExecutionOutput { stdout, exit_code })
}

/// WasmGC core module を `std::io::Write` へ接続して実行する。
///
/// 各 `print-string` chunk は `Write::write_all` で全量を消費し、partial write は内部で再試行する。
/// `WriteZero` や I/O error は sink error として Wasm 実行へ返し、正常終了後には `flush` する。
pub fn run_wasm_wasmgc_to_writer<W>(wasm_bytes: &[u8], writer: W) -> Result<i32, String>
where
    W: Write + Send + 'static,
{
    let writer = Arc::new(Mutex::new(writer));
    let writer_for_sink = Arc::clone(&writer);
    let exit_code = run_wasm_wasmgc_with_stdout_sink(wasm_bytes, move |bytes| {
        let mut writer = writer_for_sink
            .lock()
            .map_err(|_| "WasmGC stdout writer の mutex が poisoned です".to_string())?;
        writer
            .write_all(bytes)
            .map_err(|error| format!("WasmGC stdout writer failed: {error}"))
    })?;
    let mut writer = writer
        .lock()
        .map_err(|_| "WasmGC stdout writer の mutex が poisoned です".to_string())?;
    writer
        .flush()
        .map_err(|error| format!("WasmGC stdout writer flush failed: {error}"))?;
    Ok(exit_code)
}

/// WasmGC core module を実行し、stdout と exit code を capture する。
pub fn run_wasm_wasmgc_capture(wasm_bytes: &[u8]) -> Result<ExecutionOutput, String> {
    let stdout = Arc::new(Mutex::new(Vec::<u8>::new()));
    let stdout_for_sink = Arc::clone(&stdout);
    let exit_code = run_wasm_wasmgc_with_stdout_sink(wasm_bytes, move |bytes| {
        stdout_for_sink
            .lock()
            .map_err(|_| "WasmGC stdout sink の mutex が poisoned です".to_string())?
            .extend_from_slice(bytes);
        Ok(())
    })?;
    let stdout = stdout
        .lock()
        .map_err(|_| "WasmGC stdout sink の mutex が poisoned です".to_string())?;
    let stdout = String::from_utf8(stdout.clone()).map_err(|error| {
        format!(
            "WasmGC stdout の UTF-8 変換に失敗: {error}; stdout_lossy={:?}",
            String::from_utf8_lossy(&stdout)
        )
    })?;
    Ok(ExecutionOutput { stdout, exit_code })
}

/// WasmGC core module を実行し、exit code 0 の stdout だけを返す。
pub fn run_wasm_wasmgc(wasm_bytes: &[u8]) -> Result<String, String> {
    let output = run_wasm_wasmgc_capture(wasm_bytes)?;
    if output.exit_code == 0 {
        Ok(output.stdout)
    } else {
        Err(format!("WasmGC 実行に失敗: exit code {}", output.exit_code))
    }
}

fn configured_engine() -> Result<Engine, String> {
    let mut config = Config::new();
    config.wasm_gc(true);
    config.max_wasm_stack(DEFAULT_MAX_WASM_STACK);
    Engine::new(&config).map_err(|error| format!("WasmGC engine 初期化に失敗: {error}"))
}
