//! WASI Preview1 (core module) の実行経路。

use super::{
    DEFAULT_STDOUT_CAPTURE_BYTES, ExecutionOutput, RawExecutionOutput, StdinMode,
    classify_wasi_runtime_failure, configured_engine, decode_stdout_bytes, extract_i32_exit,
};
use wasmtime::*;
use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

/// Wasm バイナリを WASI 環境で実行し、stdout 出力を返す。
pub fn run_wasm_wasi(wasm_bytes: &[u8]) -> Result<String, String> {
    run_wasm_wasi_with_dir_args_and_stdin(wasm_bytes, None, &[], "")
}

/// Wasm バイナリを WASI 環境で実行 (ファイルシステムアクセス付き)。
pub fn run_wasm_wasi_with_dir(
    wasm_bytes: &[u8],
    dir: Option<&std::path::Path>,
) -> Result<String, String> {
    run_wasm_wasi_with_dir_args_and_stdin(wasm_bytes, dir, &[], "")
}

/// Wasm バイナリを WASI 環境で実行 (ファイルシステム・argv 付き)。
pub fn run_wasm_wasi_with_dir_and_args(
    wasm_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
) -> Result<String, String> {
    run_wasm_wasi_with_dir_args_and_stdin(wasm_bytes, dir, args, "")
}

/// Wasm バイナリを WASI 環境で実行 (ファイルシステム・argv・stdin 付き)。
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

/// Wasm バイナリを WASI 環境で実行し、stdout と exit code を返す。
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

/// Wasm バイナリを WASI 環境で実行し、親 stdin を継承した stdout/exit code を返す。
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

/// Wasm バイナリを実行し、UTF-8 decode 前の stdout と exit code を返す。
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
