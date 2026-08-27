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
/// 失敗メッセージへ載せる捕捉済み stdout の、先頭と末尾それぞれの上限。
const CAPTURED_STDOUT_HEAD_BYTES: usize = 2048;
const CAPTURED_STDOUT_TAIL_BYTES: usize = 2048;

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
        Err(format_nonzero_exit_error(
            &format!("WASI {mode:?} 実行に失敗"),
            output.exit_code,
            &output.stdout,
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

/// 捕捉済み stdout の先頭と末尾を残した診断表現を返す。
///
/// 末尾を必ず残すのは、CLI の `error: ...` 行が**最後に**出るからである。
/// 先頭だけ切り出す実装にすると、狙った 1 行がちょうど落ちる。
fn render_captured_stdout(stdout: &str) -> String {
    if stdout.is_empty() {
        // 空であること自体が所見になる (出力が fd 2 へ出たか、そもそも出ていない)。
        // 黙って何も足さないと「載せ忘れ」と区別が付かない。
        return "<空>".to_string();
    }
    let bytes = stdout.as_bytes();
    if bytes.len() <= CAPTURED_STDOUT_HEAD_BYTES + CAPTURED_STDOUT_TAIL_BYTES {
        return format!("{stdout:?}");
    }
    let head = String::from_utf8_lossy(&bytes[..CAPTURED_STDOUT_HEAD_BYTES]);
    let tail = String::from_utf8_lossy(&bytes[bytes.len() - CAPTURED_STDOUT_TAIL_BYTES..]);
    let omitted = bytes.len() - CAPTURED_STDOUT_HEAD_BYTES - CAPTURED_STDOUT_TAIL_BYTES;
    format!("{head:?}...<{omitted} bytes 省略>...{tail:?}")
}

/// exit code 非 0 の失敗メッセージを組み立てる。
///
/// **既存メッセージを前置きとして逐語で残し、後ろへ足すだけにする。**
/// 書き換えると、まだ見つかっていない `contains` assertion を壊し得る。
///
/// 捕捉済み stdout を必ず添えるのは、selfhost CLI の `cli-stderr` が
/// 名前に反して fd 2 ではなく `print-string` 経由で fd 1 へ書くためである
/// (`selfhost/src/App/EmbeddedCli.ls`)。診断に要る `error: ...` 行は stdout 側にある。
/// ここで捨てると、テストの失敗メッセージが `exit code 1` だけになって原因が追えない
/// (`ISSUES.md` の `I-91`)。
pub(crate) fn format_nonzero_exit_error(label: &str, exit_code: i32, stdout: &str) -> String {
    format!(
        "{label}: exit code {exit_code}; stdout={}",
        render_captured_stdout(stdout)
    )
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
