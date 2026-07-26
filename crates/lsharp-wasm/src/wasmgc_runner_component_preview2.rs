use std::path::Path;

use wasmtime::component::{Component, Linker as ComponentLinker, Val as ComponentVal};
use wasmtime::{Store, StoreContextMut};

use wasmtime_wasi::bindings::cli::stdout::Host as Preview2StdoutHost;
use wasmtime_wasi::{HostOutputStream, ResourceTable, WasiCtx, WasiCtxBuilder, WasiImpl, WasiView};

use crate::wasi_runner::ExecutionOutput;

use super::{DEFAULT_STDOUT_CAPTURE_BYTES, configured_engine};

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
    run_wasm_wasmgc_component_output_component_with_preview2_stdout_and_preopen_rights(
        component_bytes,
        dir,
        args,
        stdin_data,
        Preview2PreopenRights::read_write(),
    )
}

/// `wasmgc-output` Component を指定した preopen rights で実行する。
pub fn run_wasm_wasmgc_component_output_component_with_preview2_stdout_and_preopen_rights(
    component_bytes: &[u8],
    dir: Option<&Path>,
    args: &[&str],
    stdin_data: &str,
    preopen_rights: Preview2PreopenRights,
) -> Result<ExecutionOutput, String> {
    let preopens = dir
        .map(|host_path| vec![Preview2Preopen::new(host_path, ".", preopen_rights)])
        .unwrap_or_default();
    run_wasm_wasmgc_component_output_component_with_preview2_stdout_and_preopens(
        component_bytes,
        args,
        stdin_data,
        &preopens,
    )
}

/// `wasmgc-output` Component を複数の名前付き preopen と指定 rights で実行する。
pub fn run_wasm_wasmgc_component_output_component_with_preview2_stdout_and_preopens(
    component_bytes: &[u8],
    args: &[&str],
    stdin_data: &str,
    preopens: &[Preview2Preopen<'_>],
) -> Result<ExecutionOutput, String> {
    run_wasm_wasmgc_component_with_preview2_stdout(
        component_bytes,
        args,
        stdin_data,
        ComponentOutputRun::MainS64,
        preopens,
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
    run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopen_rights(
        component_bytes,
        dir,
        args,
        stdin_data,
        Preview2PreopenRights::read_write(),
    )
}

/// `wasi:cli/run` export を持つ Component を指定した preopen rights で実行する。
pub fn run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopen_rights(
    component_bytes: &[u8],
    dir: Option<&Path>,
    args: &[&str],
    stdin_data: &str,
    preopen_rights: Preview2PreopenRights,
) -> Result<ExecutionOutput, String> {
    let preopens = dir
        .map(|host_path| vec![Preview2Preopen::new(host_path, ".", preopen_rights)])
        .unwrap_or_default();
    run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        component_bytes,
        args,
        stdin_data,
        &preopens,
    )
}

/// `wasi:cli/run` Component を複数の名前付き preopen と指定 rights で実行する。
pub fn run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
    component_bytes: &[u8],
    args: &[&str],
    stdin_data: &str,
    preopens: &[Preview2Preopen<'_>],
) -> Result<ExecutionOutput, String> {
    run_wasm_wasmgc_component_with_preview2_stdout(
        component_bytes,
        args,
        stdin_data,
        ComponentOutputRun::WasiCli,
        preopens,
    )
}

/// Preview2 preopen に付与する directory/file rights。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preview2PreopenRights {
    pub dir: wasmtime_wasi::DirPerms,
    pub file: wasmtime_wasi::FilePerms,
}

impl Preview2PreopenRights {
    /// directory/file の読み込みだけを許可する。
    pub fn read_only() -> Self {
        Self {
            dir: wasmtime_wasi::DirPerms::READ,
            file: wasmtime_wasi::FilePerms::READ,
        }
    }

    /// 既存 runner と同じ directory/file の読み書きを許可する。
    pub fn read_write() -> Self {
        Self {
            dir: wasmtime_wasi::DirPerms::all(),
            file: wasmtime_wasi::FilePerms::all(),
        }
    }
}

impl Default for Preview2PreopenRights {
    fn default() -> Self {
        Self::read_write()
    }
}

/// Preview2 に公開する一つの preopen。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Preview2Preopen<'a> {
    /// ホスト側で公開するディレクトリ。
    pub host_path: &'a Path,
    /// guest の preopen table に見せる相対パス名。
    pub guest_path: &'a str,
    /// この preopen に付与する directory/file rights。
    pub rights: Preview2PreopenRights,
}

impl<'a> Preview2Preopen<'a> {
    /// host path、guest path、rights から preopen を作成する。
    pub fn new(host_path: &'a Path, guest_path: &'a str, rights: Preview2PreopenRights) -> Self {
        Self {
            host_path,
            guest_path,
            rights,
        }
    }
}

enum ComponentOutputRun {
    MainS64,
    WasiCli,
}

fn run_wasm_wasmgc_component_with_preview2_stdout(
    component_bytes: &[u8],
    args: &[&str],
    stdin_data: &str,
    run_mode: ComponentOutputRun,
    preopens: &[Preview2Preopen<'_>],
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
    for preopen in preopens {
        builder
            .preopened_dir(
                preopen.host_path,
                preopen.guest_path,
                preopen.rights.dir,
                preopen.rights.file,
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
            match run.call(&mut store, &[], &mut results) {
                Ok(()) => decode_wasmgc_component_run_result(results.first())?,
                Err(error) => crate::wasi_runner::extract_i32_exit(&error).ok_or_else(|| {
                    format!("WasmGC CLI Component wasi:cli/run の実行に失敗: {error:#}")
                })?,
            }
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

pub(crate) fn decode_wasmgc_component_run_result(
    result: Option<&ComponentVal>,
) -> Result<i32, String> {
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
