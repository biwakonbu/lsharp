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
            preview1::run_wasm_wasi_with_dir_args_and_stdin_capture(wasm_bytes, dir, args, stdin)
        }
        WasiMode::Preview2 => preview2::run_wasm_component_with_dir_args_and_stdin_capture(
            wasm_bytes, dir, args, stdin,
        ),
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
            preview1::run_wasm_wasi_with_dir_and_args_inherit_stdin_capture(wasm_bytes, dir, args)
        }
        WasiMode::Preview2 => preview2::run_wasm_component_with_dir_and_args_inherit_stdin_capture(
            wasm_bytes, dir, args,
        ),
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

#[path = "wasi_runner/preview1.rs"]
mod preview1;
#[path = "wasi_runner/preview2.rs"]
mod preview2;

pub use preview1::{
    run_wasm_wasi, run_wasm_wasi_with_dir, run_wasm_wasi_with_dir_and_args,
    run_wasm_wasi_with_dir_and_args_capture_raw,
    run_wasm_wasi_with_dir_and_args_inherit_stdin_capture, run_wasm_wasi_with_dir_args_and_stdin,
    run_wasm_wasi_with_dir_args_and_stdin_capture,
};
pub use preview2::{
    run_wasm_component, run_wasm_component_with_args_and_stdin,
    run_wasm_component_with_dir_and_args_inherit_stdin_capture,
    run_wasm_component_with_dir_args_and_stdin, run_wasm_component_with_dir_args_and_stdin_capture,
};

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

#[cfg(test)]
#[path = "wasi_runner_tests.rs"]
mod tests;
