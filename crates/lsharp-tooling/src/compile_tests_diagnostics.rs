fn temp_project(name: &str, source: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "lsharp_compile_diagnostic_{name}_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".git")).unwrap();
    let file = root.join("Main.ls");
    std::fs::write(&file, source).unwrap();
    (root, file)
}

#[test]
fn compile_prepare_source_preserves_lexer_code_and_span() {
    let (root, file) = temp_project("lexer", "@\n");

    let error = prepare_source_for_compile(&file)
        .expect_err("lexer error は compile 境界へ到達するべき")
        .to_string();

    assert!(
        error.contains("[LS0001]"),
        "diagnostic code missing: {error}"
    );
    assert!(error.contains("(0..1)"), "diagnostic span missing: {error}");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn compile_prepare_source_preserves_parser_eof_code_and_span() {
    let (root, file) = temp_project("parser-eof", "(defn main []");

    let error = prepare_source_for_compile(&file)
        .expect_err("parser EOF は compile 境界へ到達するべき")
        .to_string();

    assert!(
        error.contains("[LS0102]"),
        "diagnostic code missing: {error}"
    );
    assert!(
        error.contains("(0..13)"),
        "diagnostic span missing: {error}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn compile_missing_module_preserves_code_and_import_span() {
    let (root, file) = temp_project(
        "module-not-found",
        "(import MissingModule)\n(defn main [] (print 0))\n",
    );
    let output = root.join("Main.wasm");

    let error = compile_file(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WasiPreview1),
    )
    .expect_err("missing module は compile を拒否するべき")
    .to_string();

    assert!(
        error.contains("[LS3102]"),
        "diagnostic code missing: {error}"
    );
    assert!(error.contains("(0..22)"), "import span missing: {error}");
    let _ = std::fs::remove_dir_all(root);
}

/// 単一 file 経路 (`import` 無し)。
/// sibling 参照が無いので、拒否しなければ compile 成功のまま無出力バイナリになる。
#[test]
fn compile_rejects_block_form_module_body_in_single_file_path() {
    let (root, file) = temp_project("module-body-single", "(module Main (defn main [] (print 42)))\n");
    let output = root.join("Main.wasm");

    let error = compile_file(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WasiPreview1),
    )
    .expect_err("block 形式 module body は compile を拒否するべき")
    .to_string();

    assert!(
        error.contains("[LS0105]"),
        "diagnostic code missing: {error}"
    );
    assert!(error.contains("未対応の構文"), "wording missing: {error}");
    assert!(error.contains("(0.."), "span missing: {error}");
    assert!(
        !output.exists(),
        "拒否したのに artifact を書いてはいけない: {}",
        output.display()
    );
    let _ = std::fs::remove_dir_all(root);
}

/// module graph 経路。block 形式は import 先の module 側に置く。
#[test]
fn compile_rejects_block_form_module_body_in_module_graph_path() {
    let root = std::env::temp_dir().join(format!(
        "lsharp_compile_diagnostic_module_body_graph_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".git")).unwrap();
    std::fs::write(root.join("Lib.ls"), "(module Lib (defn helper [] 7))\n").unwrap();
    let main_path = root.join("Main.ls");
    std::fs::write(
        &main_path,
        "(module Main)\n(import Lib)\n(defn main [] (print 0))\n",
    )
    .unwrap();
    let output = root.join("Main.wasm");

    let error = compile_file(
        &main_path,
        Some(&output),
        false,
        Some(CompileTarget::WasiPreview1),
    )
    .expect_err("import 先の block 形式 module body も compile を拒否するべき")
    .to_string();

    assert!(
        error.contains("[LS0105]"),
        "diagnostic code missing: {error}"
    );
    assert!(error.contains("Lib.ls"), "対象 file が分からない: {error}");
    assert!(
        !output.exists(),
        "拒否したのに artifact を書いてはいけない: {}",
        output.display()
    );
    let _ = std::fs::remove_dir_all(root);
}
