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
const SELFHOST_LSP_RUNTIME_MODULES: &[&str] = &[
    "Token.ls",
    "AST.ls",
    "Lexer.ls",
    "Parser.ls",
    "Formatter.ls",
    "Linter.ls",
    "JsonRpc.ls",
    "LspServer.ls",
];
static SELFHOST_FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);

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
    let source = std::fs::read_to_string(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let program =
        lsharp_syntax::parse(&source).map_err(|e| format!("{}: {e:?}", file.display()))?;

    let module = if program
        .decls
        .iter()
        .any(|decl| matches!(decl, lsharp_syntax::ast::Decl::ImportDecl { .. }))
    {
        lsharp_ir::compile_multi_file(file).map_err(|e| format!("{}: {e}", file.display()))?
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
        "ModuleResolver.ls" => "selfhost/src/App/ModuleResolver.ls",
        "CompilerMode.ls" => "selfhost/src/App/CompilerMode.ls",
        "PipelineSmoke.ls" => "selfhost/src/App/PipelineSmoke.ls",
        "Token.ls" => "selfhost/src/Syntax/Token.ls",
        "AST.ls" => "selfhost/src/Syntax/AST.ls",
        "Span.ls" => "selfhost/src/Syntax/Span.ls",
        "Lexer.ls" => "selfhost/src/Syntax/Lexer.ls",
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
        "TypeInferSmoke.ls" => "selfhost/src/Types/TypeInferSmoke.ls",
        "TypeInfer.ls" => "selfhost/src/Types/TypeInfer.ls",
        "Constraints.ls" => "selfhost/src/Types/Constraints.ls",
        "MetadataCheck.ls" => "selfhost/src/Types/MetadataCheck.ls",
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
        "Formatter.ls" => "selfhost/src/Tools/Text/Formatter.ls",
        "Linter.ls" => "selfhost/src/Tools/Text/Linter.ls",
        "JsonRpc.ls" => "selfhost/src/Tools/Lsp/JsonRpc.ls",
        "LspServer.ls" => "selfhost/src/Tools/Lsp/LspServer.ls",
        "DocTools.ls" => "selfhost/src/Tools/Doc/DocTools.ls",
        "HtmlDoc.ls" => "selfhost/src/Tools/Doc/HtmlDoc.ls",
        "HtmlLayout.ls" => "selfhost/src/Tools/Doc/HtmlLayout.ls",
        "HtmlTemplate.ls" => "selfhost/src/Tools/Doc/HtmlTemplate.ls",
        "TestRunner.ls" => "selfhost/src/Tools/Test/TestRunner.ls",
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

/// selfhost モジュールの埋め込みソースを返す
pub(crate) fn selfhost_module(name: &str) -> &'static str {
    match name {
        "Main.ls" => include_str!("../../../../selfhost/src/App/Main.ls"),
        "Cli.ls" => include_str!("../../../../selfhost/src/App/Cli.ls"),
        "ModuleResolver.ls" => include_str!("../../../../selfhost/src/App/ModuleResolver.ls"),
        "CompilerMode.ls" => include_str!("../../../../selfhost/src/App/CompilerMode.ls"),
        "PipelineSmoke.ls" => include_str!("../../../../selfhost/src/App/PipelineSmoke.ls"),
        "Token.ls" => include_str!("../../../../selfhost/src/Syntax/Token.ls"),
        "AST.ls" => include_str!("../../../../selfhost/src/Syntax/AST.ls"),
        "Span.ls" => include_str!("../../../../selfhost/src/Syntax/Span.ls"),
        "Lexer.ls" => include_str!("../../../../selfhost/src/Syntax/Lexer.ls"),
        "Parser.ls" => include_str!("../../../../selfhost/src/Syntax/Parser.ls"),
        "IR.ls" => include_str!("../../../../selfhost/src/IR/IR.ls"),
        "Type.ls" => include_str!("../../../../selfhost/src/Types/Type.ls"),
        "TypeScheme.ls" => include_str!("../../../../selfhost/src/Types/TypeScheme.ls"),
        "TypeInferCore.ls" => include_str!("../../../../selfhost/src/Types/TypeInferCore.ls"),
        "TypeInferFunctions.ls" => {
            include_str!("../../../../selfhost/src/Types/TypeInferFunctions.ls")
        },
        "TypeInferBuiltins.ls" => include_str!("../../../../selfhost/src/Types/TypeInferBuiltins.ls"),
        "TypeInferSmoke.ls" => include_str!("../../../../selfhost/src/Types/TypeInferSmoke.ls"),
        "TypeInfer.ls" => include_str!("../../../../selfhost/src/Types/TypeInfer.ls"),
        "Compiler.ls" => include_str!("../../../../selfhost/src/Backend/Wasm/Compiler.ls"),
        "WasmEmit.ls" => include_str!("../../../../selfhost/src/Backend/Wasm/WasmEmit.ls"),
        "Formatter.ls" => include_str!("../../../../selfhost/src/Tools/Text/Formatter.ls"),
        "TestRunner.ls" => include_str!("../../../../selfhost/src/Tools/Test/TestRunner.ls"),
        "DocTools.ls" => include_str!("../../../../selfhost/src/Tools/Doc/DocTools.ls"),
        "HtmlDoc.ls" => include_str!("../../../../selfhost/src/Tools/Doc/HtmlDoc.ls"),
        "HtmlLayout.ls" => include_str!("../../../../selfhost/src/Tools/Doc/HtmlLayout.ls"),
        "HtmlTemplate.ls" => include_str!("../../../../selfhost/src/Tools/Doc/HtmlTemplate.ls"),
        "JsonRpc.ls" => include_str!("../../../../selfhost/src/Tools/Lsp/JsonRpc.ls"),
        "Linter.ls" => include_str!("../../../../selfhost/src/Tools/Text/Linter.ls"),
        "LspServer.ls" => include_str!("../../../../selfhost/src/Tools/Lsp/LspServer.ls"),
        "NativeTarget.ls" => include_str!("../../../../selfhost/src/Backend/Native/NativeTarget.ls"),
        "NativeCodegen.ls" => include_str!("../../../../selfhost/src/Backend/Native/NativeCodegen.ls"),
        "NativeEmit.ls" => include_str!("../../../../selfhost/src/Backend/Native/NativeEmit.ls"),
        "Linker.ls" => include_str!("../../../../selfhost/src/Backend/Native/Linker.ls"),
        "MacroExpand.ls" => include_str!("../../../../selfhost/src/Syntax/MacroExpand.ls"),
        other => panic!("不明な selfhost モジュール: {other}"),
    }
}

fn selfhost_fixture_dir(prefix: &str) -> std::path::PathBuf {
    let id = SELFHOST_FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/e2e-selfhost-fixtures")
        .join(format!("{prefix}-{id}"))
}

fn write_selfhost_fixture_modules(dir: &std::path::Path, modules: &[&str]) -> Result<(), String> {
    for name in modules {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(&path, selfhost_module(name))
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
            "LspServer.ls",
        ))
    } else {
        let entry_source = format!("(module Main)\n(import LspServer)\n{harness}");
        Some(try_compile_and_run_selfhost_fixture_entry(
            "lsp-harness",
            SELFHOST_LSP_RUNTIME_MODULES,
            "Main.ls",
            &entry_source,
        ))
    }
}

fn cached_selfhost_bundle(cell: &'static OnceLock<String>, modules: &[&str]) -> &'static str {
    cell.get_or_init(|| {
        modules
            .iter()
            .map(|name| selfhost_module(name))
            .collect::<Vec<_>>()
            .join("\n")
    })
}

static SELFHOST_LEXER_RUNTIME_BUNDLE: OnceLock<String> = OnceLock::new();
static SELFHOST_PARSER_RUNTIME_BUNDLE: OnceLock<String> = OnceLock::new();
static SELFHOST_TYPEINFER_RUNTIME_BUNDLE: OnceLock<String> = OnceLock::new();
static SELFHOST_CLI_RUNTIME_BUNDLE: OnceLock<String> = OnceLock::new();
static SELFHOST_NATIVE_CODEGEN_BUNDLE: OnceLock<String> = OnceLock::new();

pub(crate) fn selfhost_lexer_runtime_bundle() -> &'static str {
    cached_selfhost_bundle(&SELFHOST_LEXER_RUNTIME_BUNDLE, &["Token.ls", "Lexer.ls"])
}

pub(crate) fn selfhost_parser_runtime_bundle() -> &'static str {
    cached_selfhost_bundle(
        &SELFHOST_PARSER_RUNTIME_BUNDLE,
        &["Token.ls", "AST.ls", "Lexer.ls", "Parser.ls"],
    )
}

pub(crate) fn selfhost_typeinfer_runtime_bundle() -> &'static str {
    cached_selfhost_bundle(
        &SELFHOST_TYPEINFER_RUNTIME_BUNDLE,
        &[
            "AST.ls",
            "Type.ls",
            "TypeScheme.ls",
            "TypeInferCore.ls",
            "TypeInferFunctions.ls",
            "TypeInferBuiltins.ls",
            "TypeInfer.ls",
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
            "Compiler.ls",
            "WasmEmit.ls",
            "Formatter.ls",
            "TestRunner.ls",
            "DocTools.ls",
            "JsonRpc.ls",
            "LspServer.ls",
            "Cli.ls",
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
            selfhost_source_path("Main.ls")
                .ends_with("selfhost/src/App/Main.ls"),
            "Main.ls は canonical entrypoint を指すべき"
        );
        assert!(
            selfhost_source_path("Cli.ls")
                .ends_with("selfhost/src/App/Cli.ls"),
            "Cli.ls は App/Cli.ls を指すべき"
        );
        assert!(
            selfhost_source_path("ModuleResolver.ls")
                .ends_with("selfhost/src/App/ModuleResolver.ls"),
            "ModuleResolver.ls は App/ModuleResolver.ls を指すべき"
        );
        assert!(
            selfhost_source_path("CompilerMode.ls")
                .ends_with("selfhost/src/App/CompilerMode.ls"),
            "CompilerMode.ls は App/CompilerMode.ls を指すべき"
        );
        assert!(
            selfhost_source_path("PipelineSmoke.ls")
                .ends_with("selfhost/src/App/PipelineSmoke.ls"),
            "PipelineSmoke.ls は App/PipelineSmoke.ls を指すべき"
        );
        assert!(
            selfhost_source_path("TypeInfer.ls")
                .ends_with("selfhost/src/Types/TypeInfer.ls"),
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
            selfhost_source_path("WasmEmit.ls")
                .ends_with("selfhost/src/Backend/Wasm/WasmEmit.ls"),
            "WasmEmit.ls は Backend/Wasm/WasmEmit.ls を指すべき"
        );
    }

    #[test]
    fn test_support_selfhost_module_reads_canonical_sources() {
        assert!(selfhost_module("Main.ls").contains("(module App.Main)"));
        assert!(selfhost_module("Cli.ls").contains("(module App.Cli)"));
        assert!(selfhost_module("ModuleResolver.ls").contains("(module App.ModuleResolver)"));
        assert!(selfhost_module("CompilerMode.ls").contains("(module App.CompilerMode)"));
        assert!(selfhost_module("PipelineSmoke.ls").contains("(module App.PipelineSmoke)"));
        assert!(selfhost_module("TypeInferFunctions.ls").contains("(module Types.TypeInferFunctions)"));
        assert!(selfhost_module("TypeInferBuiltins.ls").contains("(module Types.TypeInferBuiltins)"));
        assert!(selfhost_module("TypeInferSmoke.ls").contains("(module Types.TypeInferSmoke)"));
        assert!(selfhost_module("TypeInfer.ls").contains("(module Types.TypeInfer)"));
    }

    #[test]
    fn test_support_selfhost_cli_runtime_bundle_cached() {
        let first = selfhost_cli_runtime_bundle();
        let second = selfhost_cli_runtime_bundle();
        assert_eq!(first, second);
        assert_eq!(first.as_ptr(), second.as_ptr());
        assert!(first.contains(selfhost_module("Cli.ls").trim()));
    }

    #[test]
    fn test_support_selfhost_typeinfer_runtime_bundle_cached() {
        let bundle = selfhost_typeinfer_runtime_bundle();
        assert!(bundle.contains(selfhost_module("TypeInferFunctions.ls").trim()));
        assert!(bundle.contains(selfhost_module("TypeInferBuiltins.ls").trim()));
        assert!(bundle.contains(selfhost_module("TypeInfer.ls").trim()));
        assert_eq!(
            bundle.as_ptr(),
            selfhost_typeinfer_runtime_bundle().as_ptr()
        );
    }
}
