#[test]
fn test_emit_wasm_wasi_p2_basic_program_compiles() {
    let component = compile_wasi_p2("(defn main [] (print 42))");
    assert!(component.len() > 8);
    assert_eq!(&component[0..4], b"\0asm");

    let engine = wasmtime::Engine::default();
    wasmtime::component::Component::new(&engine, &component)
        .expect("P2 entrypoint は valid component を生成するべき");
}

#[test]
fn test_emit_wasm_wasi_p2_runs_print_via_component_runner() {
    let component = compile_wasi_p2("(defn main [] (print 42))");

    let output = crate::wasi_runner::run_wasm_component(&component)
        .expect("P2 component は preview2 runner で実行できるべき");
    assert_eq!(output, "42\n");
}

#[test]
fn test_emit_wasm_wasi_p2_supports_stdin_and_args() {
    let component = compile_wasi_p2(
        r#"
        (defn main []
          (do
            (print-string (command-line-arg 0))
            (print-string ":")
            (print-string (read-stdin))
            0))
        "#,
    );

    let output = crate::wasi_runner::run_wasm_component_with_args_and_stdin(
        &component,
        &["alpha"],
        "stdin-smoke",
    )
    .expect("P2 component は argv/stdin bridge を使えるべき");
    assert_eq!(output, "alpha:stdin-smoke");
}

#[test]
fn test_emit_wasm_wasi_p2_supports_large_stdout_write() {
    let payload = "x".repeat(4097);
    let component = compile_wasi_p2(&format!(
        r#"
        (defn main []
          (do
            (print-string "{payload}")
            0))
        "#
    ));

    let output = crate::wasi_runner::run_wasm_component(&component)
        .expect("P2 component は 4KiB 超の stdout write を処理できるべき");
    assert_eq!(output, payload);
}

#[test]
fn test_emit_wasm_wasi_p2_supports_file_roundtrip() {
    let component = compile_wasi_p2(
        r#"
        (defn main []
          (do
            (write-file "roundtrip.txt" "hello component")
            (print-string (read-file "roundtrip.txt"))
            0))
        "#,
    );

    let dir = std::env::temp_dir().join(format!(
        "lsharp_wasi_p2_file_roundtrip_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let output = crate::wasi_runner::run_wasm_component_with_dir_args_and_stdin(
        &component,
        Some(&dir),
        &[],
        "",
    )
    .expect("P2 component は preview2 filesystem bridge 経由で file roundtrip できるべき");
    assert_eq!(output, "hello component");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_emit_wasm_http_handler_p2_remains_component_compatible() {
    let component = compile_wasi_p2(r#"(defn handle [request] "ok")"#);

    let engine = wasmtime::Engine::default();
    wasmtime::component::Component::new(&engine, &component)
        .expect("HTTP handler P2 entrypoint は valid component を生成するべき");
}
