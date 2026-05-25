fn workspace_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("lsharp-wasm crate is nested under crates/")
        .to_path_buf()
}

#[test]
fn compiler_mode_tokenize_step_probe_distinguishes_append_state_boundaries() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/App/CompilerMode.ls"))
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
            && source
                .contains("(make-tokenize-state-from-appended-tokens 0 manual-end manual-next)")
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
            && append_state
                .contains("(make-tokenize-state-from-appended-tokens done next-pos next-tokens)")
            && append_state_end
                .contains("(make-tokenize-state-from-appended-tokens done end next-tokens)")
            && append_lex
                .contains("(make-tokenize-state-from-appended-tokens 1 start next-tokens)")
            && append_lex
                .contains("(make-tokenize-state-from-appended-tokens 0 end-pos next-tokens)"),
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
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/App/CompilerMode.ls"))
        .expect("CompilerMode.ls を読めること");
    let probe = source
        .split("(defn compile-file-mode-cache-compile-pair-progress-probe []")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn compile-file-mode-ast-chunked-step-progress-probe")
                .next()
        })
        .expect("CompilerMode.ls に compile pair progress probe が存在すること");

    assert!(
        probe.contains("(print 155)")
            && probe.contains("(decl-tag-or-minus-one decls0 0)")
            && probe.contains("(decl-tag-or-minus-one decls0 3)")
            && probe.contains("(let [reparsed (parse-program src0)")
            && probe.contains("first-defn-span (find-span-kind-index spans0 0 span-count 30)")
            && probe.contains("(print 157)")
            && probe.contains("(decl-tag-or-minus-one reparsed 1)")
            && probe.contains("(print 158)")
            && probe.contains("(span-kind-or-minus-one spans0 (- first-defn-span 1))")
            && probe.contains("(span-kind-or-minus-one spans0 (+ first-defn-span 7))")
            && probe.contains("(let [direct-pos (ref-new (- first-defn-span 1))]")
            && probe.contains("(let [direct-node (parse-expr-v3 spans0 direct-pos src0)]")
            && probe.contains("(print (vector-get direct-node 8))")
            && probe.contains("(print 159)")
            && probe.contains("(let [direct-defn-pos (ref-new first-defn-span)]")
            && probe.contains("(let [direct-defn (parse-defn-v3 spans0 direct-defn-pos src0)]")
            && source.contains("(defn print-direct-defn-build-progress-probe [spans first-defn-span src]")
            && source.contains("(print 184)")
            && source.contains("(print 189)")
            && probe.contains("(print-direct-defn-build-progress-probe spans0 first-defn-span src0)")
            && source.contains(
                "(defn print-direct-defn-return-cleanup-progress-probe [spans first-defn-span src]"
            )
            && source.contains("(print 190)")
            && source.contains("(print 191)")
            && probe
                .contains("(print-direct-defn-return-cleanup-progress-probe spans0 first-defn-span src0)")
            && probe.contains(
                "(compile-defn-functions-chunked-step-progress-debug decls0 0 (vector-length decls0) src0 ftable debug-data-ref (vector-new 8))"
            )
            && probe.contains("(print 156)")
            && probe.contains("(print (vector-length debug-functions))")
            && probe.contains("(print (vector-length (ref-get debug-data-ref)))"),
        "compile pair probe は first pair の AST tag と debug compile の関数数を出すべき"
    );
}

#[test]
fn compiler_mode_register_all_pairs_roots_final_state_before_result_alloc() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/App/CompilerMode.ls"))
        .expect("CompilerMode.ls を読めること");
    let body = source
        .split("(defn register-all-pairs [pairs idx n ftable func-idx]")
        .nth(1)
        .and_then(|tail| tail.split("(defn compile-src-decl-pairs-step").next())
        .expect("CompilerMode.ls に register-all-pairs が存在すること");

    assert!(
        body.contains("(root_push state)")
            && body.contains("next-ftable (vector-get state 2)")
            && body.contains("next-func-idx (vector-get state 3)")
            && body.contains("(push-object-vector (vector-new 2) next-ftable)")
            && body.contains("(vector-push with-ftable next-func-idx)")
            && body.contains("(root_pop)"),
        "register-all-pairs は stage2 native の final state local を result vector allocation 前に root して func index を保持するべき"
    );
}

#[test]
fn compiler_mode_source_pair_chunked_roots_function_accumulator_states() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/App/CompilerMode.ls"))
        .expect("CompilerMode.ls を読めること");
    let continue_step = source
        .split("(defn continue-compile-src-decl-pairs-step ")
        .nth(1)
        .and_then(|tail| tail.split("(defn compile-src-decl-pairs-step-8").next())
        .expect("continue-compile-src-decl-pairs-step が存在すること");
    let step8 = source
        .split("(defn compile-src-decl-pairs-step-8")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn continue-compile-src-decl-pairs-step-8")
                .next()
        })
        .expect("compile-src-decl-pairs-step-8 が存在すること");
    let continue8 = source
        .split("(defn continue-compile-src-decl-pairs-step-8")
        .nth(1)
        .and_then(|tail| tail.split("(defn compile-src-decl-pairs-step-64").next())
        .expect("continue-compile-src-decl-pairs-step-8 が存在すること");
    let step64 = source
        .split("(defn compile-src-decl-pairs-step-64")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn continue-compile-src-decl-pairs-step-64")
                .next()
        })
        .expect("compile-src-decl-pairs-step-64 が存在すること");
    let continue64 = source
        .split("(defn continue-compile-src-decl-pairs-step-64")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn compile-all-src-decl-pairs-chunked")
                .next()
        })
        .expect("continue-compile-src-decl-pairs-step-64 が存在すること");
    let chunked = source
        .split("(defn compile-all-src-decl-pairs-chunked")
        .nth(1)
        .and_then(|tail| tail.split("(defn compile-all-src-decl-pairs ").next())
        .expect("compile-all-src-decl-pairs-chunked が存在すること");

    for (name, body) in [
        ("continue step", continue_step),
        ("continue step-8", continue8),
        ("continue step-64", continue64),
    ] {
        assert!(
            body.contains("next-functions (vector-get state 2)")
                && body.contains("(root_push next-functions)")
                && !body.contains("data-ref (vector-get state 2)"),
            "{name} は state から取り出した functions accumulator を次 helper 呼び出し前に root するべき"
        );
    }

    for (name, body) in [("step-8", step8), ("step-64", step64)] {
        assert!(
            body.contains("(root_push functions)")
                && body.find("(root_push functions)")
                    < body.find("state (compile-src-decl-pairs-step"),
            "{name} wrapper は初回 pair step 呼び出し前に functions accumulator を root するべき"
        );
    }

    assert!(
        chunked.contains("state0 (compile-src-decl-pairs-step-64")
            && chunked.contains("(root_push state0)")
            && chunked.contains("state1 (continue-compile-src-decl-pairs-step-64")
            && chunked.contains("(root_push state1)")
            && chunked.contains("result (vector-get state1 2)")
            && !chunked.contains("(vector-get\n    (continue-compile-src-decl-pairs-step-64"),
        "compile-all-src-decl-pairs-chunked は stage2 x86 native の state local 崩れを避けるため state を root してから result を取り出すべき"
    );
}

#[test]
fn compiler_with_source_compile_step_roots_next_functions_before_state_alloc() {
    let source =
        std::fs::read_to_string(workspace_root().join("selfhost/src/Backend/Wasm/Compiler.ls"))
            .expect("Compiler.ls を読めること");
    let body = source
        .split("(defn compile-defn-functions-step-with-source-body-impl-3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn compile-let-with-ftable-impl-body-impl-3")
                .next()
        })
        .expect(
            "Compiler.ls に compile-defn-functions-step-with-source-body-impl-3 が存在すること",
        );

    assert!(
        body.contains("functions-slot (root_push functions)")
            && body.contains("next-functions (push-object-vector functions compiled-fn)")
            && body.contains("(root_set functions-slot next-functions)")
            && body.contains("(make-compile-step-state 0 (+ idx 1) next-functions)")
            && !body.contains("compile-defn-functions-step-finish functions compiled-fn idx"),
        "compile-defn-functions-step-with-source は stage2 x86 native の local 保持崩れを避けるため next-functions を state allocation 前に root slot へ戻すべき"
    );
}

#[test]
fn compiler_with_source_skip_decl_roots_functions_until_state_alloc() {
    let source =
        std::fs::read_to_string(workspace_root().join("selfhost/src/Backend/Wasm/Compiler.ls"))
            .expect("Compiler.ls を読めること");
    let body = source
        .split("(defn compile-defn-functions-step-with-source-body-impl-3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn compile-let-with-ftable-impl-body-impl-3")
                .next()
        })
        .expect(
            "Compiler.ls に compile-defn-functions-step-with-source-body-impl-3 が存在すること",
        );

    assert!(
        body.contains(
            "(let [result (make-compile-step-state 0 (+ idx 1) functions)]\n            (do\n              (root_pop)"
        )
            && body.contains("              result)))))))")
            && !body.contains(
                "(root_pop)\n          (root_pop)\n          (root_pop)\n          (root_pop)\n          (root_pop)\n          (make-compile-step-state 0 (+ idx 1) functions)"
            ),
        "compile-defn-functions-step-with-source の non-defn skip branch は functions を root したまま state allocation するべき"
    );
}

#[test]
fn compiler_source_chunked_roots_step_states_before_result_extract() {
    let source =
        std::fs::read_to_string(workspace_root().join("selfhost/src/Backend/Wasm/Compiler.ls"))
            .expect("Compiler.ls を読めること");
    let body = source
        .split("(defn compile-source-defn-functions-chunked")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn continue-compile-let-chain-step-with-source")
                .next()
        })
        .expect("Compiler.ls に compile-source-defn-functions-chunked が存在すること");

    assert!(
        body.contains("state0 (compile-defn-functions-step-64-with-source")
            && body.contains("(root_push state0)")
            && body.contains("state1 (continue-compile-defn-functions-step-64-with-source")
            && body.contains("(root_push state1)")
            && body.contains("result (vector-get state1 2)")
            && !body.contains("(vector-get (continue-compile-defn-functions-step-64-with-source"),
        "compile-source-defn-functions-chunked は stage2 x86 native の state local 崩れを避けるため step/continue state を root してから result を取り出すべき"
    );
}

#[test]
fn compiler_source_chunked_roots_next_functions_between_step_helpers() {
    let source =
        std::fs::read_to_string(workspace_root().join("selfhost/src/Backend/Wasm/Compiler.ls"))
            .expect("Compiler.ls を読めること");
    let continue_step = source
        .split("(defn continue-compile-defn-functions-step-with-source")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn continue-compile-defn-functions-step-times-with-source")
                .next()
        })
        .expect("continue-compile-defn-functions-step-with-source が存在すること");
    let step8 = source
        .split("(defn compile-defn-functions-step-8-with-source")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn continue-compile-defn-functions-step-8-with-source")
                .next()
        })
        .expect("compile-defn-functions-step-8-with-source が存在すること");
    let continue8 = source
        .split("(defn continue-compile-defn-functions-step-8-with-source")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn compile-defn-functions-step-64-with-source")
                .next()
        })
        .expect("continue-compile-defn-functions-step-8-with-source が存在すること");
    let step64 = source
        .split("(defn compile-defn-functions-step-64-with-source")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn continue-compile-defn-functions-step-64-with-source")
                .next()
        })
        .expect("compile-defn-functions-step-64-with-source が存在すること");
    let continue64 = source
        .split("(defn continue-compile-defn-functions-step-64-with-source")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn compile-source-defn-functions-chunked")
                .next()
        })
        .expect("continue-compile-defn-functions-step-64-with-source が存在すること");

    for (name, body) in [
        ("continue step", continue_step),
        ("continue step-8", continue8),
        ("continue step-64", continue64),
    ] {
        assert!(
            body.contains("next-functions (vector-get state 2)")
                && body.contains("(root_push next-functions)")
                && !body.contains("data-ref (vector-get state 2)"),
            "{name} は state から取り出した functions accumulator を次 helper 呼び出し前に root するべき"
        );
    }

    for (name, body) in [("step-8", step8), ("step-64", step64)] {
        assert!(
            body.contains("(root_push functions)")
                && body.find("(root_push functions)")
                    < body.find("state (compile-defn-functions-step-with-source"),
            "{name} wrapper は初回 step 呼び出し前に functions accumulator を root するべき"
        );
    }
}

#[test]
fn parser_program_step_delegates_expr_append_to_rooted_helper() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Parser.ls"))
        .expect("Parser.ls を読めること");
    let step = source
        .split("(defn parse-program-step-v3 [spans pos-ref src result]")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn parse-program-step-64-loop-bounded")
                .next()
        })
        .expect("Parser.ls に parse-program-step-v3 が存在すること");

    assert!(
        step.contains("(let [next-result (vector-push-single-rooted-v3 result expr)")
            && step.contains("(root_set result-slot next-result)")
            && !step.contains("(let [next-result (vector-push result expr)"),
        "parse-program-step-v3 は top-level AST node append を小さい rooted helper に委譲するべき"
    );
}

#[test]
fn parser_defn_body_finalize_uses_small_rooted_helper() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Parser.ls"))
        .expect("Parser.ls を読めること");
    let body = source
        .split("(defn parse-defn-bodyless-or-body-v3 [spans pos-ref src defn-node param-count]")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn parse-defn-bodyless-or-body-with-meta-v3")
                .next()
        })
        .expect("Parser.ls に parse-defn-bodyless-or-body-v3 が存在すること");

    assert!(
        body.contains("(finalize-defn-body-v3 defn-node param-count body)")
            && body.contains(
                "(finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count body)"
            )
            && !body.contains("node-with-placeholder"),
        "parse-defn body finalize は x86 stage2 の local 保持崩れを避けるため小さい rooted helper に委譲するべき"
    );
}

#[test]
fn parser_parse_defn_uses_direct_tail_sequence_without_ref_roundtrip() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Parser.ls"))
        .expect("Parser.ls を読めること");
    let parse_defn = source
        .split("(defn parse-defn-v3 [spans pos-ref src]")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defmacro-v3").next())
        .expect("Parser.ls に parse-defn-v3 が存在すること");

    assert!(
        parse_defn.contains("(skip-optional-type-sig-v3 spans pos-ref src)")
            && parse_defn.contains("(skip-optional-where-v3 spans pos-ref src)")
            && parse_defn.contains("(parse-defn-bodyless-or-body-with-meta-v3")
            && parse_defn.contains("(let [body (parse-expr-v3 spans pos-ref src)]")
            && parse_defn.contains(
                "(finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count body)"
            )
            && !parse_defn.contains("(parse-defn-tail-v3 spans pos-ref src defn-node param-count)")
            && !parse_defn.contains("(parse-defn-bodyless-or-body-v3\n")
            && !parse_defn.contains("parsed-ref"),
        "parse-defn-v3 は x86 stage2 の helper return 崩れを避けるため non-meta body parse を direct に持つべき"
    );
}

#[test]
fn parser_parse_defn_returns_explicit_parsed_after_root_pops_without_ref_roundtrip() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Parser.ls"))
        .expect("Parser.ls を読めること");
    let parse_defn = source
        .split("(defn parse-defn-v3 [spans pos-ref src]")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defmacro-v3").next())
        .expect("Parser.ls に parse-defn-v3 が存在すること");

    assert!(
        parse_defn.contains("(root_push result)")
            && parse_defn.contains("(root_pop)\n                                parsed")
            && !parse_defn.contains("result-slot")
            && !parse_defn.contains("(root_set result-slot")
            && !parse_defn.contains("parsed-ref"),
        "parse-defn-v3 は root_pop/root_set の戻り値に依存せず、root cleanup 後に explicit parsed を返すべき"
    );
}
