//! WASI 実行ヘルパー
//!
//! Wasm バイナリを wasmtime の WASI 環境で実行するユーティリティ。
//! driver, e2e テスト, test_runner の 3 箇所で重複していたコードを統合。
//!
//! ## 実行モード
//!
//! - **Preview1**: 既存の core Wasm module を `wasi_snapshot_preview1` で実行
//! - **Preview2**: Component Model ベースの `.component.wasm` を実行

use wasmtime::*;
use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

const DEFAULT_MAX_WASM_STACK: usize = 64 * 1024 * 1024;
const DEFAULT_STDOUT_CAPTURE_BYTES: usize = 64 * 1024 * 1024;

fn configured_engine() -> Result<Engine, String> {
    let mut config = Config::new();
    // セルフホストの stage1→stage2 実行は深い再帰を踏みやすいため、
    // wasmtime のデフォルト stack より広めに取る。
    config.max_wasm_stack(DEFAULT_MAX_WASM_STACK);
    Engine::new(&config).map_err(|e| format!("wasmtime engine 初期化に失敗: {e}"))
}

/// WASI 実行モードの選択
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasiMode {
    /// WASI Preview1 (wasi_snapshot_preview1) — 既存の実行パス
    Preview1,
    /// WASI Preview2 (Component Model) — 新しい実行パス
    Preview2,
}

/// 明示した WASI mode で Wasm/Component を実行し、stdout を返す
pub fn run_wasm_with_mode(wasm_bytes: &[u8], mode: WasiMode) -> Result<String, String> {
    run_wasm_with_mode_and_dir_args_and_stdin(wasm_bytes, mode, None, &[], "")
}

/// 明示した WASI mode で Wasm/Component を実行する (ファイルシステム・argv・stdin 付き)
pub fn run_wasm_with_mode_and_dir_args_and_stdin(
    wasm_bytes: &[u8],
    mode: WasiMode,
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin: &str,
) -> Result<String, String> {
    let output = run_wasm_with_mode_capture(wasm_bytes, mode, dir, args, stdin)?;
    if output.exit_code == 0 {
        Ok(output.stdout)
    } else {
        Err(format!(
            "WASI {:?} 実行に失敗: exit code {}",
            mode, output.exit_code
        ))
    }
}

/// 明示した WASI mode で Wasm/Component を実行し、stdout と exit code を返す
pub fn run_wasm_with_mode_capture(
    wasm_bytes: &[u8],
    mode: WasiMode,
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin: &str,
) -> Result<ExecutionOutput, String> {
    match mode {
        WasiMode::Preview1 => {
            run_wasm_wasi_with_dir_args_and_stdin_capture(wasm_bytes, dir, args, stdin)
        }
        WasiMode::Preview2 => {
            run_wasm_component_with_dir_args_and_stdin_capture(wasm_bytes, dir, args, stdin)
        }
    }
}

/// 明示した WASI mode で Wasm/Component を実行し、親 stdin を継承した stdout/exit code を返す
pub fn run_wasm_with_mode_and_args_inherit_stdin_capture(
    wasm_bytes: &[u8],
    mode: WasiMode,
    dir: Option<&std::path::Path>,
    args: &[&str],
) -> Result<ExecutionOutput, String> {
    match mode {
        WasiMode::Preview1 => {
            run_wasm_wasi_with_dir_and_args_inherit_stdin_capture(wasm_bytes, dir, args)
        }
        WasiMode::Preview2 => {
            run_wasm_component_with_dir_and_args_inherit_stdin_capture(wasm_bytes, dir, args)
        }
    }
}

/// Wasm/Component 実行結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionOutput {
    pub stdout: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawExecutionOutput {
    pub stdout_bytes: Vec<u8>,
    pub exit_code: i32,
}

fn decode_stdout_bytes(bytes: &[u8]) -> Result<String, String> {
    String::from_utf8(bytes.to_vec()).map_err(|e| {
        let lossy = String::from_utf8_lossy(bytes);
        let hex_prefix = bytes
            .iter()
            .take(32)
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "stdout の UTF-8 変換に失敗: {e}; lossy_stdout={lossy:?}; stdout_hex_prefix={hex_prefix}"
        )
    })
}

fn format_component_trap_with_stdout(error: String, bytes: &[u8]) -> String {
    if bytes.is_empty() {
        error
    } else {
        format!("{error}; stdout_lossy={:?}", String::from_utf8_lossy(bytes))
    }
}

enum StdinMode<'a> {
    Memory(&'a str),
    Inherit,
}

pub(crate) fn extract_i32_exit(err: &wasmtime::Error) -> Option<i32> {
    for cause in err.chain() {
        if let Some(exit) = cause.downcast_ref::<wasmtime_wasi::I32Exit>() {
            return Some(exit.0);
        }
    }
    let rendered = format!("{err:#}");
    let marker = "Exited with i32 exit status ";
    if let Some(start) = rendered.find(marker) {
        let digits = rendered[start + marker.len()..]
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect::<String>();
        if let Ok(code) = digits.parse::<i32>() {
            return Some(code);
        }
    }
    None
}

/// WASI core runtime の容量拡張失敗を安定した診断コードへ分類する。
///
/// Wasmtime は `memory.grow` の失敗を、helper 関数内の `unreachable` trap として
/// 報告する。core WASI の helper 関数 index と trap の形を同時に確認し、ユーザー
/// 関数や Component Model の trap を誤って `LS4002` に分類しない。
pub fn classify_wasi_runtime_failure(error: &str) -> String {
    let is_root_slot_invariant_trap = crate::wasi::ROOT_SLOT_INVARIANT_FUNCTION_INDICES
        .iter()
        .any(|index| error.contains(&format!("<wasm function {index}>")));
    if is_root_slot_invariant_trap {
        return format!("LS4003: GC root slot の整合性が壊れました; {error}");
    }

    let is_capacity_trap = error.contains("unreachable")
        && crate::wasi::CAPACITY_FAILURE_FUNCTION_INDICES
            .iter()
            .any(|index| error.contains(&format!("<wasm function {index}>")));
    if is_capacity_trap {
        format!("LS4002: GC / linear memory の容量上限に達しました; {error}")
    } else {
        error.to_string()
    }
}

/// Wasm バイナリを WASI 環境で実行し、stdout 出力を返す
pub fn run_wasm_wasi(wasm_bytes: &[u8]) -> Result<String, String> {
    run_wasm_wasi_with_dir_args_and_stdin(wasm_bytes, None, &[], "")
}

/// Wasm バイナリを WASI 環境で実行 (ファイルシステムアクセス付き)
pub fn run_wasm_wasi_with_dir(
    wasm_bytes: &[u8],
    dir: Option<&std::path::Path>,
) -> Result<String, String> {
    run_wasm_wasi_with_dir_args_and_stdin(wasm_bytes, dir, &[], "")
}

/// Wasm バイナリを WASI 環境で実行 (ファイルシステム・argv 付き)
pub fn run_wasm_wasi_with_dir_and_args(
    wasm_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
) -> Result<String, String> {
    run_wasm_wasi_with_dir_args_and_stdin(wasm_bytes, dir, args, "")
}

/// Wasm バイナリを WASI 環境で実行 (ファイルシステム・argv・stdin 付き)
pub fn run_wasm_wasi_with_dir_args_and_stdin(
    wasm_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin: &str,
) -> Result<String, String> {
    let output = run_wasm_wasi_with_dir_args_and_stdin_capture(wasm_bytes, dir, args, stdin)?;
    if output.exit_code == 0 {
        Ok(output.stdout)
    } else {
        Err(format!("実行に失敗: exit code {}", output.exit_code))
    }
}

/// Wasm バイナリを WASI 環境で実行し、stdout と exit code を返す
pub fn run_wasm_wasi_with_dir_args_and_stdin_capture(
    wasm_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin: &str,
) -> Result<ExecutionOutput, String> {
    let output = run_wasm_wasi_capture_raw(wasm_bytes, dir, args, StdinMode::Memory(stdin))?;
    let stdout = decode_stdout_bytes(&output.stdout_bytes)?;
    Ok(ExecutionOutput {
        stdout,
        exit_code: output.exit_code,
    })
}

/// Wasm バイナリを WASI 環境で実行し、親 stdin を継承した stdout/exit code を返す
pub fn run_wasm_wasi_with_dir_and_args_inherit_stdin_capture(
    wasm_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
) -> Result<ExecutionOutput, String> {
    let output = run_wasm_wasi_capture_raw(wasm_bytes, dir, args, StdinMode::Inherit)?;
    let stdout = decode_stdout_bytes(&output.stdout_bytes)?;
    Ok(ExecutionOutput {
        stdout,
        exit_code: output.exit_code,
    })
}

pub fn run_wasm_wasi_with_dir_and_args_capture_raw(
    wasm_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
) -> Result<RawExecutionOutput, String> {
    run_wasm_wasi_capture_raw(wasm_bytes, dir, args, StdinMode::Memory(""))
}

fn run_wasm_wasi_capture_raw(
    wasm_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin_mode: StdinMode<'_>,
) -> Result<RawExecutionOutput, String> {
    let engine = configured_engine()?;
    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t)
        .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(DEFAULT_STDOUT_CAPTURE_BYTES);
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    match stdin_mode {
        StdinMode::Memory(stdin) => {
            let stdin = wasmtime_wasi::pipe::MemoryInputPipe::new(stdin.as_bytes().to_vec());
            builder.stdin(stdin);
        }
        StdinMode::Inherit => {
            builder.inherit_stdin();
        }
    }
    builder.args(args);
    if let Some(dir_path) = dir {
        builder
            .preopened_dir(
                dir_path,
                ".",
                wasmtime_wasi::DirPerms::all(),
                wasmtime_wasi::FilePerms::all(),
            )
            .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
    }
    let wasi = builder.build_p1();
    let mut store = Store::new(&engine, wasi);

    let module = wasmtime::Module::new(&engine, wasm_bytes)
        .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("インスタンス化に失敗: {e}"))?;

    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| format!("_start 関数が見つかりません: {e}"))?;
    let execution = start.call(&mut store, ());
    let mut trap_error = None;
    let exit_code = match execution {
        Ok(()) => 0,
        Err(e) => {
            if let Some(exit) = extract_i32_exit(&e) {
                exit
            } else {
                let rendered = format!("実行に失敗: {e:#}");
                trap_error = Some(classify_wasi_runtime_failure(&rendered));
                1
            }
        }
    };

    drop(store);
    let bytes = stdout
        .try_into_inner()
        .ok_or_else(|| "stdout の取得に失敗".to_string())?;
    if let Some(trap_error) = trap_error {
        if bytes.is_empty() {
            return Err(trap_error);
        }
        return Err(format!(
            "{trap_error}; stdout_lossy={:?}",
            String::from_utf8_lossy(&bytes)
        ));
    }
    Ok(RawExecutionOutput {
        stdout_bytes: bytes.to_vec(),
        exit_code,
    })
}

// ---------------------------------------------------------------------------
// Preview2 (Component Model) 実行パス
// ---------------------------------------------------------------------------

use wasmtime::component::{Component, Linker as ComponentLinker, ResourceTable};
use wasmtime_wasi::{WasiCtx, WasiView};

/// Preview2 Component Model 用の状態
struct ComponentState {
    ctx: WasiCtx,
    table: ResourceTable,
}

impl WasiView for ComponentState {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.ctx
    }
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

/// Component Wasm (.component.wasm) を WASI Preview2 環境で実行し、stdout 出力を返す
///
/// Preview2 の Component Model API を使用して実行する。
/// 入力は Component 形式の Wasm バイナリである必要がある (core module ではない)。
pub fn run_wasm_component(component_bytes: &[u8]) -> Result<String, String> {
    run_wasm_component_with_dir_args_and_stdin(component_bytes, None, &[], "")
}

/// Component Wasm を WASI Preview2 環境で実行 (argv・stdin 付き)
///
/// フル機能の Preview2 実行関数。Component 形式の Wasm バイナリを
/// WASI Preview2 コンテキストで実行する。
pub fn run_wasm_component_with_args_and_stdin(
    component_bytes: &[u8],
    args: &[&str],
    stdin_data: &str,
) -> Result<String, String> {
    let output = run_wasm_component_with_dir_args_and_stdin_capture(
        component_bytes,
        None,
        args,
        stdin_data,
    )?;
    if output.exit_code == 0 {
        Ok(output.stdout)
    } else {
        Err(format!(
            "Component 実行に失敗: exit code {}",
            output.exit_code
        ))
    }
}

/// Component Wasm を WASI Preview2 環境で実行 (ファイルシステム・argv・stdin 付き)
pub fn run_wasm_component_with_dir_args_and_stdin(
    component_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin_data: &str,
) -> Result<String, String> {
    let output =
        run_wasm_component_with_dir_args_and_stdin_capture(component_bytes, dir, args, stdin_data)?;
    if output.exit_code == 0 {
        Ok(output.stdout)
    } else {
        Err(format!(
            "Component 実行に失敗: exit code {}",
            output.exit_code
        ))
    }
}

/// Component Wasm を WASI Preview2 環境で実行し、stdout と exit code を返す
pub fn run_wasm_component_with_dir_args_and_stdin_capture(
    component_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin_data: &str,
) -> Result<ExecutionOutput, String> {
    run_wasm_component_capture(component_bytes, dir, args, StdinMode::Memory(stdin_data))
}

/// Component Wasm を WASI Preview2 環境で実行し、親 stdin を継承した stdout/exit code を返す
pub fn run_wasm_component_with_dir_and_args_inherit_stdin_capture(
    component_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
) -> Result<ExecutionOutput, String> {
    run_wasm_component_capture(component_bytes, dir, args, StdinMode::Inherit)
}

fn run_wasm_component_capture(
    component_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin_mode: StdinMode<'_>,
) -> Result<ExecutionOutput, String> {
    let engine = configured_engine()?;

    let mut linker = ComponentLinker::<ComponentState>::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker)
        .map_err(|e| format!("WASI Preview2 リンクに失敗: {e}"))?;

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(DEFAULT_STDOUT_CAPTURE_BYTES);
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    match stdin_mode {
        StdinMode::Memory(stdin_data) => {
            let stdin = wasmtime_wasi::pipe::MemoryInputPipe::new(stdin_data.as_bytes().to_vec());
            builder.stdin(stdin);
        }
        StdinMode::Inherit => {
            builder.inherit_stdin();
        }
    }
    builder.args(args);
    if let Some(dir_path) = dir {
        builder
            .preopened_dir(
                dir_path,
                ".",
                wasmtime_wasi::DirPerms::all(),
                wasmtime_wasi::FilePerms::all(),
            )
            .map_err(|e| format!("component preopened_dir に失敗: {e}"))?;
    }

    let state = ComponentState {
        ctx: builder.build(),
        table: ResourceTable::new(),
    };
    let mut store = Store::new(&engine, state);

    let component = Component::new(&engine, component_bytes)
        .map_err(|e| format!("Component の読み込みに失敗: {e}"))?;

    let instance = linker
        .instantiate(&mut store, &component)
        .map_err(|e| format!("Component インスタンス化に失敗: {e}"))?;

    let execution = if let Some(run_export) =
        find_component_run_func(&component, &instance, &mut store)
    {
        call_component_run(&mut store, run_export)
    } else {
        // P1 の _start 不在時と同様にエラーを返す
        return Err(
            "Component に run 関数が見つかりません (wasi:cli/run@0.2.x#run または run export が必要)"
                .to_string(),
        );
    };

    drop(store);
    let bytes = stdout
        .try_into_inner()
        .ok_or_else(|| "stdout の取得に失敗".to_string())?;
    let exit_code = execution.map_err(|error| format_component_trap_with_stdout(error, &bytes))?;
    let stdout = decode_stdout_bytes(&bytes)?;
    Ok(ExecutionOutput { stdout, exit_code })
}

struct ComponentRunExport {
    func: wasmtime::component::Func,
    returns_exit_bool: bool,
}

fn find_component_run_func(
    component: &Component,
    instance: &wasmtime::component::Instance,
    store: &mut Store<ComponentState>,
) -> Option<ComponentRunExport> {
    for export_name in ["wasi:cli/run@0.2.3#run", "wasi:cli/run@0.2.0#run"] {
        if let Some(run_func) = instance.get_func(&mut *store, export_name) {
            return Some(ComponentRunExport {
                func: run_func,
                returns_exit_bool: true,
            });
        }
    }

    if let Some(run_func) = instance.get_func(&mut *store, "run") {
        return Some(ComponentRunExport {
            func: run_func,
            returns_exit_bool: false,
        });
    }

    for interface_name in ["wasi:cli/run@0.2.3", "wasi:cli/run@0.2.0"] {
        if let Some((_, run_instance_index)) = component.export_index(None, interface_name)
            && let Some((_, run_func_index)) =
                component.export_index(Some(&run_instance_index), "run")
            && let Some(run_func) = instance.get_func(&mut *store, run_func_index)
        {
            return Some(ComponentRunExport {
                func: run_func,
                returns_exit_bool: true,
            });
        }
    }

    None
}

fn call_component_run(
    store: &mut Store<ComponentState>,
    run_export: ComponentRunExport,
) -> Result<i32, String> {
    if run_export.returns_exit_bool {
        let mut results = [wasmtime::component::Val::Bool(false)];
        let execution = run_export.func.call(&mut *store, &[], &mut results);
        match execution {
            Ok(()) => decode_component_run_result(&results[0]),
            Err(e) => {
                if let Some(exit) = extract_i32_exit(&e) {
                    Ok(exit)
                } else {
                    Err(format!("Component 実行に失敗: {e}"))
                }
            }
        }
    } else {
        let execution = run_export.func.call(&mut *store, &[], &mut []);
        match execution {
            Ok(()) => Ok(0),
            Err(e) => {
                if let Some(exit) = extract_i32_exit(&e) {
                    Ok(exit)
                } else {
                    Err(format!("Component 実行に失敗: {e}"))
                }
            }
        }
    }
}

fn decode_component_run_result(result: &wasmtime::component::Val) -> Result<i32, String> {
    match result {
        wasmtime::component::Val::Bool(false) => Ok(0),
        wasmtime::component::Val::Bool(true) => Ok(1),
        wasmtime::component::Val::Result(Ok(None)) => Ok(0),
        wasmtime::component::Val::Result(Err(None)) => Ok(1),
        _ => Err("Component run の戻り値型が想定外です".to_string()),
    }
}

#[cfg(test)]
#[path = "wasi_runner_tests.rs"]
mod tests;
