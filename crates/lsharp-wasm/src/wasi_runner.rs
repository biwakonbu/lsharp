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

/// WASI 実行モードの選択
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasiMode {
    /// WASI Preview1 (wasi_snapshot_preview1) — 既存の実行パス
    Preview1,
    /// WASI Preview2 (Component Model) — 新しい実行パス
    Preview2,
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
    let engine = Engine::default();
    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t)
        .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
    let stdin = wasmtime_wasi::pipe::MemoryInputPipe::new(stdin.as_bytes().to_vec());
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    builder.stdin(stdin);
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
    start
        .call(&mut store, ())
        .map_err(|e| format!("実行に失敗: {e}"))?;

    drop(store);
    let bytes = stdout
        .try_into_inner()
        .ok_or_else(|| "stdout の取得に失敗".to_string())?;
    String::from_utf8(bytes.to_vec()).map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))
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
    run_wasm_component_with_args_and_stdin(component_bytes, &[], "")
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
    let engine = Engine::default();

    let mut linker = ComponentLinker::<ComponentState>::new(&engine);
    wasmtime_wasi::add_to_linker_sync(&mut linker)
        .map_err(|e| format!("WASI Preview2 リンクに失敗: {e}"))?;

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
    let stdin = wasmtime_wasi::pipe::MemoryInputPipe::new(stdin_data.as_bytes().to_vec());
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    builder.stdin(stdin);
    builder.args(args);

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

    // wasi:cli/run@0.2.0 の run 関数を呼び出す
    let func = instance
        .get_func(&mut store, "wasi:cli/run@0.2.0#run")
        .or_else(|| {
            // フォールバック: export された run 関数を探す
            instance.get_func(&mut store, "run")
        });

    match func {
        Some(run_func) => {
            let mut results = [wasmtime::component::Val::Bool(false)];
            run_func
                .call(&mut store, &[], &mut results)
                .map_err(|e| format!("Component 実行に失敗: {e}"))?;
        }
        None => {
            // P1 の _start 不在時と同様にエラーを返す
            return Err("Component に run 関数が見つかりません (wasi:cli/run#run または run export が必要)".to_string());
        }
    }

    drop(store);
    let bytes = stdout
        .try_into_inner()
        .ok_or_else(|| "stdout の取得に失敗".to_string())?;
    String::from_utf8(bytes.to_vec()).map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_wasm_wasi_invalid_bytes() {
        // 不正な Wasm バイナリでエラーが返ること
        let result = run_wasm_wasi(&[0, 1, 2, 3]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Wasm モジュールの読み込みに失敗")
        );
    }

    #[test]
    fn test_run_wasm_wasi_hello() {
        // 実際のコンパイラで hello world を実行
        use lsharp_ir::lower::Lower;
        use lsharp_types::infer::Infer;

        let source = r#"(defn main [] (print 42))"#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lower = Lower::new();
        let module = lower.lower_program(&program, &type_results).unwrap();
        let wasm_bytes = crate::wasi::emit_wasm_wasi(&module).unwrap();

        let result = run_wasm_wasi(&wasm_bytes);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "42");
    }

    #[test]
    fn test_wasi_mode_enum_exists() {
        // WasiMode enum の各バリアントが存在し、区別できること
        let p1 = WasiMode::Preview1;
        let p2 = WasiMode::Preview2;
        assert_ne!(p1, p2);
        assert_eq!(p1, WasiMode::Preview1);
        assert_eq!(p2, WasiMode::Preview2);
    }

    #[test]
    fn test_wasi_mode_debug_display() {
        // WasiMode の Debug 表示が正しいこと
        assert_eq!(format!("{:?}", WasiMode::Preview1), "Preview1");
        assert_eq!(format!("{:?}", WasiMode::Preview2), "Preview2");
    }

    #[test]
    fn test_wasi_mode_copy_clone() {
        // WasiMode が Copy + Clone を実装していること
        let mode = WasiMode::Preview2;
        let copied = mode;
        let cloned = mode.clone();
        assert_eq!(mode, copied);
        assert_eq!(mode, cloned);
    }

    #[test]
    fn test_run_wasm_component_invalid_bytes() {
        // 不正なバイナリで適切なエラーが返ること
        let result = run_wasm_component(&[0, 1, 2, 3]);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Component の読み込みに失敗")
        );
    }

    #[test]
    fn test_run_wasm_component_minimal() {
        // 最小限の Component Wasm (run export なし) はエラーを返すこと
        let component_bytes = build_minimal_component_wasm();
        let result = run_wasm_component(&component_bytes);
        // P1 の _start 不在と同様、run 関数がない component はエラー
        assert!(result.is_err(), "run export なしの component は失敗すべき");
        let err = result.unwrap_err();
        assert!(
            err.contains("run 関数が見つかりません"),
            "エラーメッセージに run 関数不在が含まれること: {err}"
        );
    }

    /// テスト用: 最小限の WASI Component Wasm を構築する
    /// wasm-encoder を使って空の command component を生成
    fn build_minimal_component_wasm() -> Vec<u8> {
        use wasm_encoder::Component;

        // wasm-encoder で最小の component binary を構築
        // 空の component (imports/exports なし) として正常にインスタンス化できる
        let component = Component::new();
        component.finish()
    }
}
