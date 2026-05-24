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
            && source.contains("(print 9000000065)")
            && source.contains("(append-span-token tokens0 manual-kind2 manual-ws manual-end)")
            && source.contains("(print (vector-length manual-next))")
            && source.contains("(print 9000000066)")
            && source.contains(
                "(make-tokenize-state-from-appended-tokens 0 manual-end manual-next)"
            )
            && source.contains("(let [manual-step-tokens (vector-get manual-step-state 2)]")
            && source.contains("(print 9000000067)")
            && source.contains("(append-span-token-state tokens0 0 1 0 0 1)")
            && source.contains("(let [manual-helper-tokens (vector-get manual-helper-state 2)]")
            && source.contains("(print 9000000068)")
            && source.contains("(append-lex-result-state tokens0 (lex-one src 0 src-len) 0)")
            && source.contains("(let [manual-lex-tokens (vector-get manual-lex-state 2)]")
            && source.contains("(print 9000000060)")
            && source.contains("(tokenize-spans-step src 0 src-len tokens0)")
            && source.contains("(print 9000000061)")
            && source.contains("(tokenize-spans-step-512 src 0 src-len tokens0)"),
        "tokenizer probe は vector-push、append-span-token、make-tokenize-state、step 本体を分けて観測できるべき"
    );
}

#[test]
fn lexer_roots_appended_tokens_before_state_storage() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Lexer.ls"))
        .expect("Lexer.ls を読めること");

    let helper = source
        .split("(defn make-tokenize-state-from-appended-tokens [done next-pos next-tokens]")
        .nth(1)
        .and_then(|tail| tail.split("(defn append-span-token-state").next())
        .expect("Lexer.ls は append 後 tokens を caller root して state 化する helper を持つこと");
    let append_state = source
        .split("(defn append-span-token-state [tokens done next-pos kind start end]")
        .nth(1)
        .and_then(|tail| tail.split("(defn append-span-token-state-end").next())
        .expect("Lexer.ls に append-span-token-state が存在すること");
    let append_state_end = source
        .split("(defn append-span-token-state-end [tokens done kind start end]")
        .nth(1)
        .and_then(|tail| tail.split("(defn append-lex-result-state").next())
        .expect("Lexer.ls に append-span-token-state-end が存在すること");
    let append_lex = source
        .split("(defn append-lex-result-state [tokens result start]")
        .nth(1)
        .and_then(|tail| tail.split("(defn append-lex-result-state-rst").next())
        .expect("Lexer.ls に append-lex-result-state が存在すること");

    assert!(
        helper.contains("(root_push next-tokens)")
            && helper.contains("(make-tokenize-state done next-pos next-tokens)")
            && helper.contains("(root_pop)")
            && append_state.contains(
                "(make-tokenize-state-from-appended-tokens done next-pos next-tokens)"
            )
            && append_state_end
                .contains("(make-tokenize-state-from-appended-tokens done end next-tokens)")
            && append_lex.contains("(make-tokenize-state-from-appended-tokens 1 start next-tokens)")
            && append_lex.contains(
                "(make-tokenize-state-from-appended-tokens 0 end-pos next-tokens)"
            ),
        "append-span-token の戻り値は x86 stage2 native で state 格納前に caller 側 root が必要"
    );
}

#[test]
fn lexer_tokenize_spans_step_roots_params_across_skip_and_lex() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Lexer.ls"))
        .expect("Lexer.ls を読めること");
    let step = source
        .split("(defn tokenize-spans-step [src pos len tokens]")
        .nth(1)
        .and_then(|tail| tail.split("(defn tokenize-spans-step-2").next())
        .expect("Lexer.ls に tokenize-spans-step が存在すること");

    assert!(
        step.contains("(root_push src)")
            && step.contains("(root_push tokens)")
            && step.contains("(let [ws-pos (skip-ws-loop src pos len)]")
            && step.contains("(let [state")
            && step.contains("(root_push state)")
            && step.contains("(root_pop)")
            && step.contains("(root_pop)")
            && step.contains("(root_pop)")
            && step.contains("state"),
        "tokenize-spans-step は skip/lex/append の間に src/tokens/state を自前で root するべき"
    );
}

#[test]
fn lexer_tokenize_spans_step_delegates_append_state_to_small_helpers() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Lexer.ls"))
        .expect("Lexer.ls を読めること");
    let step = source
        .split("(defn tokenize-spans-step [src pos len tokens]")
        .nth(1)
        .and_then(|tail| tail.split("(defn tokenize-spans-step-2").next())
        .expect("Lexer.ls に tokenize-spans-step が存在すること");

    assert!(
        step.contains("(append-span-token-state tokens 1 ws-pos 99 ws-pos ws-pos)")
            && step.contains("(append-span-token-state tokens 0 end-pos kind ws-pos end-pos)")
            && !step.contains("(let [next-tokens (append-span-token tokens kind ws-pos end-pos)]"),
        "tokenize-spans-step は stage2 x86 の local 保持崩れを避けるため append/state 化を小さい helper に委譲するべき"
    );
}

#[test]
fn compiler_mode_compile_pair_probe_prints_first_pair_debug_compile() {
    let source =
        std::fs::read_to_string(workspace_root().join("selfhost/src/App/CompilerMode.ls"))
            .expect("CompilerMode.ls を読めること");
    let probe = source
        .split("(defn compile-file-mode-cache-compile-pair-progress-probe []")
        .nth(1)
        .and_then(|tail| tail.split("(defn compile-file-mode-ast-chunked-step-progress-probe").next())
        .expect("CompilerMode.ls に compile pair progress probe が存在すること");

    assert!(
        probe.contains("(print 155)")
            && probe.contains("(decl-tag-or-minus-one decls0 0)")
            && probe.contains("(decl-tag-or-minus-one decls0 3)")
            && probe.contains(
                "(compile-defn-functions-chunked-step-progress-debug decls0 0 (vector-length decls0) src0 ftable debug-data-ref (vector-new 8))"
            )
            && probe.contains("(print 156)")
            && probe.contains("(print (vector-length debug-functions))")
            && probe.contains("(print (vector-length (ref-get debug-data-ref)))"),
        "compile pair probe は first pair の AST tag と debug compile の関数数を出すべき"
    );
}
