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
