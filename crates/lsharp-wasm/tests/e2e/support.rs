#![allow(dead_code)]

//! E2E テスト: L# ソースコード → Wasm コンパイル → wasmtime 実行
//!
//! examples/ ディレクトリのサンプルファイルや手書きのテストケースを
//! 完全なパイプライン（パース → 型チェック → IR → Wasm → 実行）で検証する。
//!
//! ## 検証レベル
//! - `compile_and_run`: フルパイプライン実行（stdout 出力を検証）
//! - `compile_only` + `assert_valid_wasm`: Wasm バイナリ生成まで検証
//! - `typecheck_only`: 型チェックまで検証
//! - `should_fail_typecheck` / `should_fail_parse`: エラーケース検証
//!
//! GC 型（ADT, レコード）を含むコードは wasmtime の GC feature が
//! 未有効のため `compile_only` で検証する。`_compile` サフィックスのテストがこれに該当。

pub(crate) use lsharp_ir::lower::Lower;
pub(crate) use lsharp_types::infer::Infer;
use std::sync::{
    OnceLock,
    atomic::{AtomicUsize, Ordering},
};

const SELFHOST_LSP_RUNTIME_SENTINEL: &str = ";;__SELFHOST_LSP_RUNTIME__";
const WASI_STDOUT_CAPTURE_BYTES: usize = 16 * 1024 * 1024;
const SELFHOST_LSP_RUNTIME_MODULES: &[&str] = &[
    "Token.ls",
    "AST.ls",
    "Lexer.ls",
    "Parser.ls",
    "ModuleResolver.ls",
    "FormatterExpr.ls",
    "FormatterDecl.ls",
    "Formatter.ls",
    "Linter.ls",
    "JsonRpc.ls",
    "LspServerCore.ls",
    "LspServerNav.ls",
    "LspServer.ls",
];
pub(crate) const SELFHOST_APP_MAIN_REPRESENTATIVE_MODULES: &[&str] = &[
    "Main.ls",
    "PipelineSmoke.ls",
    "ModuleResolver.ls",
    "CompilerMode.ls",
    "Token.ls",
    "AST.ls",
    "Lexer.ls",
    "LexerCompat.ls",
    "Parser.ls",
    "MacroExpand.ls",
    "IR.ls",
    "Type.ls",
    "TypeScheme.ls",
    "TypeInferCore.ls",
    "TypeInferFunctions.ls",
    "TypeInferBuiltins.ls",
    "TypeInfer.ls",
    "TypeInferApply.ls",
    "TypeInferBlock.ls",
    "TypeInferPattern.ls",
    "TypeInferRecord.ls",
    "TypeInferRecordDecl.ls",
    "TypeInferAdt.ls",
    "CompilerBase.ls",
    "CompilerSplit.ls",
    "Compiler.ls",
    "WasiBackend.ls",
    "WasmEmit.ls",
    "NativeTarget.ls",
    "NativeCodegen.ls",
    "NativeEmit.ls",
    "Linker.ls",
];
static SELFHOST_FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);
static FIXTURE_RUN_ID: OnceLock<String> = OnceLock::new();
// selfhost acceptance は Wasmtime 側で 64 MiB の wasm stack を許可するため、
// それを包む host thread には十分な余裕を持たせる。
pub(crate) const NATIVE_HARNESS_STACK_BYTES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RuntimeTelemetry {
    pub(crate) heap_ptr: i32,
    pub(crate) heap_start: i32,
    pub(crate) alloc_count: i32,
    pub(crate) gc_collection_count: i32,
    pub(crate) gc_freed_count: i32,
    pub(crate) gc_free_list_count: i32,
    pub(crate) gc_free_list_scan_steps: i32,
    pub(crate) gc_live_alloc_count: i32,
    pub(crate) root_stack_top: i32,
    pub(crate) root_stack_base: i32,
    pub(crate) root_stack_capacity: i32,
    pub(crate) root_slots: [i64; 8],
}

/// ソースコードをパースする
pub(crate) fn parse_for_pipeline(source: &str) -> lsharp_syntax::ast::Program {
    lsharp_syntax::parse(source).unwrap()
}

/// ソースコードをパースし、マクロ展開まで適用する
pub(crate) fn parse_for_expanded_pipeline(source: &str) -> lsharp_syntax::ast::Program {
    lsharp_syntax::parse_and_expand(source).unwrap()
}

/// ソースコードをコンパイルして WASI 環境で実行し、stdout 出力を返す
pub(crate) fn compile_and_run(source: &str) -> String {
    if let Some(result) = try_compile_and_run_lsp_runtime(source) {
        return result.unwrap();
    }
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    run_wasi(&wasm_bytes)
}

/// ソースコードをマクロ展開込みでコンパイルして実行する
pub(crate) fn compile_and_run_expanded(source: &str) -> String {
    let program = parse_for_expanded_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    run_wasi(&wasm_bytes)
}

/// ソースコードをコンパイルしてファイルシステムアクセス付きで実行
pub(crate) fn compile_and_run_with_dir(source: &str, dir: &std::path::Path) -> String {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir(&wasm_bytes, Some(dir)).unwrap()
}

/// ソースコードをコンパイルしてコマンドライン引数付きで実行
pub(crate) fn compile_and_run_with_args(source: &str, args: &[&str]) -> String {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(&wasm_bytes, None, args).unwrap()
}

/// ソースコードをコンパイルしてコマンドライン引数・stdin 付きで実行
pub(crate) fn compile_and_run_with_args_and_stdin(
    source: &str,
    args: &[&str],
    stdin: &str,
) -> String {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin(&wasm_bytes, None, args, stdin)
        .unwrap()
}

/// ソースコードをコンパイルしてファイルシステム・argv 付きで実行
pub(crate) fn compile_and_run_with_dir_and_args(
    source: &str,
    dir: &std::path::Path,
    args: &[&str],
) -> String {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(&wasm_bytes, Some(dir), args).unwrap()
}

/// ソースコードをコンパイルのみ（Wasm バイナリ生成まで）
pub(crate) fn compile_only(source: &str) -> Vec<u8> {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap()
}

/// ドライバの `lsharp compile` と同等の経路でファイルをコンパイルする (エラーは Result)
pub(crate) fn try_compile_file_only(file: &std::path::Path) -> Result<Vec<u8>, String> {
    let file = file.to_path_buf();
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        let source =
            std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let program =
            lsharp_syntax::parse(&source).map_err(|e| format!("{}: {e:?}", file.display()))?;

        let module = if program
            .decls
            .iter()
            .any(|decl| matches!(decl, lsharp_syntax::ast::Decl::ImportDecl { .. }))
        {
            lsharp_ir::compile_multi_file(&file).map_err(|e| format!("{}: {e}", file.display()))?
        } else {
            let mut infer = Infer::new();
            let type_results = infer
                .infer_program(&program)
                .map_err(|e| format!("{}: {e:?}", file.display()))?;
            let mut lower = Lower::new();
            lower
                .lower_program(&program, &type_results)
                .map_err(|e| format!("{}: {e:?}", file.display()))?
        };

        lsharp_wasm::wasi::emit_wasm_wasi(&module).map_err(|e| format!("Wasm: {e:?}"))
    })
}

/// ドライバの `lsharp compile` と同等の経路でファイルをコンパイルする
pub(crate) fn compile_file_only(file: &std::path::Path) -> Vec<u8> {
    try_compile_file_only(file).unwrap()
}

/// エントリ `.ls` をコンパイルして WASI 実行 (エラーは Result)
pub(crate) fn try_compile_and_run_file(path: &std::path::Path) -> Result<String, String> {
    let wasm = try_compile_file_only(path)?;
    lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm).map_err(|e| format!("実行: {e:?}"))
}

/// Wasm バイナリを WASI 環境で実行
pub(crate) fn run_wasi(wasm_bytes: &[u8]) -> String {
    lsharp_wasm::wasi_runner::run_wasm_wasi(wasm_bytes).unwrap()
}

/// Wasmtime の resource limiter で memory.grow 失敗を再現するテスト用実行器。
pub(crate) fn try_compile_and_run_with_memory_limit(
    source: &str,
    memory_limit_bytes: usize,
) -> Result<String, String> {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer
        .infer_program(&program)
        .map_err(|e| format!("型推論エラー: {e:?}"))?;
    let mut lower = Lower::new();
    let module = lower
        .lower_program(&program, &type_results)
        .map_err(|e| format!("IR変換エラー: {e:?}"))?;
    let wasm_bytes =
        lsharp_wasm::wasi::emit_wasm_wasi(&module).map_err(|e| format!("Wasm生成エラー: {e:?}"))?;

    use wasmtime::{Linker, Module, Store, StoreLimitsBuilder};
    use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

    let engine = wasmtime::Engine::default();
    let mut linker = Linker::<(WasiP1Ctx, wasmtime::StoreLimits)>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |state| &mut state.0)
        .map_err(|e| format!("WASI linker 構築に失敗: {e}"))?;

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(WASI_STDOUT_CAPTURE_BYTES);
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    builder.args(&["memory-limit"]);
    let wasi = builder.build_p1();
    let limits = StoreLimitsBuilder::new()
        .memory_size(memory_limit_bytes)
        .build();
    let mut store = Store::new(&engine, (wasi, limits));
    store.limiter(|state| &mut state.1);

    let module =
        Module::new(&engine, wasm_bytes).map_err(|e| format!("Wasm module 構築失敗: {e}"))?;
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("WASI instance 化失敗: {e}"))?;
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| format!("_start export 取得失敗: {e}"))?;
    start.call(&mut store, ()).map_err(|e| {
        let rendered = format!("runtime trap: {e:#}");
        lsharp_wasm::wasi_runner::classify_wasi_runtime_failure(&rendered)
    })?;

    drop(store);
    let bytes = stdout
        .try_into_inner()
        .ok_or_else(|| "stdout 取得失敗: pipe が解放されていない".to_string())?;
    String::from_utf8(bytes.to_vec()).map_err(|e| format!("stdout UTF-8 変換失敗: {e}"))
}

/// Wasmtime の wasm stack 上限で再帰の失敗境界を再現するテスト用実行器。
pub(crate) fn try_compile_and_run_with_wasm_stack_limit(
    source: &str,
    max_wasm_stack: usize,
) -> Result<String, String> {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer
        .infer_program(&program)
        .map_err(|e| format!("型推論エラー: {e:?}"))?;
    let mut lower = Lower::new();
    let module = lower
        .lower_program(&program, &type_results)
        .map_err(|e| format!("IR変換エラー: {e:?}"))?;
    let wasm_bytes =
        lsharp_wasm::wasi::emit_wasm_wasi(&module).map_err(|e| format!("Wasm生成エラー: {e:?}"))?;

    use wasmtime::{Config, Engine, Linker, Module, Store};
    use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

    let mut config = Config::new();
    config.max_wasm_stack(max_wasm_stack);
    let engine = Engine::new(&config).map_err(|e| format!("Wasmtime engine 構築失敗: {e}"))?;
    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |state| state)
        .map_err(|e| format!("WASI linker 構築失敗: {e}"))?;

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(WASI_STDOUT_CAPTURE_BYTES);
    let mut builder = WasiCtxBuilder::new();
    builder.stdout(stdout.clone());
    builder.args(&["recursion-limit"]);
    let wasi = builder.build_p1();
    let mut store = Store::new(&engine, wasi);
    let module =
        Module::new(&engine, wasm_bytes).map_err(|e| format!("Wasm module 構築失敗: {e}"))?;
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("WASI instance 化失敗: {e}"))?;
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .map_err(|e| format!("_start export 取得失敗: {e}"))?;
    start
        .call(&mut store, ())
        .map_err(|e| format!("runtime trap: {e:#}"))?;

    drop(store);
    let bytes = stdout
        .try_into_inner()
        .ok_or_else(|| "stdout 取得失敗: pipe が解放されていない".to_string())?;
    String::from_utf8(bytes.to_vec()).map_err(|e| format!("stdout UTF-8 変換失敗: {e}"))
}

pub(crate) fn compile_and_capture_runtime_telemetry(source: &str) -> (String, RuntimeTelemetry) {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    capture_runtime_telemetry_with_context(&wasm_bytes, None, &["telemetry"], "", false)
}

pub(crate) fn compile_and_capture_runtime_telemetry_after_collect(
    source: &str,
) -> (String, RuntimeTelemetry) {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    capture_runtime_telemetry_with_context(&wasm_bytes, None, &["telemetry"], "", true)
}

pub(crate) fn compile_and_capture_runtime_telemetry_with_dir(
    source: &str,
    dir: &std::path::Path,
) -> (String, RuntimeTelemetry) {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    capture_runtime_telemetry_with_context(&wasm_bytes, Some(dir), &["telemetry"], "", false)
}

pub(crate) fn compile_and_capture_runtime_telemetry_with_args(
    source: &str,
    args: &[&str],
) -> (String, RuntimeTelemetry) {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    capture_runtime_telemetry_with_context(&wasm_bytes, None, args, "", false)
}

pub(crate) fn compile_and_capture_runtime_telemetry_with_args_and_stdin(
    source: &str,
    args: &[&str],
    stdin: &str,
) -> (String, RuntimeTelemetry) {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    capture_runtime_telemetry_with_context(&wasm_bytes, None, args, stdin, false)
}

pub(crate) fn compile_and_capture_runtime_telemetry_after_collect_with_args_and_stdin(
    source: &str,
    args: &[&str],
    stdin: &str,
) -> (String, RuntimeTelemetry) {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    capture_runtime_telemetry_with_context(&wasm_bytes, None, args, stdin, true)
}

pub(crate) fn compile_and_capture_runtime_telemetry_series(
    source: &str,
    iterations: usize,
) -> (String, Vec<RuntimeTelemetry>) {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    capture_runtime_telemetry_series_with_context(&wasm_bytes, None, &["telemetry"], "", iterations)
}

fn capture_runtime_telemetry_with_context(
    wasm_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin_data: &str,
    collect_after_run: bool,
) -> (String, RuntimeTelemetry) {
    use wasmtime::{Linker, Module, Store};
    use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

    let mut config = wasmtime::Config::new();
    config.max_wasm_stack(64 * 1024 * 1024);
    let engine = wasmtime::Engine::new(&config).expect("telemetry Wasmtime engine 構築に失敗");
    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
        .expect("WASI linker 構築に失敗");

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(WASI_STDOUT_CAPTURE_BYTES);
    let stdin = wasmtime_wasi::pipe::MemoryInputPipe::new(stdin_data.as_bytes().to_vec());
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
            .expect("preopened_dir に失敗");
    }
    let mut store = Store::new(&engine, builder.build_p1());

    let module = Module::new(&engine, wasm_bytes).expect("Wasm module 構築に失敗");
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("WASI instance 化に失敗");
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .expect("_start export が必要");
    let exit_code = match start.call(&mut store, ()) {
        Ok(()) => 0,
        Err(err) => extract_i32_exit(&err).unwrap_or_else(|| panic!("_start 実行に失敗: {err}")),
    };
    assert_eq!(exit_code, 0, "_start は exit code 0 で終わるべき");
    if collect_after_run {
        instance
            .get_typed_func::<(), i64>(&mut store, "__lsharp_gc_collect")
            .expect("__lsharp_gc_collect export が必要")
            .call(&mut store, ())
            .expect("__lsharp_gc_collect 実行に失敗");
    }

    let telemetry = read_runtime_telemetry(&instance, &mut store);

    drop(store);
    let bytes = stdout.try_into_inner().expect("stdout 取得に失敗");
    let stdout = String::from_utf8(bytes.to_vec()).expect("stdout UTF-8 変換に失敗");
    (stdout, telemetry)
}

fn capture_runtime_telemetry_series_with_context(
    wasm_bytes: &[u8],
    dir: Option<&std::path::Path>,
    args: &[&str],
    stdin_data: &str,
    iterations: usize,
) -> (String, Vec<RuntimeTelemetry>) {
    use wasmtime::{Linker, Module, Store};
    use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

    let engine = wasmtime::Engine::default();
    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
        .expect("WASI linker 構築に失敗");

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(WASI_STDOUT_CAPTURE_BYTES);
    let stdin = wasmtime_wasi::pipe::MemoryInputPipe::new(stdin_data.as_bytes().to_vec());
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
            .expect("preopened_dir に失敗");
    }
    let mut store = Store::new(&engine, builder.build_p1());

    let module = Module::new(&engine, wasm_bytes).expect("Wasm module 構築に失敗");
    let instance = linker
        .instantiate(&mut store, &module)
        .expect("WASI instance 化に失敗");
    let start = instance
        .get_typed_func::<(), ()>(&mut store, "_start")
        .expect("_start export が必要");
    let gc_collect = instance
        .get_typed_func::<(), i64>(&mut store, "__lsharp_gc_collect")
        .expect("__lsharp_gc_collect export が必要");
    let mut series = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let exit_code = match start.call(&mut store, ()) {
            Ok(()) => 0,
            Err(err) => {
                extract_i32_exit(&err).unwrap_or_else(|| panic!("_start 実行に失敗: {err}"))
            }
        };
        assert_eq!(exit_code, 0, "_start は exit code 0 で終わるべき");
        gc_collect
            .call(&mut store, ())
            .expect("__lsharp_gc_collect 実行に失敗");
        series.push(read_runtime_telemetry(&instance, &mut store));
    }

    drop(store);
    let bytes = stdout.try_into_inner().expect("stdout 取得に失敗");
    let stdout = String::from_utf8(bytes.to_vec()).expect("stdout UTF-8 変換に失敗");
    (stdout, series)
}

fn read_runtime_telemetry(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<wasmtime_wasi::preview1::WasiP1Ctx>,
) -> RuntimeTelemetry {
    let heap_ptr = read_i32_global(instance, store, "__lsharp_heap_ptr");
    let heap_start = read_i32_global(instance, store, "__lsharp_heap_start");
    let alloc_count = read_i32_global(instance, store, "__lsharp_alloc_count");
    let gc_collection_count = read_i32_global(instance, store, "__lsharp_gc_collection_count");
    let gc_freed_count = read_i32_global(instance, store, "__lsharp_gc_freed_count");
    let gc_free_list_count = read_i32_global(instance, store, "__lsharp_gc_free_list_count");
    let gc_free_list_scan_steps =
        read_i32_global(instance, store, "__lsharp_gc_free_list_scan_steps");
    let gc_live_alloc_count = read_i32_global(instance, store, "__lsharp_gc_live_alloc_count");
    let root_stack_top = read_i32_global(instance, store, "__lsharp_root_stack_top");
    let root_stack_base = read_i32_global(instance, store, "__lsharp_root_stack_base");
    let root_stack_capacity = read_i32_global(instance, store, "__lsharp_root_stack_capacity");
    let memory = instance
        .get_memory(&mut *store, "memory")
        .expect("memory export が必要");
    let mut root_slots = [0i64; 8];
    let captured_slots = usize::min(root_stack_top.max(0) as usize, root_slots.len());
    for (slot, value) in root_slots.iter_mut().take(captured_slots).enumerate() {
        *value = read_i64_memory(&memory, store, (root_stack_base as usize) + (slot * 8));
    }
    RuntimeTelemetry {
        heap_ptr,
        heap_start,
        alloc_count,
        gc_collection_count,
        gc_freed_count,
        gc_free_list_count,
        gc_free_list_scan_steps,
        gc_live_alloc_count,
        root_stack_top,
        root_stack_base,
        root_stack_capacity,
        root_slots,
    }
}

fn read_i32_global(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<wasmtime_wasi::preview1::WasiP1Ctx>,
    name: &str,
) -> i32 {
    let global = instance
        .get_global(&mut *store, name)
        .unwrap_or_else(|| panic!("runtime telemetry export が見つからない: {name}"));
    global
        .get(&mut *store)
        .i32()
        .unwrap_or_else(|| panic!("runtime telemetry export が i32 ではない: {name}"))
}

fn extract_i32_exit(err: &wasmtime::Error) -> Option<i32> {
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

fn read_i64_memory(
    memory: &wasmtime::Memory,
    store: &wasmtime::Store<wasmtime_wasi::preview1::WasiP1Ctx>,
    offset: usize,
) -> i64 {
    let data = memory.data(store);
    let bytes: [u8; 8] = data[offset..offset + 8]
        .try_into()
        .expect("runtime telemetry root slot 読み取り範囲外");
    i64::from_le_bytes(bytes)
}

/// 型チェックでエラーになることを検証
pub(crate) fn should_fail_typecheck(source: &str) {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    assert!(infer.infer_program(&program).is_err());
}

/// パースでエラーになることを検証
pub(crate) fn should_fail_parse(source: &str) {
    assert!(lsharp_syntax::parse(source).is_err());
}

/// 型チェックまで成功することを検証（結果が空でないことも確認）
pub(crate) fn typecheck_only(source: &str) {
    let program = parse_for_pipeline(source);
    let mut infer = Infer::new();
    let results = infer.infer_program(&program).unwrap();
    assert!(!results.is_empty(), "型推論結果が空");
}

/// 型チェックまで成功することを検証（マクロ展開込み）
pub(crate) fn typecheck_only_expanded(source: &str) {
    let program = parse_for_expanded_pipeline(source);
    let mut infer = Infer::new();
    let results = infer.infer_program(&program).unwrap();
    assert!(!results.is_empty(), "型推論結果が空");
}

/// Wasm バイナリのマジックバイトとサイズを検証
pub(crate) fn assert_valid_wasm(wasm: &[u8]) {
    assert!(
        wasm.len() > 8,
        "Wasm バイナリが小さすぎる: {} bytes",
        wasm.len()
    );
    assert_eq!(&wasm[0..4], b"\0asm", "Wasm マジックバイトが不正");
}

/// code section の **全関数本体** を個別に検証し、壊れている関数を列挙する。
///
/// `assert_valid_wasm` はマジックバイトと長さしか見ない。`validate_wasm_detailed`
/// (`selfhost_bootstrap_four_layer/part_000.rs`) は `ValidPayload::Func` を捨てるため
/// 関数本体を一つも検証しない。どちらも「緑になること」と「検査していること」が
/// 一致しないので、本体の型検査が要る箇所ではこちらを使う。
pub(crate) fn validate_wasm_function_bodies(wasm: &[u8]) -> Result<(), String> {
    use wasmparser::{Parser, Payload, ValidPayload, Validator, WasmFeatures};

    let mut validator = Validator::new_with_features(WasmFeatures::default());
    let mut pending = Vec::new();
    for payload in Parser::new(0).parse_all(wasm) {
        let payload = payload.map_err(|error| format!("parse error: {error}"))?;
        // body の位置をエラーメッセージへ載せるため、code section entry だけ範囲を控える
        let range = match &payload {
            Payload::CodeSectionEntry(body) => Some(body.range()),
            _ => None,
        };
        let valid = validator.payload(&payload).map_err(|error| {
            format!(
                "validate error at offset {}: {}",
                error.offset(),
                error.message()
            )
        })?;
        if let ValidPayload::Func(to_validate, body) = valid {
            pending.push((range, to_validate, body));
        }
    }

    let mut allocations = wasmparser::FuncValidatorAllocations::default();
    let mut broken = Vec::new();
    for (range, to_validate, body) in pending {
        let index = to_validate.index;
        let mut func_validator = to_validate.into_validator(allocations);
        if let Err(error) = func_validator.validate(&body) {
            let where_ = range
                .map(|r| format!(" body=[{}..{}]", r.start, r.end))
                .unwrap_or_default();
            broken.push(format!(
                "func[{index}]{where_} err@{} (0x{:x}): {}",
                error.offset(),
                error.offset(),
                error.message()
            ));
        }
        allocations = func_validator.into_allocations();
    }

    if broken.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "関数本体の検証に失敗: {} 件\n{}",
            broken.len(),
            broken.join("\n")
        ))
    }
}

/// examples ディレクトリのファイルパスを構築
pub(crate) fn example_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

pub(crate) fn selfhost_project_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(crate) fn selfhost_package_root() -> std::path::PathBuf {
    selfhost_project_root().join("selfhost")
}

pub(crate) fn selfhost_source_path(name: &str) -> std::path::PathBuf {
    selfhost_project_root().join(match name {
        "Main.ls" => "selfhost/src/App/Main.ls",
        "Cli.ls" => "selfhost/src/App/Cli.ls",
        "EmbeddedCli.ls" => "selfhost/src/App/EmbeddedCli.ls",
        "ModuleResolver.ls" => "selfhost/src/App/ModuleResolver.ls",
        "CompilerMode.ls" => "selfhost/src/App/CompilerMode.ls",
        "PipelineSmoke.ls" => "selfhost/src/App/PipelineSmoke.ls",
        "Token.ls" => "selfhost/src/Syntax/Token.ls",
        "AST.ls" => "selfhost/src/Syntax/AST.ls",
        "Span.ls" => "selfhost/src/Syntax/Span.ls",
        "Lexer.ls" => "selfhost/src/Syntax/Lexer.ls",
        "LexerCompat.ls" => "selfhost/src/Syntax/LexerCompat.ls",
        "Parser.ls" => "selfhost/src/Syntax/Parser.ls",
        "MacroExpand.ls" => "selfhost/src/Syntax/MacroExpand.ls",
        "Derive.ls" => "selfhost/src/Syntax/Derive.ls",
        "Hygiene.ls" => "selfhost/src/Syntax/Hygiene.ls",
        "IR.ls" => "selfhost/src/IR/IR.ls",
        "Lower.ls" => "selfhost/src/IR/Lower.ls",
        "LowerExpr.ls" => "selfhost/src/IR/LowerExpr.ls",
        "LowerDecl.ls" => "selfhost/src/IR/LowerDecl.ls",
        "LowerPattern.ls" => "selfhost/src/IR/LowerPattern.ls",
        "Closure.ls" => "selfhost/src/IR/Closure.ls",
        "ModuleGraph.ls" => "selfhost/src/IR/ModuleGraph.ls",
        "Type.ls" => "selfhost/src/Types/Type.ls",
        "TypeScheme.ls" => "selfhost/src/Types/TypeScheme.ls",
        "TypeInferCore.ls" => "selfhost/src/Types/TypeInferCore.ls",
        "TypeInferFunctions.ls" => "selfhost/src/Types/TypeInferFunctions.ls",
        "TypeInferBuiltins.ls" => "selfhost/src/Types/TypeInferBuiltins.ls",
        "TypeInferApply.ls" => "selfhost/src/Types/TypeInferApply.ls",
        "TypeInferBlock.ls" => "selfhost/src/Types/TypeInferBlock.ls",
        "TypeInferPattern.ls" => "selfhost/src/Types/TypeInferPattern.ls",
        "TypeInferRecord.ls" => "selfhost/src/Types/TypeInferRecord.ls",
        "TypeInferRecordDecl.ls" => "selfhost/src/Types/TypeInferRecordDecl.ls",
        "TypeInferAdt.ls" => "selfhost/src/Types/TypeInferAdt.ls",
        "TypeInferAssertions.ls" => "selfhost/src/Types/TypeInferAssertions.ls",
        "MetadataMigration.ls" => "selfhost/src/Types/MetadataMigration.ls",
        "TypeInferSmoke.ls" => "selfhost/src/Types/TypeInferSmoke.ls",
        "TypeInfer.ls" => "selfhost/src/Types/TypeInfer.ls",
        "Constraints.ls" => "selfhost/src/Types/Constraints.ls",
        "MetadataCheck.ls" => "selfhost/src/Types/MetadataCheck.ls",
        "CompilerBase.ls" => "selfhost/src/Backend/Wasm/CompilerBase.ls",
        "CompilerSplit.ls" => "selfhost/src/Backend/Wasm/CompilerSplit.ls",
        "Compiler.ls" => "selfhost/src/Backend/Wasm/Compiler.ls",
        "WasmEmit.ls" => "selfhost/src/Backend/Wasm/WasmEmit.ls",
        "Codegen.ls" => "selfhost/src/Backend/Wasm/Codegen.ls",
        "Emit.ls" => "selfhost/src/Backend/Wasm/Emit.ls",
        "WasiBackend.ls" => "selfhost/src/Backend/Wasm/WasiBackend.ls",
        "WasiRunner.ls" => "selfhost/src/Backend/Wasm/WasiRunner.ls",
        "NativeTarget.ls" => "selfhost/src/Backend/Native/NativeTarget.ls",
        "NativeCodegen.ls" => "selfhost/src/Backend/Native/NativeCodegen.ls",
        "NativeEmit.ls" => "selfhost/src/Backend/Native/NativeEmit.ls",
        "Linker.ls" => "selfhost/src/Backend/Native/Linker.ls",
        "FormatterExpr.ls" => "selfhost/src/Tools/Text/FormatterExpr.ls",
        "FormatterDecl.ls" => "selfhost/src/Tools/Text/FormatterDecl.ls",
        "Formatter.ls" => "selfhost/src/Tools/Text/Formatter.ls",
        "Linter.ls" => "selfhost/src/Tools/Text/Linter.ls",
        "JsonRpc.ls" => "selfhost/src/Tools/Lsp/JsonRpc.ls",
        "LspServerCore.ls" => "selfhost/src/Tools/Lsp/LspServerCore.ls",
        "LspServerNav.ls" => "selfhost/src/Tools/Lsp/LspServerNav.ls",
        "LspServer.ls" => "selfhost/src/Tools/Lsp/LspServer.ls",
        "DocTools.ls" => "selfhost/src/Tools/Doc/DocTools.ls",
        "DocJson.ls" => "selfhost/src/Tools/Doc/DocJson.ls",
        "HtmlDoc.ls" => "selfhost/src/Tools/Doc/HtmlDoc.ls",
        "HtmlLayout.ls" => "selfhost/src/Tools/Doc/HtmlLayout.ls",
        "HtmlTemplate.ls" => "selfhost/src/Tools/Doc/HtmlTemplate.ls",
        "PropertyRunner.ls" => "selfhost/src/Tools/Test/PropertyRunner.ls",
        "TestRunner.ls" => "selfhost/src/Tools/Test/TestRunner.ls",
        "Whitespace.ls" => "selfhost/src/Tools/Validation/Whitespace.ls",
        "IntentSource.ls" => "selfhost/src/Tools/Validation/IntentSource.ls",
        "ReviewIdentity.ls" => "selfhost/src/Tools/Validation/ReviewIdentity.ls",
        "ManifestInput.ls" => "selfhost/src/Tools/Validation/ManifestInput.ls",
        "Evidence.ls" => "selfhost/src/Tools/Validation/Evidence.ls",
        "Stale.ls" => "selfhost/src/Tools/Validation/Stale.ls",
        "GC.ls" => "selfhost/src/Runtime/GC.ls",
        other => panic!("不明な selfhost canonical module path: {other}"),
    })
}

pub(crate) fn selfhost_fixture_module_relative_path(name: &str) -> std::path::PathBuf {
    let src_root = selfhost_project_root().join("selfhost/src");
    selfhost_source_path(name)
        .strip_prefix(&src_root)
        .unwrap_or_else(|_| panic!("fixture relative path へ変換できない: {name}"))
        .to_path_buf()
}

/// selfhost/src/App/Main.ls のパス (import 解決にはマルチファイルコンパイルが必要)
pub(crate) fn selfhost_main_path() -> std::path::PathBuf {
    selfhost_source_path("Main.ls")
}

pub(crate) fn parser_runtime_modules() -> (String, String, String, String) {
    let token_ls = std::fs::read_to_string(selfhost_source_path("Token.ls"))
        .expect("canonical Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(selfhost_source_path("AST.ls"))
        .expect("canonical AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(selfhost_source_path("Lexer.ls"))
        .expect("canonical Lexer.ls が読み込めない");
    let parser_ls = std::fs::read_to_string(selfhost_source_path("Parser.ls"))
        .expect("canonical Parser.ls が読み込めない");
    (token_ls, ast_ls, lexer_ls, parser_ls)
}

pub(crate) fn typescheme_runtime_modules() -> (String, String) {
    let type_ls = std::fs::read_to_string(selfhost_source_path("Type.ls"))
        .expect("canonical Type.ls が読み込めない");
    let type_scheme_ls = std::fs::read_to_string(selfhost_source_path("TypeScheme.ls"))
        .expect("canonical TypeScheme.ls が読み込めない");
    (type_ls, type_scheme_ls)
}

fn selfhost_module_raw(name: &str) -> &'static str {
    match name {
        "Main.ls" => include_str!("../../../../selfhost/src/App/Main.ls"),
        "Cli.ls" => include_str!("../../../../selfhost/src/App/Cli.ls"),
        "ModuleResolver.ls" => include_str!("../../../../selfhost/src/App/ModuleResolver.ls"),
        "CompilerMode.ls" => include_str!("../../../../selfhost/src/App/CompilerMode.ls"),
        "GC.ls" => include_str!("../../../../selfhost/src/Runtime/GC.ls"),
        "PipelineSmoke.ls" => include_str!("../../../../selfhost/src/App/PipelineSmoke.ls"),
        "Token.ls" => include_str!("../../../../selfhost/src/Syntax/Token.ls"),
        "AST.ls" => include_str!("../../../../selfhost/src/Syntax/AST.ls"),
        "Span.ls" => include_str!("../../../../selfhost/src/Syntax/Span.ls"),
        "Lexer.ls" => include_str!("../../../../selfhost/src/Syntax/Lexer.ls"),
        "LexerCompat.ls" => include_str!("../../../../selfhost/src/Syntax/LexerCompat.ls"),
        "Parser.ls" => include_str!("../../../../selfhost/src/Syntax/Parser.ls"),
        "IR.ls" => include_str!("../../../../selfhost/src/IR/IR.ls"),
        "Type.ls" => include_str!("../../../../selfhost/src/Types/Type.ls"),
        "TypeScheme.ls" => include_str!("../../../../selfhost/src/Types/TypeScheme.ls"),
        "TypeInferCore.ls" => include_str!("../../../../selfhost/src/Types/TypeInferCore.ls"),
        "TypeInferFunctions.ls" => {
            include_str!("../../../../selfhost/src/Types/TypeInferFunctions.ls")
        }
        "TypeInferBuiltins.ls" => {
            include_str!("../../../../selfhost/src/Types/TypeInferBuiltins.ls")
        }
        "TypeInferApply.ls" => include_str!("../../../../selfhost/src/Types/TypeInferApply.ls"),
        "TypeInferBlock.ls" => include_str!("../../../../selfhost/src/Types/TypeInferBlock.ls"),
        "TypeInferPattern.ls" => include_str!("../../../../selfhost/src/Types/TypeInferPattern.ls"),
        "TypeInferRecord.ls" => include_str!("../../../../selfhost/src/Types/TypeInferRecord.ls"),
        "TypeInferRecordDecl.ls" => {
            include_str!("../../../../selfhost/src/Types/TypeInferRecordDecl.ls")
        }
        "TypeInferAdt.ls" => include_str!("../../../../selfhost/src/Types/TypeInferAdt.ls"),
        "TypeInferAssertions.ls" => {
            include_str!("../../../../selfhost/src/Types/TypeInferAssertions.ls")
        }
        "MetadataMigration.ls" => {
            include_str!("../../../../selfhost/src/Types/MetadataMigration.ls")
        }
        "TypeInferSmoke.ls" => include_str!("../../../../selfhost/src/Types/TypeInferSmoke.ls"),
        "TypeInfer.ls" => include_str!("../../../../selfhost/src/Types/TypeInfer.ls"),
        "CompilerBase.ls" => include_str!("../../../../selfhost/src/Backend/Wasm/CompilerBase.ls"),
        "CompilerSplit.ls" => {
            include_str!("../../../../selfhost/src/Backend/Wasm/CompilerSplit.ls")
        }
        "Compiler.ls" => include_str!("../../../../selfhost/src/Backend/Wasm/Compiler.ls"),
        "WasiBackend.ls" => include_str!("../../../../selfhost/src/Backend/Wasm/WasiBackend.ls"),
        "WasmEmit.ls" => include_str!("../../../../selfhost/src/Backend/Wasm/WasmEmit.ls"),
        "FormatterExpr.ls" => include_str!("../../../../selfhost/src/Tools/Text/FormatterExpr.ls"),
        "FormatterDecl.ls" => include_str!("../../../../selfhost/src/Tools/Text/FormatterDecl.ls"),
        "Formatter.ls" => include_str!("../../../../selfhost/src/Tools/Text/Formatter.ls"),
        "TestRunner.ls" => include_str!("../../../../selfhost/src/Tools/Test/TestRunner.ls"),
        "DocTools.ls" => include_str!("../../../../selfhost/src/Tools/Doc/DocTools.ls"),
        "DocJson.ls" => include_str!("../../../../selfhost/src/Tools/Doc/DocJson.ls"),
        "HtmlDoc.ls" => include_str!("../../../../selfhost/src/Tools/Doc/HtmlDoc.ls"),
        "HtmlLayout.ls" => include_str!("../../../../selfhost/src/Tools/Doc/HtmlLayout.ls"),
        "HtmlTemplate.ls" => include_str!("../../../../selfhost/src/Tools/Doc/HtmlTemplate.ls"),
        "PropertyRunner.ls" => {
            include_str!("../../../../selfhost/src/Tools/Test/PropertyRunner.ls")
        }
        "JsonRpc.ls" => include_str!("../../../../selfhost/src/Tools/Lsp/JsonRpc.ls"),
        "Linter.ls" => include_str!("../../../../selfhost/src/Tools/Text/Linter.ls"),
        "LspServerCore.ls" => include_str!("../../../../selfhost/src/Tools/Lsp/LspServerCore.ls"),
        "LspServerNav.ls" => include_str!("../../../../selfhost/src/Tools/Lsp/LspServerNav.ls"),
        "LspServer.ls" => include_str!("../../../../selfhost/src/Tools/Lsp/LspServer.ls"),
        "Whitespace.ls" => {
            include_str!("../../../../selfhost/src/Tools/Validation/Whitespace.ls")
        }
        "ManifestInput.ls" => {
            include_str!("../../../../selfhost/src/Tools/Validation/ManifestInput.ls")
        }
        "NativeTarget.ls" => {
            include_str!("../../../../selfhost/src/Backend/Native/NativeTarget.ls")
        }
        "NativeCodegen.ls" => {
            include_str!("../../../../selfhost/src/Backend/Native/NativeCodegen.ls")
        }
        "NativeEmit.ls" => include_str!("../../../../selfhost/src/Backend/Native/NativeEmit.ls"),
        "Linker.ls" => include_str!("../../../../selfhost/src/Backend/Native/Linker.ls"),
        "MacroExpand.ls" => include_str!("../../../../selfhost/src/Syntax/MacroExpand.ls"),
        other => panic!("不明な selfhost モジュール: {other}"),
    }
}

/// selfhost モジュールの埋め込みソースを返す
pub(crate) fn selfhost_module(name: &str) -> &'static str {
    match name {
        "Compiler.ls" => concat!(
            include_str!("../../../../selfhost/src/Backend/Wasm/CompilerBase.ls"),
            "\n",
            include_str!("../../../../selfhost/src/Backend/Wasm/CompilerSplit.ls"),
            "\n",
            include_str!("../../../../selfhost/src/Backend/Wasm/Compiler.ls")
        ),
        other => selfhost_module_raw(other),
    }
}

fn fixture_run_id() -> &'static str {
    FIXTURE_RUN_ID.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("p{}-{nanos}", std::process::id())
    })
}

pub(crate) fn target_fixture_dir(category: &str, prefix: &str, id: usize) -> std::path::PathBuf {
    std::env::var_os("LSHARP_E2E_FIXTURE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target")
        })
        .join(category)
        .join(format!("{prefix}-{}-{id}", fixture_run_id()))
}

fn selfhost_fixture_dir(prefix: &str) -> std::path::PathBuf {
    let id = SELFHOST_FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
    target_fixture_dir("e2e-selfhost-fixtures", prefix, id)
}

fn read_leb_u32_local(bytes: &[u8], pos: &mut usize) -> u32 {
    let mut value = 0_u32;
    let mut shift = 0;
    loop {
        let byte = bytes[*pos];
        *pos += 1;
        value |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
    }
    value
}

fn extract_section_bytes_local(wasm: &[u8], target_id: u8) -> Option<Vec<u8>> {
    use wasmparser::{Parser, Payload};

    for payload in Parser::new(0).parse_all(wasm) {
        let Ok(payload) = payload else {
            return None;
        };
        match payload {
            Payload::TypeSection(reader) if target_id == 1 => {
                let range = reader.range();
                return Some(wasm.get(range.start..range.end)?.to_vec());
            }
            Payload::FunctionSection(reader) if target_id == 3 => {
                let range = reader.range();
                return Some(wasm.get(range.start..range.end)?.to_vec());
            }
            _ => {}
        }
    }
    None
}

struct LocalBoundViolation {
    absolute_index: u32,
    local_index: u32,
    total_locals: u32,
    param_count: u32,
    declared_locals: u32,
}

const WASI_USER_FUNC_BASE: u32 = 26;

fn first_local_bound_violation(wasm: &[u8]) -> Option<LocalBoundViolation> {
    use wasmparser::{Operator, Parser, Payload, TypeRef};

    let type_bytes = extract_section_bytes_local(wasm, 1)?;
    let function_bytes = extract_section_bytes_local(wasm, 3)?;
    let mut pos = 0usize;
    let type_count = read_leb_u32_local(&type_bytes, &mut pos) as usize;
    let mut param_counts = Vec::with_capacity(type_count);
    for _ in 0..type_count {
        if pos >= type_bytes.len() || type_bytes[pos] != 0x60 {
            break;
        }
        pos += 1;
        let param_count = read_leb_u32_local(&type_bytes, &mut pos);
        param_counts.push(param_count);
        pos += param_count as usize;
        let result_count = read_leb_u32_local(&type_bytes, &mut pos);
        pos += result_count as usize;
    }

    let mut pos = 0usize;
    let function_count = read_leb_u32_local(&function_bytes, &mut pos) as usize;
    let mut type_indices = Vec::with_capacity(function_count);
    for _ in 0..function_count {
        type_indices.push(read_leb_u32_local(&function_bytes, &mut pos));
    }

    let mut imported_functions = 0u32;
    for payload in Parser::new(0).parse_all(wasm) {
        let Ok(payload) = payload else {
            return None;
        };
        if let Payload::ImportSection(reader) = payload {
            for import in reader.into_iter().flatten() {
                if matches!(import.ty, TypeRef::Func(_)) {
                    imported_functions += 1;
                }
            }
        }
    }

    let mut func_index = 0u32;
    for payload in Parser::new(0).parse_all(wasm) {
        let Ok(payload) = payload else {
            return None;
        };
        let Payload::CodeSectionEntry(body) = payload else {
            continue;
        };
        let declared_locals = body
            .get_locals_reader()
            .ok()
            .map(|reader| {
                let mut total = 0_u32;
                for local in reader.into_iter().flatten() {
                    total += local.0;
                }
                total
            })
            .unwrap_or(0);
        let type_index = type_indices.get(func_index as usize).copied().unwrap_or(0);
        let param_count = param_counts.get(type_index as usize).copied().unwrap_or(0);
        let total_locals = param_count + declared_locals;
        let mut reader = match body.get_operators_reader() {
            Ok(reader) => reader,
            Err(_) => return None,
        };
        while !reader.eof() {
            let Ok(op) = reader.read() else {
                return None;
            };
            let local_index = match op {
                Operator::LocalGet { local_index }
                | Operator::LocalSet { local_index }
                | Operator::LocalTee { local_index } => Some(local_index),
                _ => None,
            };
            if let Some(local_index) = local_index
                && local_index >= total_locals
            {
                return Some(LocalBoundViolation {
                    absolute_index: imported_functions + func_index,
                    local_index,
                    total_locals,
                    param_count,
                    declared_locals,
                });
            }
        }
        func_index += 1;
    }
    None
}

fn rust_function_local_summary(
    entry_path: &std::path::Path,
    absolute_index: u32,
) -> Option<(String, String)> {
    let module = lsharp_ir::compile_multi_file(entry_path).ok()?;
    let idx = absolute_index.checked_sub(WASI_USER_FUNC_BASE)? as usize;
    let func = module.functions.get(idx)?;
    let total_locals = (func.params.len() + func.locals.len()) as u32;
    let mut max_local = None;
    let mut first_invalid = None;
    for instr in &func.body {
        let local_index = match instr {
            lsharp_ir::Instruction::LocalGet(local_index)
            | lsharp_ir::Instruction::LocalSet(local_index)
            | lsharp_ir::Instruction::LocalTee(local_index) => Some(*local_index),
            _ => None,
        };
        if let Some(local_index) = local_index {
            max_local =
                Some(max_local.map_or(local_index, |current: u32| current.max(local_index)));
            if first_invalid.is_none() && local_index >= total_locals {
                first_invalid = Some(format!("{instr:?}"));
            }
        }
    }
    let body = func
        .body
        .iter()
        .enumerate()
        .map(|(idx, instr)| format!("{idx}:{instr:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let summary = format!(
        "Rust IR params {}, locals {}, max body local {:?}, first invalid {:?}; body [{body}]",
        func.params.len(),
        func.locals.len(),
        max_local,
        first_invalid
    );
    Some((func.name.clone(), summary))
}

/// ModuleGraph::discover が `nearest_src_root` で解決できるよう、`src/<canonical-relative>` に書き出す
fn expand_selfhost_fixture_modules<'a>(modules: &'a [&'a str]) -> Vec<&'a str> {
    let mut expanded = modules.to_vec();
    let needs_compiler_base = expanded.iter().any(|name| {
        matches!(
            *name,
            "CompilerSplit.ls" | "Compiler.ls" | "CompilerMode.ls"
        )
    });
    if needs_compiler_base && !expanded.contains(&"CompilerBase.ls") {
        let insert_at = expanded
            .iter()
            .position(|name| {
                matches!(
                    *name,
                    "CompilerSplit.ls" | "Compiler.ls" | "CompilerMode.ls"
                )
            })
            .unwrap_or(expanded.len());
        expanded.insert(insert_at, "CompilerBase.ls");
    }
    let needs_compiler_split = expanded
        .iter()
        .any(|name| matches!(*name, "Compiler.ls" | "CompilerMode.ls"));
    if needs_compiler_split && !expanded.contains(&"CompilerSplit.ls") {
        let insert_at = expanded
            .iter()
            .position(|name| matches!(*name, "Compiler.ls" | "CompilerMode.ls"))
            .unwrap_or(expanded.len());
        expanded.insert(insert_at, "CompilerSplit.ls");
    }
    expanded
}

fn write_selfhost_fixture_modules(dir: &std::path::Path, modules: &[&str]) -> Result<(), String> {
    let src_root = dir.join("src");
    for name in expand_selfhost_fixture_modules(modules) {
        let rel = selfhost_fixture_module_relative_path(name);
        let path = src_root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&path, selfhost_module_raw(name))
            .map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

fn try_compile_and_run_selfhost_fixture_entry(
    fixture_name: &str,
    modules: &[&str],
    entry_file: &str,
    entry_source: &str,
) -> Result<String, String> {
    let dir = selfhost_fixture_dir(fixture_name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let result = (|| {
        write_selfhost_fixture_modules(&dir, modules)?;
        let entry_path = dir.join(entry_file);
        if let Some(parent) = entry_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&entry_path, entry_source)
            .map_err(|e| format!("{}: {e}", entry_path.display()))?;
        try_compile_and_run_file(&entry_path)
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

pub(crate) fn try_compile_and_run_selfhost_fixture_entry_with_dir_and_args(
    fixture_name: &str,
    modules: &[&str],
    entry_file: &str,
    entry_source: &str,
    args: &[&str],
) -> Result<String, String> {
    let dir = selfhost_fixture_dir(fixture_name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let result = (|| {
        write_selfhost_fixture_modules(&dir, modules)?;
        let entry_path = dir.join(entry_file);
        if let Some(parent) = entry_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&entry_path, entry_source)
            .map_err(|e| format!("{}: {e}", entry_path.display()))?;
        let wasm = try_compile_file_only(&entry_path)?;
        wasmparser::Validator::new()
            .validate_all(&wasm)
            .map_err(|e| {
                if let Some(detail) = first_local_bound_violation(&wasm) {
                    let entry_path_for_name = entry_path.clone();
                    let function_detail = run_with_expanded_stack(
                        NATIVE_HARNESS_STACK_BYTES,
                        move || rust_function_local_summary(&entry_path_for_name, detail.absolute_index),
                    );
                    match function_detail {
                        Some((name, rust_summary)) => format!(
                            "Wasm validate: {e}; function {} ({name}): local {} >= total_locals {} (params {}, declared {}); {rust_summary}",
                            detail.absolute_index,
                            detail.local_index,
                            detail.total_locals,
                            detail.param_count,
                            detail.declared_locals
                        ),
                        None => format!(
                            "Wasm validate: {e}; function {}: local {} >= total_locals {} (params {}, declared {})",
                            detail.absolute_index,
                            detail.local_index,
                            detail.total_locals,
                            detail.param_count,
                            detail.declared_locals
                        ),
                    }
                } else {
                    format!("Wasm validate: {e}")
                }
            })?;
        lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(&wasm, Some(&dir), args)
            .map_err(|e| format!("実行: {e:?}"))
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

pub(crate) fn try_compile_and_run_selfhost_fixture_entry_keep_dir_with_args(
    fixture_name: &str,
    modules: &[&str],
    entry_file: &str,
    entry_source: &str,
    args: &[&str],
) -> Result<(std::path::PathBuf, String), String> {
    let (dir, result) = try_compile_and_run_selfhost_fixture_entry_preserve_dir_with_args(
        fixture_name,
        modules,
        entry_file,
        entry_source,
        args,
    )?;
    match result {
        Ok(output) => Ok((dir, output)),
        Err(err) => {
            let _ = std::fs::remove_dir_all(&dir);
            Err(err)
        }
    }
}

pub(crate) fn try_compile_and_run_selfhost_fixture_entry_keep_dir_with_args_raw(
    fixture_name: &str,
    modules: &[&str],
    entry_file: &str,
    entry_source: &str,
    args: &[&str],
) -> Result<(std::path::PathBuf, Vec<u8>), String> {
    let dir = selfhost_fixture_dir(fixture_name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let result = (|| {
        write_selfhost_fixture_modules(&dir, modules)?;
        let entry_path = dir.join(entry_file);
        if let Some(parent) = entry_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&entry_path, entry_source)
            .map_err(|e| format!("{}: {e}", entry_path.display()))?;
        let wasm = try_compile_file_only(&entry_path)?;
        wasmparser::Validator::new()
            .validate_all(&wasm)
            .map_err(|e| format!("Wasm validate: {e}"))?;
        lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args_capture_raw(
            &wasm,
            Some(&dir),
            args,
        )
        .map(|output| output.stdout_bytes)
        .map_err(|e| format!("実行: {e:?}"))
    })();
    match result {
        Ok(output) => Ok((dir, output)),
        Err(err) => {
            let _ = std::fs::remove_dir_all(&dir);
            Err(err)
        }
    }
}

pub(crate) fn try_compile_and_run_selfhost_fixture_entry_preserve_dir_with_args(
    fixture_name: &str,
    modules: &[&str],
    entry_file: &str,
    entry_source: &str,
    args: &[&str],
) -> Result<(std::path::PathBuf, Result<String, String>), String> {
    let dir = selfhost_fixture_dir(fixture_name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let result = (|| {
        write_selfhost_fixture_modules(&dir, modules)?;
        let entry_path = dir.join(entry_file);
        if let Some(parent) = entry_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&entry_path, entry_source)
            .map_err(|e| format!("{}: {e}", entry_path.display()))?;
        let wasm = try_compile_file_only(&entry_path)?;
        wasmparser::Validator::new()
            .validate_all(&wasm)
            .map_err(|e| format!("Wasm validate: {e}"))?;
        lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(&wasm, Some(&dir), args)
            .map_err(|e| format!("実行: {e:?}"))
    })();
    Ok((dir, result))
}

pub(crate) fn try_compile_and_run_selfhost_fixture_entry_with_dir_and_args_raw(
    fixture_name: &str,
    modules: &[&str],
    entry_file: &str,
    entry_source: &str,
    args: &[&str],
) -> Result<Vec<u8>, String> {
    let dir = selfhost_fixture_dir(fixture_name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let result = (|| {
        write_selfhost_fixture_modules(&dir, modules)?;
        let entry_path = dir.join(entry_file);
        if let Some(parent) = entry_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&entry_path, entry_source)
            .map_err(|e| format!("{}: {e}", entry_path.display()))?;
        let wasm = try_compile_file_only(&entry_path)?;
        wasmparser::Validator::new()
            .validate_all(&wasm)
            .map_err(|e| format!("Wasm validate: {e}"))?;
        lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args_capture_raw(
            &wasm,
            Some(&dir),
            args,
        )
        .map(|output| output.stdout_bytes)
        .map_err(|e| format!("実行: {e:?}"))
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn try_compile_and_run_selfhost_fixture_module(
    fixture_name: &str,
    modules: &[&str],
    entry_file: &str,
) -> Result<String, String> {
    let dir = selfhost_fixture_dir(fixture_name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let result = (|| {
        write_selfhost_fixture_modules(&dir, modules)?;
        try_compile_and_run_file(&dir.join(entry_file))
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn try_compile_and_run_lsp_runtime(source: &str) -> Option<Result<String, String>> {
    let harness = source.strip_prefix(SELFHOST_LSP_RUNTIME_SENTINEL)?;
    let harness = harness.trim_start_matches('\n');
    if harness.trim().is_empty() {
        Some(try_compile_and_run_selfhost_fixture_module(
            "lsp-runtime",
            SELFHOST_LSP_RUNTIME_MODULES,
            "src/Tools/Lsp/LspServer.ls",
        ))
    } else {
        let entry_source = format!(
            "(module App.Main)\n(import Tools.Lsp.LspServerCore)\n(import Tools.Lsp.LspServerNav)\n(import Tools.Lsp.LspServer)\n{harness}"
        );
        Some(try_compile_and_run_selfhost_fixture_entry(
            "lsp-harness",
            SELFHOST_LSP_RUNTIME_MODULES,
            "src/App/Main.ls",
            &entry_source,
        ))
    }
}

/// bundle 化に伴うソース正規化。
///
/// bundle は複数モジュールを 1 本の束として渡すため、モジュール間の `import` 行は
/// 残せない。**この関数が正規化の単一正本**であり、bundle 生成側と検査側の両方から
/// 呼ぶこと。片側だけを直すと検査が黙って陳腐化する (`TESTGATE-02`)。
pub(crate) fn normalize_selfhost_bundle_source(source: &str) -> String {
    source.replace("(import Types.TypeInfer)\n", "")
}

fn cached_selfhost_bundle(cell: &'static OnceLock<String>, modules: &[&str]) -> &'static str {
    cell.get_or_init(|| {
        expand_selfhost_fixture_modules(modules)
            .iter()
            .map(|name| {
                let path = selfhost_source_path(name);
                let source = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                    panic!("selfhost bundle 読み込み失敗 {}: {}", path.display(), e)
                });
                normalize_selfhost_bundle_source(&source)
            })
            .collect::<Vec<_>>()
            .join("\n")
    })
}

static SELFHOST_LEXER_RUNTIME_BUNDLE: OnceLock<String> = OnceLock::new();
static SELFHOST_PARSER_RUNTIME_BUNDLE: OnceLock<String> = OnceLock::new();
static SELFHOST_PARSER_TYPEINFER_RUNTIME_BUNDLE: OnceLock<String> = OnceLock::new();
static SELFHOST_TYPEINFER_RUNTIME_BUNDLE: OnceLock<String> = OnceLock::new();
static SELFHOST_TEST_RUNNER_RUNTIME_BUNDLE: OnceLock<String> = OnceLock::new();
static SELFHOST_MIGRATION_RUNTIME_BUNDLE: OnceLock<String> = OnceLock::new();
static SELFHOST_CLI_RUNTIME_BUNDLE: OnceLock<String> = OnceLock::new();
static SELFHOST_EMBEDDED_CLI_RUNTIME_BUNDLE: OnceLock<String> = OnceLock::new();
static SELFHOST_NATIVE_CODEGEN_BUNDLE: OnceLock<String> = OnceLock::new();

pub(crate) fn selfhost_lexer_runtime_bundle() -> &'static str {
    cached_selfhost_bundle(&SELFHOST_LEXER_RUNTIME_BUNDLE, &["Token.ls", "Lexer.ls"])
}

pub(crate) fn selfhost_parser_runtime_bundle() -> &'static str {
    cached_selfhost_bundle(
        &SELFHOST_PARSER_RUNTIME_BUNDLE,
        &[
            "Token.ls",
            "AST.ls",
            "Lexer.ls",
            "LexerCompat.ls",
            "Parser.ls",
        ],
    )
}

/// selfhost parser と型推論を一体で検証するための最小 runtime bundle
pub(crate) fn selfhost_parser_typeinfer_runtime_bundle() -> &'static str {
    cached_selfhost_bundle(
        &SELFHOST_PARSER_TYPEINFER_RUNTIME_BUNDLE,
        &[
            "Token.ls",
            "AST.ls",
            "Lexer.ls",
            "LexerCompat.ls",
            "Parser.ls",
            "Type.ls",
            "TypeScheme.ls",
            "TypeInferCore.ls",
            "TypeInferFunctions.ls",
            "TypeInferBuiltins.ls",
            "TypeInfer.ls",
            "TypeInferApply.ls",
            "TypeInferBlock.ls",
            "TypeInferPattern.ls",
            "TypeInferRecord.ls",
            "TypeInferRecordDecl.ls",
            "TypeInferAdt.ls",
            "TypeInferAssertions.ls",
            "MetadataMigration.ls",
        ],
    )
}

pub(crate) fn selfhost_typeinfer_runtime_bundle() -> &'static str {
    cached_selfhost_bundle(
        &SELFHOST_TYPEINFER_RUNTIME_BUNDLE,
        &[
            "Token.ls",
            "AST.ls",
            "Lexer.ls",
            "LexerCompat.ls",
            "Parser.ls",
            "Type.ls",
            "TypeScheme.ls",
            "TypeInferCore.ls",
            "TypeInferFunctions.ls",
            "TypeInferBuiltins.ls",
            "TypeInfer.ls",
            "TypeInferApply.ls",
            "TypeInferBlock.ls",
            "TypeInferPattern.ls",
            "TypeInferRecord.ls",
            "TypeInferRecordDecl.ls",
            "TypeInferAdt.ls",
            "TypeInferAssertions.ls",
        ],
    )
}

/// selfhost TestRunner の直接 projection を検証するための最小 runtime bundle
pub(crate) fn selfhost_test_runner_runtime_bundle() -> &'static str {
    cached_selfhost_bundle(
        &SELFHOST_TEST_RUNNER_RUNTIME_BUNDLE,
        &[
            "Token.ls",
            "AST.ls",
            "Lexer.ls",
            "LexerCompat.ls",
            "Parser.ls",
            "PropertyRunner.ls",
            "TestRunner.ls",
        ],
    )
}

/// selfhost migration classifier と raw contract scanner を検証する最小 runtime bundle
pub(crate) fn selfhost_migration_runtime_bundle() -> &'static str {
    cached_selfhost_bundle(
        &SELFHOST_MIGRATION_RUNTIME_BUNDLE,
        &[
            "Token.ls",
            "AST.ls",
            "Lexer.ls",
            "LexerCompat.ls",
            "Parser.ls",
            "Type.ls",
            "TypeScheme.ls",
            "TypeInferCore.ls",
            "TypeInferFunctions.ls",
            "TypeInferBuiltins.ls",
            "TypeInfer.ls",
            "TypeInferApply.ls",
            "TypeInferBlock.ls",
            "TypeInferPattern.ls",
            "TypeInferRecord.ls",
            "TypeInferRecordDecl.ls",
            "TypeInferAdt.ls",
            "TypeInferAssertions.ls",
            "MetadataMigration.ls",
            "PropertyRunner.ls",
            "TestRunner.ls",
        ],
    )
}

/// selfhost/src/App/Cli.ls を直接実行するための最小 runtime bundle
pub(crate) fn selfhost_cli_runtime_bundle() -> &'static str {
    cached_selfhost_bundle(
        &SELFHOST_CLI_RUNTIME_BUNDLE,
        &[
            "Token.ls",
            "AST.ls",
            "Lexer.ls",
            "Parser.ls",
            "IR.ls",
            "Type.ls",
            "TypeScheme.ls",
            "TypeInferCore.ls",
            "TypeInferFunctions.ls",
            "TypeInferBuiltins.ls",
            "TypeInfer.ls",
            "TypeInferApply.ls",
            "TypeInferBlock.ls",
            "TypeInferPattern.ls",
            "TypeInferRecord.ls",
            "TypeInferRecordDecl.ls",
            "TypeInferAdt.ls",
            "TypeInferAssertions.ls",
            "MetadataMigration.ls",
            "Compiler.ls",
            "WasiBackend.ls",
            "WasmEmit.ls",
            "ModuleResolver.ls",
            "CompilerMode.ls",
            "FormatterExpr.ls",
            "FormatterDecl.ls",
            "Formatter.ls",
            "PropertyRunner.ls",
            "TestRunner.ls",
            "DocTools.ls",
            "DocJson.ls",
            "JsonRpc.ls",
            "LspServerCore.ls",
            "LspServerNav.ls",
            "LspServer.ls",
            "Whitespace.ls",
            "IntentSource.ls",
            "ReviewIdentity.ls",
            "ManifestInput.ls",
            "Evidence.ls",
            "Stale.ls",
            "Cli.ls",
        ],
    )
}

/// selfhost/src/App/EmbeddedCli.ls を直接実行するための最小 runtime bundle
pub(crate) fn selfhost_embedded_cli_runtime_bundle() -> &'static str {
    cached_selfhost_bundle(
        &SELFHOST_EMBEDDED_CLI_RUNTIME_BUNDLE,
        &[
            "Token.ls",
            "AST.ls",
            "Lexer.ls",
            "Parser.ls",
            "IR.ls",
            "Type.ls",
            "TypeScheme.ls",
            "TypeInferCore.ls",
            "TypeInferFunctions.ls",
            "TypeInferBuiltins.ls",
            "TypeInfer.ls",
            "TypeInferApply.ls",
            "TypeInferBlock.ls",
            "TypeInferPattern.ls",
            "TypeInferRecord.ls",
            "TypeInferRecordDecl.ls",
            "TypeInferAdt.ls",
            "TypeInferAssertions.ls",
            "MetadataMigration.ls",
            "Compiler.ls",
            "WasiBackend.ls",
            "WasmEmit.ls",
            "ModuleResolver.ls",
            "CompilerMode.ls",
            "FormatterExpr.ls",
            "FormatterDecl.ls",
            "Formatter.ls",
            "PropertyRunner.ls",
            "TestRunner.ls",
            "DocTools.ls",
            "DocJson.ls",
            "JsonRpc.ls",
            "LspServerCore.ls",
            "LspServerNav.ls",
            "LspServer.ls",
            "Whitespace.ls",
            "IntentSource.ls",
            "ReviewIdentity.ls",
            "Evidence.ls",
            "Stale.ls",
            "EmbeddedCli.ls",
        ],
    )
}

pub(crate) fn selfhost_lsp_runtime_bundle() -> &'static str {
    SELFHOST_LSP_RUNTIME_SENTINEL
}

/// native code generation (NativeCodegen + NativeEmit + NativeTarget)
pub(crate) fn selfhost_native_codegen_bundle() -> &'static str {
    cached_selfhost_bundle(
        &SELFHOST_NATIVE_CODEGEN_BUNDLE,
        &["NativeTarget.ls", "NativeCodegen.ls", "NativeEmit.ls"],
    )
}

/// エントリ `.ls` ファイルから依存を解決してコンパイルし、WASI 実行結果を返す
pub(crate) fn compile_and_run_file(path: &std::path::Path) -> String {
    try_compile_and_run_file(path).unwrap()
}

/// selfhost fixture をコンパイルして実行し、runtime telemetry も取得する。
pub(crate) fn compile_and_capture_selfhost_fixture_runtime_telemetry(
    fixture_name: &str,
    modules: &[&str],
    entry_file: &str,
    entry_source: &str,
) -> (String, RuntimeTelemetry) {
    let dir = selfhost_fixture_dir(fixture_name);
    std::fs::create_dir_all(&dir).expect("selfhost telemetry fixture dir 作成失敗");
    let result = (|| {
        write_selfhost_fixture_modules(&dir, modules)
            .expect("selfhost telemetry modules 書き込み失敗");
        let entry_path = dir.join(entry_file);
        if let Some(parent) = entry_path.parent() {
            std::fs::create_dir_all(parent).expect("selfhost telemetry entry parent 作成失敗");
        }
        std::fs::write(&entry_path, entry_source).expect("selfhost telemetry entry 書き込み失敗");
        let wasm =
            try_compile_file_only(&entry_path).expect("selfhost telemetry fixture compile 失敗");
        wasmparser::Validator::new()
            .validate_all(&wasm)
            .expect("selfhost telemetry fixture Wasm validate 失敗");
        capture_runtime_telemetry_with_context(&wasm, Some(&dir), &["telemetry"], "", false)
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// 深い selfhost native harness 用に大きめの stack でクロージャを実行する。
pub(crate) fn run_with_expanded_stack<T, F>(stack_size: usize, f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    match std::thread::Builder::new()
        .stack_size(stack_size)
        .spawn(f)
        .expect("expanded stack thread 起動失敗")
        .join()
    {
        Ok(value) => value,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

// =====================================================// ブートストラップ検証: セルフホストモジュールの個別コンパイル・実行
// =====================================================
/// インラインソースからフルパイプラインを実行する。
/// 本番のブートストラップ検証は `try_compile_and_run_file`（マルチファイル・import 経路）を主とする。
/// 最小再現・スニペット専用の将来テスト用に残す。
#[allow(dead_code)]
pub(crate) fn try_compile_and_run(source: &str) -> Result<String, String> {
    let program = lsharp_syntax::parse(source).map_err(|e| format!("パースエラー: {:?}", e))?;
    let mut infer = Infer::new();
    let type_results = infer
        .infer_program(&program)
        .map_err(|e| format!("型推論エラー: {:?}", e))?;
    let mut lower = Lower::new();
    let module = lower
        .lower_program(&program, &type_results)
        .map_err(|e| format!("IR変換エラー: {:?}", e))?;
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module)
        .map_err(|e| format!("Wasm生成エラー: {:?}", e))?;
    lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes).map_err(|e| format!("実行エラー: {:?}", e))
}

// === P3-3: メタデータテスト実行評価 E2E テスト ===

/// メタデータテスト用ヘルパー: テストプログラムを生成・コンパイル・実行して結果を返す
pub(crate) fn run_metadata_tests(source: &str) -> Vec<lsharp_wasm::test_runner::TestResult> {
    let program = lsharp_syntax::parse(source).unwrap();
    let tests = lsharp_types::metadata_check::generate_tests(&program);
    let test_source = lsharp_wasm::test_runner::generate_test_program(&program, &tests);

    let test_program = lsharp_syntax::parse(&test_source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&test_program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&test_program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes).unwrap();

    lsharp_wasm::test_runner::parse_test_output(&output, &tests, &program)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_support_selfhost_source_path_prefers_canonical_tree() {
        assert!(
            selfhost_source_path("Main.ls").ends_with("selfhost/src/App/Main.ls"),
            "Main.ls は canonical entrypoint を指すべき"
        );
        assert!(
            selfhost_source_path("Cli.ls").ends_with("selfhost/src/App/Cli.ls"),
            "Cli.ls は App/Cli.ls を指すべき"
        );
        assert!(
            selfhost_source_path("ModuleResolver.ls")
                .ends_with("selfhost/src/App/ModuleResolver.ls"),
            "ModuleResolver.ls は App/ModuleResolver.ls を指すべき"
        );
        assert!(
            selfhost_source_path("CompilerMode.ls").ends_with("selfhost/src/App/CompilerMode.ls"),
            "CompilerMode.ls は App/CompilerMode.ls を指すべき"
        );
        assert!(
            selfhost_source_path("PipelineSmoke.ls").ends_with("selfhost/src/App/PipelineSmoke.ls"),
            "PipelineSmoke.ls は App/PipelineSmoke.ls を指すべき"
        );
        assert!(
            selfhost_source_path("TypeInfer.ls").ends_with("selfhost/src/Types/TypeInfer.ls"),
            "TypeInfer.ls は Types/TypeInfer.ls を指すべき"
        );
        assert!(
            selfhost_source_path("TypeInferFunctions.ls")
                .ends_with("selfhost/src/Types/TypeInferFunctions.ls"),
            "TypeInferFunctions.ls は Types/TypeInferFunctions.ls を指すべき"
        );
        assert!(
            selfhost_source_path("TypeInferBuiltins.ls")
                .ends_with("selfhost/src/Types/TypeInferBuiltins.ls"),
            "TypeInferBuiltins.ls は Types/TypeInferBuiltins.ls を指すべき"
        );
        assert!(
            selfhost_source_path("TypeInferSmoke.ls")
                .ends_with("selfhost/src/Types/TypeInferSmoke.ls"),
            "TypeInferSmoke.ls は Types/TypeInferSmoke.ls を指すべき"
        );
        assert!(
            selfhost_source_path("CompilerBase.ls")
                .ends_with("selfhost/src/Backend/Wasm/CompilerBase.ls"),
            "CompilerBase.ls は Backend/Wasm/CompilerBase.ls を指すべき"
        );
        assert!(
            selfhost_source_path("CompilerSplit.ls")
                .ends_with("selfhost/src/Backend/Wasm/CompilerSplit.ls"),
            "CompilerSplit.ls は Backend/Wasm/CompilerSplit.ls を指すべき"
        );
        assert!(
            selfhost_source_path("WasmEmit.ls").ends_with("selfhost/src/Backend/Wasm/WasmEmit.ls"),
            "WasmEmit.ls は Backend/Wasm/WasmEmit.ls を指すべき"
        );
    }

    #[test]
    fn test_support_selfhost_module_reads_canonical_sources() {
        assert!(selfhost_module("Main.ls").contains("(module App.Main)"));
        assert!(selfhost_module("Cli.ls").contains("(module App.Cli)"));
        assert!(selfhost_module("ModuleResolver.ls").contains("(module App.ModuleResolver)"));
        assert!(selfhost_module("CompilerMode.ls").contains("(module App.CompilerMode)"));
        assert!(selfhost_module("CompilerBase.ls").contains("(module Backend.Wasm.CompilerBase)"));
        assert!(
            selfhost_module("CompilerSplit.ls").contains("(module Backend.Wasm.CompilerSplit)")
        );
        assert!(selfhost_module("Compiler.ls").contains("(module Backend.Wasm.CompilerBase)"));
        assert!(selfhost_module("Compiler.ls").contains("(module Backend.Wasm.CompilerSplit)"));
        assert!(selfhost_module("PipelineSmoke.ls").contains("(module App.PipelineSmoke)"));
        assert!(
            selfhost_module("TypeInferFunctions.ls").contains("(module Types.TypeInferFunctions)")
        );
        assert!(
            selfhost_module("TypeInferBuiltins.ls").contains("(module Types.TypeInferBuiltins)")
        );
        assert!(selfhost_module("TypeInferSmoke.ls").contains("(module Types.TypeInferSmoke)"));
        assert!(selfhost_module("TypeInfer.ls").contains("(module Types.TypeInfer)"));
    }

    #[test]
    fn test_support_selfhost_cli_runtime_bundle_cached() {
        let first = selfhost_cli_runtime_bundle();
        let second = selfhost_cli_runtime_bundle();
        assert_eq!(first, second);
        assert_eq!(first.as_ptr(), second.as_ptr());
        assert!(first.contains("(module App.Cli)"));
        assert!(first.contains("(defn main []"));
        assert!(first.contains("(module Tools.Validation.ManifestInput)"));
        assert!(first.contains(&bundle_expectation("CompilerSplit.ls")));
    }

    /// bundle へ入る際の期待テキスト。生ソースではなく、bundle と同じ正規化を通す。
    /// ここを `selfhost_module()` の生テキストに戻すと `TESTGATE-02` が再発する。
    fn bundle_expectation(name: &str) -> String {
        normalize_selfhost_bundle_source(selfhost_module(name))
            .trim()
            .to_string()
    }

    /// 正規化が実際に効いていることを直接固定する。
    ///
    /// これが無いと `bundle_expectation()` を生ソースへ戻しても
    /// 「両側が生ソース」で静かに通り続ける経路が残る (`TESTGATE-02` の再発経路)。
    #[test]
    fn test_support_bundle_normalization_drops_shared_import_line() {
        let raw = selfhost_module("TypeInferApply.ls");
        assert!(
            raw.contains("(import Types.TypeInfer)\n"),
            "前提が崩れている: 生ソースが import 行を持たない"
        );

        let bundle = selfhost_typeinfer_runtime_bundle();
        assert!(
            !bundle.contains("(import Types.TypeInfer)\n"),
            "bundle 側の正規化が効いていない"
        );
        assert!(
            !bundle.contains(raw.trim()),
            "生ソースの verbatim 包含が成立してしまっている。正規化の前提が変わったら本 test ごと見直すこと"
        );
    }

    #[test]
    fn test_support_selfhost_typeinfer_runtime_bundle_cached() {
        let bundle = selfhost_typeinfer_runtime_bundle();
        assert!(bundle.contains(&bundle_expectation("TypeInferFunctions.ls")));
        assert!(bundle.contains(&bundle_expectation("TypeInferBuiltins.ls")));
        assert!(bundle.contains(&bundle_expectation("TypeInferApply.ls")));
        assert!(bundle.contains(&bundle_expectation("TypeInferBlock.ls")));
        assert!(bundle.contains(&bundle_expectation("TypeInferPattern.ls")));
        assert!(bundle.contains(&bundle_expectation("TypeInferRecord.ls")));
        assert!(bundle.contains(&bundle_expectation("TypeInferRecordDecl.ls")));
        assert!(bundle.contains(&bundle_expectation("TypeInferAdt.ls")));
        assert!(bundle.contains(&bundle_expectation("TypeInfer.ls")));
        assert_eq!(
            bundle.as_ptr(),
            selfhost_typeinfer_runtime_bundle().as_ptr()
        );
    }

    #[test]
    fn test_support_selfhost_parser_typeinfer_runtime_bundle_cached() {
        let bundle = selfhost_parser_typeinfer_runtime_bundle();
        assert!(bundle.contains(&bundle_expectation("Parser.ls")));
        assert!(bundle.contains(&bundle_expectation("TypeInfer.ls")));
        assert_eq!(
            bundle.as_ptr(),
            selfhost_parser_typeinfer_runtime_bundle().as_ptr()
        );
    }

    #[test]
    fn test_support_target_fixture_dir_is_process_scoped() {
        let first = target_fixture_dir("e2e-native-fixtures", "native-host-bytes", 0);
        let second = target_fixture_dir("e2e-native-fixtures", "native-host-bytes", 1);

        let first = first.to_string_lossy();
        let second = second.to_string_lossy();
        let expected_root = std::env::var_os("LSHARP_E2E_FIXTURE_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target")
            });
        assert!(
            first.contains(&format!("{}/e2e-native-fixtures/", expected_root.display())),
            "fixture dir は default target または LSHARP_E2E_FIXTURE_ROOT 配下に作るべき: {first}"
        );
        assert!(first.contains("/native-host-bytes-"));
        assert!(first.ends_with("-0"));
        assert!(second.ends_with("-1"));
        assert_ne!(first, second, "counter が違えば fixture dir も変わるべき");
    }
}
