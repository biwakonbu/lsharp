#![allow(dead_code)]

use lsharp_ir::lower::Lower;
use lsharp_types::infer::Infer;

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

pub(crate) fn selfhost_source_path(name: &str) -> std::path::PathBuf {
    selfhost_project_root().join(match name {
        "Main.ls" => "selfhost/src/App/Main.ls",
        "Cli.ls" => "selfhost/src/App/Cli.ls",
        "Token.ls" => "selfhost/src/Syntax/Token.ls",
        "AST.ls" => "selfhost/src/Syntax/AST.ls",
        "Lexer.ls" => "selfhost/src/Syntax/Lexer.ls",
        "Parser.ls" => "selfhost/src/Syntax/Parser.ls",
        "Derive.ls" => "selfhost/src/Syntax/Derive.ls",
        "Hygiene.ls" => "selfhost/src/Syntax/Hygiene.ls",
        "IR.ls" => "selfhost/src/IR/IR.ls",
        "Type.ls" => "selfhost/src/Types/Type.ls",
        "TypeScheme.ls" => "selfhost/src/Types/TypeScheme.ls",
        "TypeInferCore.ls" => "selfhost/src/Types/TypeInferCore.ls",
        "TypeInfer.ls" => "selfhost/src/Types/TypeInfer.ls",
        "Constraints.ls" => "selfhost/src/Types/Constraints.ls",
        "Compiler.ls" => "selfhost/src/Backend/Wasm/Compiler.ls",
        "WasmEmit.ls" => "selfhost/src/Backend/Wasm/WasmEmit.ls",
        "Formatter.ls" => "selfhost/src/Tools/Text/Formatter.ls",
        "TestRunner.ls" => "selfhost/src/Tools/Test/TestRunner.ls",
        "DocTools.ls" => "selfhost/src/Tools/Doc/DocTools.ls",
        "HtmlDoc.ls" => "selfhost/src/Tools/Doc/HtmlDoc.ls",
        "HtmlLayout.ls" => "selfhost/src/Tools/Doc/HtmlLayout.ls",
        "HtmlTemplate.ls" => "selfhost/src/Tools/Doc/HtmlTemplate.ls",
        "JsonRpc.ls" => "selfhost/src/Tools/Lsp/JsonRpc.ls",
        "Linter.ls" => "selfhost/src/Tools/Text/Linter.ls",
        "MacroExpand.ls" => "selfhost/src/Syntax/MacroExpand.ls",
        other => panic!("不明な selfhost canonical module path: {other}"),
    })
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

pub(crate) fn parser_macroexpand_runtime_modules() -> (String, String, String, String, String) {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let macroexpand_ls = std::fs::read_to_string(selfhost_source_path("MacroExpand.ls"))
        .expect("canonical MacroExpand.ls が読み込めない");
    (token_ls, ast_ls, lexer_ls, parser_ls, macroexpand_ls)
}

/// selfhost/src/App/Cli.ls を直接実行するための最小 runtime bundle
pub(crate) fn selfhost_cli_runtime_bundle() -> String {
    let token_ls = std::fs::read_to_string(selfhost_source_path("Token.ls"))
        .expect("canonical Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(selfhost_source_path("AST.ls"))
        .expect("canonical AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(selfhost_source_path("Lexer.ls"))
        .expect("canonical Lexer.ls が読み込めない");
    let parser_ls = std::fs::read_to_string(selfhost_source_path("Parser.ls"))
        .expect("canonical Parser.ls が読み込めない");
    let ir_ls = std::fs::read_to_string(selfhost_source_path("IR.ls"))
        .expect("canonical IR.ls が読み込めない");
    let type_ls = std::fs::read_to_string(selfhost_source_path("Type.ls"))
        .expect("canonical Type.ls が読み込めない");
    let type_scheme_ls = std::fs::read_to_string(selfhost_source_path("TypeScheme.ls"))
        .expect("canonical TypeScheme.ls が読み込めない");
    let type_infer_core_ls = std::fs::read_to_string(selfhost_source_path("TypeInferCore.ls"))
        .expect("canonical TypeInferCore.ls が読み込めない");
    let type_infer_ls = std::fs::read_to_string(selfhost_source_path("TypeInfer.ls"))
        .expect("canonical TypeInfer.ls が読み込めない");
    let compiler_ls = std::fs::read_to_string(selfhost_source_path("Compiler.ls"))
        .expect("canonical Compiler.ls が読み込めない");
    let wasm_emit_ls = std::fs::read_to_string(selfhost_source_path("WasmEmit.ls"))
        .expect("canonical WasmEmit.ls が読み込めない");
    let formatter_ls = std::fs::read_to_string(selfhost_source_path("Formatter.ls"))
        .expect("canonical Formatter.ls が読み込めない");
    let test_runner_ls = std::fs::read_to_string(selfhost_source_path("TestRunner.ls"))
        .expect("canonical TestRunner.ls が読み込めない");
    let doc_tools_ls = std::fs::read_to_string(selfhost_source_path("DocTools.ls"))
        .expect("canonical DocTools.ls が読み込めない");
    let cli_ls = std::fs::read_to_string(selfhost_source_path("Cli.ls"))
        .expect("canonical Cli.ls が読み込めない");

    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        token_ls,
        ast_ls,
        lexer_ls,
        parser_ls,
        ir_ls,
        type_ls,
        type_scheme_ls,
        type_infer_core_ls,
        type_infer_ls,
        compiler_ls,
        wasm_emit_ls,
        formatter_ls,
        test_runner_ls,
        doc_tools_ls,
        cli_ls
    )
}

/// エントリ `.ls` ファイルから依存を解決してコンパイルし、WASI 実行結果を返す
pub(crate) fn compile_and_run_file(path: &std::path::Path) -> String {
    try_compile_and_run_file(path).unwrap()
}
