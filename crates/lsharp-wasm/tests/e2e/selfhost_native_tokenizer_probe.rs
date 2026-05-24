fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("lsharp-wasm crate is nested under crates/")
        .to_path_buf()
}

#[test]
fn compiler_mode_tokenize_step_probe_distinguishes_append_state_boundaries() {
    let source =
        std::fs::read_to_string(workspace_root().join("selfhost/src/App/CompilerMode.ls"))
            .expect("CompilerMode.ls を読めること");

    assert!(
        source.contains("(defn print-tokenize-step-progress-probe [src]")
            && source.contains("(print 9000000062)")
            && source.contains("(let [manual-kind (vector-push manual-base 0)]")
            && source.contains("(let [manual-start (vector-push manual-kind 0)]")
            && source.contains("(let [manual-token (vector-push manual-start 1)]")
            && source.contains("(print (vector-length manual-kind))")
            && source.contains("(print (vector-length manual-start))")
            && source.contains("(print (vector-length manual-token))")
            && source.contains("(print 9000000063)")
            && source.contains("(append-span-token manual-append-base 0 0 1)")
            && source.contains("(print (vector-length manual-appended))")
            && source.contains("(print 9000000064)")
            && source.contains("(make-tokenize-state 0 1 manual-appended)")
            && source.contains("(let [manual-state-tokens (vector-get manual-state 2)]")
            && source.contains("(print (vector-length manual-state-tokens))")
            && source.contains("(print 9000000060)")
            && source.contains("(tokenize-spans-step src 0 src-len tokens0)")
            && source.contains("(print 9000000061)")
            && source.contains("(tokenize-spans-step-512 src 0 src-len tokens0)"),
        "tokenizer probe は vector-push、append-span-token、make-tokenize-state、step 本体を分けて観測できるべき"
    );
}
