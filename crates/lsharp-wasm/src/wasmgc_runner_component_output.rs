use std::sync::{Arc, Mutex};

use wasmtime::component::{Component, Linker as ComponentLinker, Val as ComponentVal};
use wasmtime::{ExternType, Instance, Module, Store};

use crate::wasi_runner::ExecutionOutput;
use crate::wasmgc_host::create_component_output_import;

use super::{DEFAULT_STDOUT_CAPTURE_BYTES, configured_engine, run_wasm_wasmgc_with_stdout_sink};

#[path = "wasmgc_runner_output_writer.rs"]
mod output_writer;
#[path = "wasmgc_runner_component_preview2.rs"]
mod preview2;

pub use output_writer::{
    run_wasm_wasmgc_component_output_to_fd_write, run_wasm_wasmgc_component_output_to_writer,
    run_wasm_wasmgc_to_writer,
};

pub use preview2::{
    Preview2Preopen, Preview2PreopenRights, run_wasm_wasmgc_component_cli_with_preview2_stdout,
    run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopen_rights,
    run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens,
    run_wasm_wasmgc_component_output_component_with_preview2_stdout,
    run_wasm_wasmgc_component_output_component_with_preview2_stdout_and_preopen_rights,
    run_wasm_wasmgc_component_output_component_with_preview2_stdout_and_preopens,
};

#[cfg(test)]
pub(super) use preview2::decode_wasmgc_component_run_result;

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

/// WIT `wasmgc-output` Component を実際に instantiate し、stdout interface を sink へ接続する。
///
/// Component 境界では `list<u8>` が `Vec<u8>` として lift される。WASI Preview1/Preview2 の
/// linker へ暗黙に fallback せず、`lsharp:wasmgc-output/stdout@0.1.0` のみを定義する。
pub fn run_wasm_wasmgc_component_output_component_with_stdout_sink<F>(
    component_bytes: &[u8],
    sink: F,
) -> Result<i32, String>
where
    F: Fn(&[u8]) -> Result<(), String> + Send + Sync + 'static,
{
    let engine = configured_engine()?;
    let component = Component::new(&engine, component_bytes)
        .map_err(|error| format!("WasmGC output Component の読み込みに失敗: {error}"))?;
    let sink = Arc::new(Mutex::new(sink));
    let mut linker = ComponentLinker::<()>::new(&engine);
    let mut stdout = linker
        .instance("lsharp:wasmgc-output/stdout@0.1.0")
        .map_err(|error| format!("WasmGC output stdout interface の定義に失敗: {error}"))?;
    let sink_for_write = Arc::clone(&sink);
    stdout
        .func_wrap(
            "write",
            move |_store, (bytes,): (Vec<u8>,)| -> Result<(), wasmtime::Error> {
                let sink = sink_for_write.lock().map_err(|_| {
                    wasmtime::Error::msg("WasmGC output Component sink mutex が poisoned です")
                })?;
                sink(&bytes).map_err(|error| {
                    wasmtime::Error::msg(format!("WasmGC output Component sink failed: {error}"))
                })
            },
        )
        .map_err(|error| format!("WasmGC output stdout write の定義に失敗: {error}"))?;

    let mut store = Store::new(&engine, ());
    let instance = linker
        .instantiate(&mut store, &component)
        .map_err(|error| format!("WasmGC output Component の instantiate に失敗: {error}"))?;
    let main = instance
        .get_func(&mut store, "main")
        .ok_or_else(|| "WasmGC output Component に main export がありません".to_string())?;
    let mut results = [ComponentVal::S64(0)];
    main.call(&mut store, &[], &mut results)
        .map_err(|error| format!("WasmGC output Component の実行に失敗: {error:#}"))?;
    let Some(ComponentVal::S64(exit_code)) = results.first() else {
        return Err(format!(
            "WasmGC output Component main の戻り値型が s64 ではありません: {:?}",
            results.first()
        ));
    };
    i32::try_from(*exit_code)
        .map_err(|error| format!("WasmGC output Component exit code が i32 範囲外です: {error}"))
}

/// WIT `wasmgc-output` Component の stdout と exit code を capture する。
pub fn run_wasm_wasmgc_component_output_component_capture(
    component_bytes: &[u8],
) -> Result<ExecutionOutput, String> {
    let stdout = Arc::new(Mutex::new(Vec::<u8>::new()));
    let stdout_for_sink = Arc::clone(&stdout);
    let exit_code = run_wasm_wasmgc_component_output_component_with_stdout_sink(
        component_bytes,
        move |bytes| {
            stdout_for_sink
                .lock()
                .map_err(|_| "WasmGC output Component stdout mutex が poisoned です".to_string())?
                .extend_from_slice(bytes);
            Ok(())
        },
    )?;
    let stdout = stdout
        .lock()
        .map_err(|_| "WasmGC output Component stdout mutex が poisoned です".to_string())?;
    let stdout = String::from_utf8(stdout.clone()).map_err(|error| {
        format!(
            "WasmGC output Component stdout の UTF-8 変換に失敗: {error}; stdout_lossy={:?}",
            String::from_utf8_lossy(&stdout)
        )
    })?;
    Ok(ExecutionOutput { stdout, exit_code })
}
