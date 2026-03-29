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

/// selfhost/src/App/Main.ls のパス (import 解決にはマルチファイルコンパイルが必要)
pub(crate) fn selfhost_main_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../selfhost/src/App/Main.ls")
}

/// selfhost/Cli.ls を直接実行するための最小 runtime bundle
pub(crate) fn selfhost_cli_runtime_bundle() -> String {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(project_root.join("selfhost/Lexer.ls"))
        .expect("selfhost/Lexer.ls が読み込めない");
    let parser_ls = std::fs::read_to_string(project_root.join("selfhost/Parser.ls"))
        .expect("selfhost/Parser.ls が読み込めない");
    let ir_ls = std::fs::read_to_string(project_root.join("selfhost/IR.ls"))
        .expect("selfhost/IR.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls = std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
        .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_defn_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferDefn.ls"))
            .expect("selfhost/TypeInferDefn.ls が読み込めない");
    let type_infer_ls = std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
        .expect("selfhost/TypeInfer.ls が読み込めない");
    let compiler_ls = std::fs::read_to_string(project_root.join("selfhost/Compiler.ls"))
        .expect("selfhost/Compiler.ls が読み込めない");
    let wasm_emit_ls = std::fs::read_to_string(project_root.join("selfhost/WasmEmit.ls"))
        .expect("selfhost/WasmEmit.ls が読み込めない");
    let formatter_ls = std::fs::read_to_string(project_root.join("selfhost/Formatter.ls"))
        .expect("selfhost/Formatter.ls が読み込めない");
    let test_runner_ls = std::fs::read_to_string(project_root.join("selfhost/TestRunner.ls"))
        .expect("selfhost/TestRunner.ls が読み込めない");
    let doc_tools_ls = std::fs::read_to_string(project_root.join("selfhost/DocTools.ls"))
        .expect("selfhost/DocTools.ls が読み込めない");
    let cli_ls = std::fs::read_to_string(project_root.join("selfhost/Cli.ls"))
        .expect("selfhost/Cli.ls が読み込めない");

    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        token_ls,
        ast_ls,
        lexer_ls,
        parser_ls,
        ir_ls,
        type_ls,
        type_scheme_ls,
        type_infer_core_ls,
        type_infer_defn_ls,
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
