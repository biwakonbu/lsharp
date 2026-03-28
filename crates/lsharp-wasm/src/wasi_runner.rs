//! WASI 実行ヘルパー
//!
//! Wasm バイナリを wasmtime の WASI 環境で実行するユーティリティ。
//! driver, e2e テスト, test_runner の 3 箇所で重複していたコードを統合。

use wasmtime::*;
use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

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
}
