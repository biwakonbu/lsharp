//! WasmGC core module の実行境界。
//!
//! Preview1/Preview2 の WASI runner と分離し、WasmGC の `main` export と
//! `env.print-string` host import だけを接続する。WasmGC backend はまだ WASI/component
//! module を生成しないため、ここで暗黙の WASI fallback を行わない。

use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use wasmtime::component::{Component, Linker as ComponentLinker, Val as ComponentVal};
use wasmtime::{Config, Engine, ExternType, Instance, Module, Store, StoreContextMut};
use wasmtime_wasi::bindings::cli::stdout::Host as Preview2StdoutHost;
use wasmtime_wasi::{HostOutputStream, ResourceTable, WasiCtx, WasiCtxBuilder, WasiImpl, WasiView};

use crate::wasi_runner::ExecutionOutput;
use crate::wasmgc_host::create_component_output_import;
use crate::wasmgc_host::create_print_string_import;

const DEFAULT_MAX_WASM_STACK: usize = 64 * 1024 * 1024;
const DEFAULT_STDOUT_CAPTURE_BYTES: usize = 64 * 1024 * 1024;

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

/// WasmGC core module の canonical output を `std::io::Write` へ接続して実行する。
///
/// canonical import 一回分の bytes は `Write::write_all` で全量を消費し、partial write は
/// 内部で再試行する。`WriteZero` / write error は trap として返し、main の正常終了後だけ
/// `flush` を呼び出すため、exit code と flush error の順序も固定される。
pub fn run_wasm_wasmgc_component_output_to_writer<W>(
    wasm_bytes: &[u8],
    writer: W,
) -> Result<i32, String>
where
    W: Write + Send + 'static,
{
    let writer = Arc::new(Mutex::new(writer));
    let writer_for_sink = Arc::clone(&writer);
    let exit_code = run_wasm_wasmgc_component_output_with_stdout_sink(wasm_bytes, move |bytes| {
        let mut writer = writer_for_sink
            .lock()
            .map_err(|_| "WasmGC component output writer の mutex が poisoned です".to_string())?;
        writer
            .write_all(bytes)
            .map_err(|error| format!("WasmGC component output writer failed: {error}"))
    })?;
    let mut writer = writer
        .lock()
        .map_err(|_| "WasmGC component output writer の mutex が poisoned です".to_string())?;
    writer
        .flush()
        .map_err(|error| format!("WasmGC component output writer flush failed: {error}"))?;
    Ok(exit_code)
}

/// canonical output を stdout 相当の WASI `fd_write` 境界へ接続して実行する。
///
/// `fd_write` handler は fd と一つの bytes chunk を受け取り、実際に消費した byte 数または
/// WASI errno を返す。partial write は `write_all` が再試行し、zero/over-report/errno は
/// fail-closed に停止する。handler の背後にある実 WASI context の所有権は呼び出し側に残す。
pub fn run_wasm_wasmgc_component_output_to_fd_write<F>(
    wasm_bytes: &[u8],
    fd: u32,
    fd_write: F,
) -> Result<i32, String>
where
    F: Fn(u32, &[u8]) -> Result<usize, u16> + Send + Sync + 'static,
{
    run_wasm_wasmgc_component_output_to_writer(
        wasm_bytes,
        ComponentOutputFdWriteAdapter { fd, fd_write },
    )
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

/// `wasmgc-output` Component を実 WASI Preview2 context の stdout stream へ接続して実行する。
///
/// custom `stdout.write(list<u8>)` は Preview2 `WasiCtx` の stdout resource を毎回取得し、
/// `check-write` / `write` / `flush` の順で消費する。WASI linker は同じ Component linker に
/// 登録するが、custom world が import しない WASI interface への暗黙 fallback は行わない。
pub fn run_wasm_wasmgc_component_output_component_with_preview2_stdout(
    component_bytes: &[u8],
    dir: Option<&Path>,
    args: &[&str],
    stdin_data: &str,
) -> Result<ExecutionOutput, String> {
    run_wasm_wasmgc_component_with_preview2_stdout(
        component_bytes,
        dir,
        args,
        stdin_data,
        ComponentOutputRun::MainS64,
    )
}

/// `wasi:cli/run` export を持つ WasmGC CLI Component を実 WASI Preview2 stdout stream で実行する。
///
/// custom output interface は Stage 2p と同じ `WasiCtx`/`ResourceTable` を使い、Component の
/// command entry point だけを `wasi:cli/run@0.2.3#run` へ切り替える。
pub fn run_wasm_wasmgc_component_cli_with_preview2_stdout(
    component_bytes: &[u8],
    dir: Option<&Path>,
    args: &[&str],
    stdin_data: &str,
) -> Result<ExecutionOutput, String> {
    run_wasm_wasmgc_component_with_preview2_stdout(
        component_bytes,
        dir,
        args,
        stdin_data,
        ComponentOutputRun::WasiCli,
    )
}

enum ComponentOutputRun {
    MainS64,
    WasiCli,
}

fn run_wasm_wasmgc_component_with_preview2_stdout(
    component_bytes: &[u8],
    dir: Option<&Path>,
    args: &[&str],
    stdin_data: &str,
    run_mode: ComponentOutputRun,
) -> Result<ExecutionOutput, String> {
    let engine = configured_engine()?;
    let component = Component::new(&engine, component_bytes)
        .map_err(|error| format!("WasmGC output Component の読み込みに失敗: {error}"))?;

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(DEFAULT_STDOUT_CAPTURE_BYTES);
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    builder.stdin(wasmtime_wasi::pipe::MemoryInputPipe::new(
        stdin_data.as_bytes().to_vec(),
    ));
    builder.args(args);
    if let Some(dir_path) = dir {
        builder
            .preopened_dir(
                dir_path,
                ".",
                wasmtime_wasi::DirPerms::all(),
                wasmtime_wasi::FilePerms::all(),
            )
            .map_err(|error| format!("component preopened_dir に失敗: {error}"))?;
    }

    let state = ComponentPreview2State {
        ctx: builder.build(),
        table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, state);
    let mut linker = ComponentLinker::<ComponentPreview2State>::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker)
        .map_err(|error| format!("WASI Preview2 リンクに失敗: {error}"))?;
    let mut stdout_interface = linker
        .instance("lsharp:wasmgc-output/stdout@0.1.0")
        .map_err(|error| format!("WasmGC output stdout interface の定義に失敗: {error}"))?;
    stdout_interface
        .func_wrap(
            "write",
            move |mut store: StoreContextMut<'_, ComponentPreview2State>,
                  (bytes,): (Vec<u8>,)|
                  -> Result<(), wasmtime::Error> {
                write_component_output_to_preview2_stdout(&mut store, &bytes)
                    .map_err(wasmtime::Error::msg)
            },
        )
        .map_err(|error| format!("WasmGC output stdout write の定義に失敗: {error}"))?;

    let instance = linker
        .instantiate(&mut store, &component)
        .map_err(|error| format!("WasmGC output Component の instantiate に失敗: {error}"))?;
    let exit_code = match run_mode {
        ComponentOutputRun::MainS64 => {
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
            i32::try_from(*exit_code).map_err(|error| {
                format!("WasmGC output Component exit code が i32 範囲外です: {error}")
            })?
        }
        ComponentOutputRun::WasiCli => {
            let run =
                find_wasmgc_cli_run_func(&component, &instance, &mut store).ok_or_else(|| {
                    "WasmGC CLI Component に wasi:cli/run@0.2.3#run export がありません".to_string()
                })?;
            let mut results = [ComponentVal::Bool(false)];
            run.call(&mut store, &[], &mut results).map_err(|error| {
                format!("WasmGC CLI Component wasi:cli/run の実行に失敗: {error:#}")
            })?;
            decode_wasmgc_component_run_result(results.first())?
        }
    };

    drop(store);
    let stdout = stdout
        .try_into_inner()
        .ok_or_else(|| "WasmGC output Component stdout の取得に失敗".to_string())?;
    let stdout = String::from_utf8(stdout.to_vec()).map_err(|error| {
        format!(
            "WasmGC output Component stdout の UTF-8 変換に失敗: {error}; stdout_lossy={:?}",
            String::from_utf8_lossy(&stdout)
        )
    })?;
    Ok(ExecutionOutput { stdout, exit_code })
}

fn find_wasmgc_cli_run_func(
    component: &Component,
    instance: &wasmtime::component::Instance,
    store: &mut Store<ComponentPreview2State>,
) -> Option<wasmtime::component::Func> {
    for export_name in ["wasi:cli/run@0.2.3#run", "wasi:cli/run@0.2.0#run"] {
        if let Some(run) = instance.get_func(&mut *store, export_name) {
            return Some(run);
        }
    }
    for interface_name in ["wasi:cli/run@0.2.3", "wasi:cli/run@0.2.0"] {
        if let Some((_, interface_index)) = component.export_index(None, interface_name)
            && let Some((_, run_index)) = component.export_index(Some(&interface_index), "run")
            && let Some(run) = instance.get_func(&mut *store, run_index)
        {
            return Some(run);
        }
    }
    None
}

fn decode_wasmgc_component_run_result(result: Option<&ComponentVal>) -> Result<i32, String> {
    match result {
        Some(ComponentVal::Bool(false)) => Ok(0),
        Some(ComponentVal::Bool(true)) => Ok(1),
        Some(ComponentVal::Result(Ok(None))) => Ok(0),
        Some(ComponentVal::Result(Err(None))) => Ok(1),
        other => Err(format!(
            "WasmGC CLI Component wasi:cli/run の戻り値型が想定外です: {other:?}"
        )),
    }
}

struct ComponentPreview2State {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for ComponentPreview2State {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

fn write_component_output_to_preview2_stdout(
    store: &mut StoreContextMut<'_, ComponentPreview2State>,
    bytes: &[u8],
) -> Result<(), String> {
    let mut wasi = WasiImpl(store.data_mut());
    let resource =
        <WasiImpl<&mut ComponentPreview2State> as Preview2StdoutHost>::get_stdout(&mut wasi)
            .map_err(|error| format!("WASI Preview2 stdout resource の取得に失敗: {error}"))?;
    let write_result = {
        let stream = wasi
            .table()
            .get_mut(&resource)
            .map_err(|error| format!("WASI Preview2 stdout stream の取得に失敗: {error}"))?;
        write_preview2_stream(&mut **stream, bytes)
    };
    let delete_result = wasi
        .table()
        .delete(resource)
        .map(|_| ())
        .map_err(|error| format!("WASI Preview2 stdout resource の解放に失敗: {error}"));
    write_result.and(delete_result)
}

fn write_preview2_stream(
    stream: &mut (impl HostOutputStream + ?Sized),
    bytes: &[u8],
) -> Result<(), String> {
    let mut remaining = bytes;
    while !remaining.is_empty() {
        let permit = stream
            .check_write()
            .map_err(|error| format!("WASI Preview2 stdout check-write に失敗: {error}"))?;
        if permit == 0 {
            return Err("WASI Preview2 stdout check-write が 0 bytes を返しました".to_string());
        }
        let chunk_len = permit.min(remaining.len());
        stream
            .write(remaining[..chunk_len].to_vec().into())
            .map_err(|error| format!("WASI Preview2 stdout write に失敗: {error}"))?;
        remaining = &remaining[chunk_len..];
    }
    stream
        .flush()
        .map_err(|error| format!("WASI Preview2 stdout flush に失敗: {error}"))
}

struct ComponentOutputFdWriteAdapter<F> {
    fd: u32,
    fd_write: F,
}

impl<F> Write for ComponentOutputFdWriteAdapter<F>
where
    F: Fn(u32, &[u8]) -> Result<usize, u16>,
{
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = (self.fd_write)(self.fd, bytes)
            .map_err(|errno| io::Error::other(format!("WASI fd_write errno {errno}")))?;
        if written > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "WASI fd_write over-reported bytes: {written} > {}",
                    bytes.len()
                ),
            ));
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
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
