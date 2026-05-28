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
            && probe.contains("(print (vector-length (ref-get debug-data-ref)))")
            && probe.contains("regular-data-ref (ref-new (vector-new 8))")
            && probe.contains(
                "(compile-all-src-decl-pairs-chunked all-pairs 0 n ftable regular-data-ref (vector-new 8))"
            )
            && probe.contains("(print 162)")
            && probe.contains("(print (vector-length regular-functions))")
            && probe.contains("(print (vector-length (ref-get regular-data-ref)))"),
        "compile pair probe は first pair の AST tag、debug compile、通常 chunked compile の関数数を出すべき"
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
    let pair_step = source
        .split("(defn compile-src-decl-pairs-step ")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn continue-compile-src-decl-pairs-step")
                .next()
        })
        .expect("compile-src-decl-pairs-step が存在すること");
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
    let chunked_diag_gate_pos = chunked
        .find("chunked-progress-mode (if (> (string-length (command-line-arg 8)) 0) 1 0)")
        .expect(
            "compile-all-src-decl-pairs-chunked の state 診断は progress arg で gated すること",
        );
    let state0_diag_pos = chunked
        .find("(print 167)")
        .expect("compile-all-src-decl-pairs-chunked は state0 accumulator 診断 marker を持つこと");
    let state1_diag_pos = chunked
        .find("(print 168)")
        .expect("compile-all-src-decl-pairs-chunked は state1 accumulator 診断 marker を持つこと");

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
            && chunked.contains("functions-root (root_push functions)")
            && chunked.contains("(root_set functions-root result)")
            && chunked.contains("(print (vector-get state0 0))")
            && chunked.contains("(print (vector-get state0 1))")
            && chunked.contains("(print (vector-length (vector-get state0 2)))")
            && chunked.contains("(print (vector-get state1 0))")
            && chunked.contains("(print (vector-get state1 1))")
            && chunked.contains("(print (vector-length (vector-get state1 2)))")
            && !chunked.contains("(vector-get\n    (continue-compile-src-decl-pairs-step-64"),
        "compile-all-src-decl-pairs-chunked は stage2 x86 native の state local/return 崩れを避けるため state と result slot を root してから cleanup するべき"
    );
    assert!(
        chunked_diag_gate_pos < state0_diag_pos && state0_diag_pos < state1_diag_pos,
        "compile-all-src-decl-pairs-chunked の 167/168 診断は progress gate 後、state0 から state1 の順で出すべき"
    );
    assert!(
        pair_step.contains(
            "pair-step-progress-mode (if (> (string-length (command-line-arg 8)) 0) 1 0)"
        ) && pair_step.contains("(print 169)")
            && pair_step.contains("(print idx)")
            && pair_step.contains("(print (vector-length decls))")
            && pair_step.contains("(print (vector-length functions))")
            && pair_step.contains("(print (vector-length updated-functions))")
            && pair_step.contains("(print (vector-length (ref-get data-ref)))"),
        "compile-src-decl-pairs-step は payload helper 内の source compile return と state allocation の境界を 169 で診断できるべき"
    );
    assert!(
        pair_step.contains("(print 170)")
            && pair_step.contains("(print (string-length src))")
            && pair_step.contains("(print (text-char-or-minus-one src 8))")
            && pair_step.contains("(print (text-char-or-minus-one src 19))")
            && pair_step.contains("(print (decl-tag-or-minus-one decls 0))")
            && pair_step.contains("(print (decl-tag-or-minus-one decls 3))"),
        "compile-src-decl-pairs-step は failing pair を source/module に対応付ける 170 診断を持つべき"
    );
    let updated_functions_pos = pair_step
        .find("updated-functions (compile-source-defn-functions-chunked")
        .expect("compile-src-decl-pairs-step は source-aware compile を呼ぶこと");
    let src_root_pos = pair_step
        .find("src-slot (root_push src)")
        .expect("compile-src-decl-pairs-step は source compile 前に src を root すること");
    let decls_root_pos = pair_step
        .find("decls-slot (root_push decls)")
        .expect("compile-src-decl-pairs-step は source compile 前に decls を root すること");
    let pre_diag_pos = pair_step
        .find("(print 171)")
        .expect("compile-src-decl-pairs-step は source compile 前の pair identity 診断を持つこと");
    assert!(
        src_root_pos < updated_functions_pos
            && decls_root_pos < updated_functions_pos
            && pre_diag_pos < updated_functions_pos,
        "compile-src-decl-pairs-step は source compile 中の GC で pair source/decls local が stale にならないよう、呼び出し前に root と pre 診断を置くべき"
    );
}

#[test]
fn compiler_mode_parse_pair_progress_splits_parse_pair_and_cache_storage() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/App/CompilerMode.ls"))
        .expect("CompilerMode.ls を読めること");
    let parse_pair = source
        .split("(defn parse-src-decl-pair [src]")
        .nth(1)
        .and_then(|tail| tail.split("(defn load-src-decl-pair-with-cache").next())
        .expect("parse-src-decl-pair が存在すること");
    let load_pair = source
        .split("(defn load-src-decl-pair-with-cache [path cache-ref parse-count-ref]")
        .nth(1)
        .and_then(|tail| tail.split("(defn make-pairs-step-state").next())
        .expect("load-src-decl-pair-with-cache が存在すること");

    let parse_progress_gate_pos = parse_pair
        .find("parse-pair-progress-mode (if (> (string-length (command-line-arg 8)) 0) 1 0)")
        .expect("parse-src-decl-pair の診断は progress arg で gated すること");
    let parse_program_pos = parse_pair
        .find("decls (parse-program src)")
        .expect("parse-src-decl-pair は parse-program の戻りを decls に保持すること");
    let parse_result_diag_pos = parse_pair
        .find("(print 217)")
        .expect("parse-src-decl-pair は parse-program 直後の decl tag 診断を持つこと");
    let pair_result_diag_pos = parse_pair
        .find("(print 218)")
        .expect("parse-src-decl-pair は make-src-decl-pair 後の decl tag 診断を持つこと");
    let make_pair_pos = parse_pair
        .find("pair (make-src-decl-pair src decls)")
        .expect("parse-src-decl-pair は src/decls pair を作ること");
    assert!(
        parse_progress_gate_pos < parse_program_pos
            && parse_program_pos < parse_result_diag_pos
            && parse_result_diag_pos < make_pair_pos
            && make_pair_pos < pair_result_diag_pos,
        "parse-src-decl-pair は parse result と pair result の境界を順序付き marker で分けるべき"
    );
    assert!(
        parse_pair.contains("(print (string-length src))")
            && parse_pair.contains("(print (vector-length decls))")
            && parse_pair.contains("(print (decl-tag-or-minus-one decls 0))")
            && parse_pair.contains("(print (decl-tag-or-minus-one decls 3))"),
        "parse-src-decl-pair の 217/218 は source 長、decl 長、先頭 decl tag を同じ形で出すべき"
    );

    let cache_entry_pos = load_pair
        .find("entry (make-src-decl-cache-entry fingerprint pair)")
        .expect("load-src-decl-pair-with-cache は fresh pair を cache entry 化すること");
    let cache_diag_pos = load_pair.find("(print 219)").expect(
        "load-src-decl-pair-with-cache は cache storage 直前の pair decl tag 診断を持つこと",
    );
    assert!(
        cache_entry_pos < cache_diag_pos
            && load_pair.contains("(print (vector-length (vector-get pair 1)))")
            && load_pair.contains("(print (decl-tag-or-minus-one (vector-get pair 1) 0))")
            && load_pair.contains("(print (decl-tag-or-minus-one (vector-get pair 1) 3))"),
        "load-src-decl-pair-with-cache は cache entry 作成後、保存前の pair decl tag を診断できるべき"
    );
}

#[test]
fn parser_parse_program_step_progress_splits_expr_append_and_state_storage() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Parser.ls"))
        .expect("Parser.ls を読めること");
    let step = source
        .split("(defn parse-program-step-v3 [spans pos-ref src result]")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn parse-program-step-64-loop-bounded")
                .next()
        })
        .expect("parse-program-step-v3 が存在すること");

    let gate_pos = step
        .find("parse-program-progress-mode (if (> (string-length (command-line-arg 8)) 0) 1 0)")
        .expect("parse-program-step-v3 の診断は progress arg で gated すること");
    let expr_pos = step
        .find("expr (parse-expr-v3 spans pos-ref src)")
        .expect("parse-program-step-v3 は parse-expr の戻りを保持すること");
    let expr_diag_pos = step
        .find("(print 221)")
        .expect("parse-program-step-v3 は parse-expr 直後の expr 診断を持つこと");
    let append_pos = step
        .find("next-result (vector-push-single-rooted-v3 result expr)")
        .expect("parse-program-step-v3 は expr を result に append すること");
    let append_diag_pos = step
        .find("(print 222)")
        .expect("parse-program-step-v3 は vector append 直後の result 診断を持つこと");
    let state_pos = step
        .find("state (do")
        .expect("parse-program-step-v3 は next-result を state 化すること");
    let state_diag_pos = step
        .find("(print 223)")
        .expect("parse-program-step-v3 は state 格納後の result 診断を持つこと");

    assert!(
        gate_pos < expr_pos
            && expr_pos < expr_diag_pos
            && expr_diag_pos < append_pos
            && append_pos < append_diag_pos
            && append_diag_pos < state_pos
            && state_pos < state_diag_pos,
        "parse-program-step-v3 は parse-expr / append / state 格納の順に tag を診断できるべき"
    );
    assert!(
        step.contains("before-pos (ref-get pos-ref)")
            && step.contains("before-kind (p-current spans pos-ref)")
            && step.contains(
                "head-kind (if (== before-kind 0) (span-kind spans (+ before-pos 1)) -1)"
            )
            && step.contains("result-len (vector-length result)")
            && step.contains("(print head-kind)")
            && step.contains("(print (vector-get expr 0))")
            && step.contains("(print (vector-length expr))")
            && step.contains("(print (vector-get (vector-get next-result result-len) 0))")
            && step.contains("(print (vector-get (vector-get (vector-get state 1) result-len) 0))"),
        "parse-program-step-v3 の 221/222/223 は位置、入力 token kind、expr tag、append 後 tag、state 後 tag を揃えて出すべき"
    );
}

#[test]
fn parser_parse_defn_body_branch_balances_local_roots_only() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Parser.ls"))
        .expect("Parser.ls を読めること");
    let parse_defn = source
        .split("(defn parse-defn-v3 [spans pos-ref src]")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defmacro-v3").next())
        .expect("parse-defn-v3 が存在すること");
    let body_branch = parse_defn
        .split("parsed-body (parse-expr-v3 spans pos-ref src)")
        .nth(1)
        .and_then(|tail| tail.split("parsed-defn))))").next())
        .expect("parse-defn-v3 は通常 body branch を持つこと");

    assert!(
        body_branch.contains("(root_push parsed-body)")
            && body_branch.contains(
                "parsed-defn (finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count parsed-body)"
            ),
        "parse-defn-v3 の通常 body branch は body を root して ref-backed wrapper finalize すること"
    );
    let root_pop_count = body_branch.matches("(root_pop)").count();
    assert_eq!(
        root_pop_count, 5,
        "parse-defn-v3 の通常 body branch は progress marker 用 parsed root と result / with-params / defn-node / body の local roots だけを pop し、caller root を pop してはいけない"
    );
}

#[test]
fn parser_parse_defn_progress_splits_finalize_body_handoff() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Parser.ls"))
        .expect("Parser.ls を読めること");
    let finalize_body = source
        .split("(defn finalize-defn-body-v3 [defn-node param-count body]")
        .nth(1)
        .and_then(|tail| tail.split("(defn maybe-append-defn-meta-v3").next())
        .expect("finalize-defn-body-v3 が存在すること");
    let parse_defn = source
        .split("(defn parse-defn-v3 [spans pos-ref src]")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defmacro-v3").next())
        .expect("parse-defn-v3 が存在すること");
    let body_branch = parse_defn
        .split("parsed-body (parse-expr-v3 spans pos-ref src)")
        .nth(1)
        .and_then(|tail| tail.split("parsed-defn))))").next())
        .expect("parse-defn-v3 は通常 body branch を持つこと");

    let finalize_set_pos = finalize_body
        .find("parsed (vector-set-at-rooted-v3 node-with-placeholder body-idx body)")
        .expect("finalize-defn-body-v3 は placeholder 後に body を set すること");
    let finalize_diag_pos = finalize_body
        .find("(print 225)")
        .expect("finalize-defn-body-v3 は parsed 作成直後の defn body 診断を持つこと");
    assert!(
        finalize_set_pos < finalize_diag_pos
            && finalize_body.contains("(print body-idx)")
            && finalize_body.contains("(print (vector-get node-with-placeholder 0))")
            && finalize_body.contains("(print (vector-length node-with-placeholder))")
            && finalize_body.contains("(print (vector-get parsed 0))")
            && finalize_body.contains("(print (vector-length parsed))"),
        "finalize-defn-body-v3 の 225 は body slot set 直後の tag/len を出すべき"
    );

    let parsed_pos = body_branch
        .find("parsed-defn (finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count parsed-body)")
        .expect("parse-defn-v3 は通常 body branch で parsed を作ること");
    let parsed_diag_pos = body_branch
        .find("(print 224)")
        .expect("parse-defn-v3 は finalize 後の parsed handoff 診断を持つこと");
    assert!(
        parsed_pos < parsed_diag_pos
            && body_branch.contains("(print param-count)")
            && body_branch.contains("(print (vector-get defn-node 0))")
            && body_branch.contains("(print (vector-length defn-node))")
            && body_branch.contains("(print (vector-get parsed-body 0))")
            && body_branch.contains("(print (vector-length parsed-body))")
            && body_branch.contains("(print (vector-get parsed-defn 0))")
            && body_branch.contains("(print (vector-length parsed-defn))")
            && body_branch.contains("(print (ref-get pos-ref))"),
        "parse-defn-v3 の 224 は finalize 後の defn/body/parsed tag と pos を出すべき"
    );
}

#[test]
fn parser_finalize_defn_wrapper_marks_pre_and_post_cleanup_handoff() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Parser.ls"))
        .expect("Parser.ls を読めること");
    let finalize_body = source
        .split("(defn finalize-defn-parsed-body-v3 [spans pos-ref defn-node param-count body]")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defn-bodyless-or-body-v3").next())
        .expect("finalize-defn-parsed-body-v3 が存在すること");

    let ref_set_pos = finalize_body
        .find("(ref-set parsed-ref parsed)")
        .expect("wrapper は parsed を parsed-ref に退避すること");
    let first_marker_pos = finalize_body
        .find("(print 226)")
        .expect("wrapper は cleanup 前の handoff marker 226 を持つこと");
    let second_marker_pos = finalize_body
        .rfind("(print 226)")
        .expect("wrapper は cleanup 後の handoff marker 226 を持つこと");
    let return_pos = finalize_body
        .rfind("(ref-get parsed-ref)")
        .expect("wrapper は parsed-ref を返すこと");

    assert!(
        ref_set_pos < first_marker_pos
            && first_marker_pos < second_marker_pos
            && second_marker_pos < return_pos
            && finalize_body.contains("(print 0)")
            && finalize_body.contains("(print 1)")
            && finalize_body.contains("(print (vector-get body 0))")
            && finalize_body.contains("(print (vector-length body))")
            && finalize_body.contains("(print (vector-get parsed 0))")
            && finalize_body.contains("(print (vector-length parsed))")
            && finalize_body.contains("(print (vector-get (ref-get parsed-ref) 0))")
            && finalize_body.contains("(print (vector-length (ref-get parsed-ref)))"),
        "finalize-defn-parsed-body-v3 は cleanup 前後で body / parsed / parsed-ref を比較できる marker 226 を出すべき"
    );
}

#[test]
fn parser_parse_defn_roots_finalize_result_before_progress_marker() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Parser.ls"))
        .expect("Parser.ls を読めること");
    let parse_defn = source
        .split("(defn parse-defn-v3 [spans pos-ref src]")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defmacro-v3").next())
        .expect("parse-defn-v3 が存在すること");
    let body_branch = parse_defn
        .split("(let [parsed-body (parse-expr-v3 spans pos-ref src)]")
        .nth(1)
        .expect("parse-defn-v3 は通常 body branch を持つこと");

    let parsed_pos = body_branch
        .find("parsed-defn (finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count parsed-body)")
        .expect("parse-defn-v3 は wrapper の戻り値を parsed に受けること");
    let root_parsed_pos = body_branch[parsed_pos..]
        .find("(root_push parsed-defn)")
        .map(|pos| parsed_pos + pos)
        .expect("parse-defn-v3 は progress marker 前に parsed を root すること");
    let marker_pos = body_branch
        .find("(print 224)")
        .expect("parse-defn-v3 は handoff marker 224 を持つこと");
    let first_pop_after_marker = body_branch[marker_pos..]
        .find("(root_pop)")
        .map(|pos| marker_pos + pos)
        .expect("marker 224 後に parsed root を外す root_pop が存在すること");

    assert!(
        parsed_pos < root_parsed_pos
            && root_parsed_pos < marker_pos
            && marker_pos < first_pop_after_marker,
        "parse-defn-v3 は progress mode の command-line-arg/print allocation で parsed local を壊さないよう、marker 224 の前後だけ parsed を root するべき"
    );
}

#[test]
fn parser_parse_defn_uses_branch_unique_body_bindings_for_x86_env_isolation() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Parser.ls"))
        .expect("Parser.ls を読めること");
    let parse_defn = source
        .split("(defn parse-defn-v3 [spans pos-ref src]")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defmacro-v3").next())
        .expect("Parser.ls に parse-defn-v3 が存在すること");
    let helper = source
        .split("(defn parse-defn-bodyless-or-body-v3 [spans pos-ref src defn-node param-count]")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn parse-defn-bodyless-or-body-with-meta-v3")
                .next()
        })
        .expect("Parser.ls に parse-defn-bodyless-or-body-v3 が存在すること");

    assert!(
        parse_defn.contains("empty-body (make-int-node 0)")
            && parse_defn.contains("empty-parsed (finalize-defn-body-v3 defn-node param-count empty-body)")
            && parse_defn.contains("parsed-body (parse-expr-v3 spans pos-ref src)")
            && parse_defn.contains(
                "parsed-defn (finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count parsed-body)"
            )
            && !parse_defn.contains("(let [body (make-int-node 0)]")
            && !parse_defn.contains("(let [body (parse-expr-v3 spans pos-ref src)]")
            && !parse_defn.contains("parsed (finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count body)"),
        "parse-defn-v3 は sibling branch の mutable env 汚染を避けるため body/parsed binding 名を再利用しない"
    );
    assert!(
        helper.contains("bodyless-body (make-int-node 0)")
            && helper.contains("bodyless-parsed (finalize-defn-body-v3 defn-node param-count bodyless-body)")
            && helper.contains("helper-body (parse-expr-v3 spans pos-ref src)")
            && helper.contains(
                "helper-parsed (finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count helper-body)"
            )
            && !helper.contains("(let [body (make-int-node 0)]")
            && !helper.contains("(let [body (parse-expr-v3 spans pos-ref src)]"),
        "parse-defn helper も body/parsed binding 名を branch 間で再利用しない"
    );
}

#[test]
fn compiler_mode_compile_file_functions_roots_chunked_result_before_cleanup() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/App/CompilerMode.ls"))
        .expect("CompilerMode.ls を読めること");
    let body = source
        .split("(defn compile-file-functions-with-cache ")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn compile-file-functions-payload-with-cache")
                .next()
        })
        .expect("compile-file-functions-with-cache が存在すること");

    let result_pos = body
        .find("functions (compile-all-src-decl-pairs-chunked")
        .expect("compile-file-functions-with-cache は chunked compile result を持つこと");
    let root_pos = body
        .find("(root_push functions)")
        .expect("compile-file-functions-with-cache は functions result を root すること");
    let first_pop_pos = body
        .find("(root_pop)")
        .expect("compile-file-functions-with-cache は cleanup root_pop を持つこと");

    assert!(
        result_pos < root_pos && root_pos < first_pop_pos,
        "compile-file-functions-with-cache は chunked compile result を cleanup 前に root するべき"
    );
}

#[test]
fn compiler_mode_payload_with_cache_roots_payload_result_before_cleanup() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/App/CompilerMode.ls"))
        .expect("CompilerMode.ls を読めること");
    let body = source
        .split("(defn compile-file-functions-payload-with-cache ")
        .nth(1)
        .and_then(|tail| tail.split("(defn compile-file-mode-cache-probe").next())
        .expect("compile-file-functions-payload-with-cache が存在すること");

    let cache_root_pos = body.find("cache-root (root_push cache-ref)").expect(
        "compile-file-functions-payload-with-cache は cache-ref を pairs 取得前に root すること",
    );
    let parse_count_root_pos = body
        .find("parse-count-root (root_push parse-count-ref)")
        .expect(
            "compile-file-functions-payload-with-cache は parse-count-ref を pairs 取得前に root すること",
        );
    let all_pairs_pos = body
        .find("all-pairs (compile-file-pairs-with-cache")
        .expect("compile-file-functions-payload-with-cache は pairs を自前で取得すること");
    let all_pairs_root_pos = body
        .find("(root_push all-pairs)")
        .expect("compile-file-functions-payload-with-cache は all-pairs を root すること");
    let data_ref_root_pos = body.find("(root_push data-ref)").expect(
        "compile-file-functions-payload-with-cache は data-ref を compile 前に root すること",
    );
    let pairs_diag_pos = body
        .find("(print 165)")
        .expect("compile-file-functions-payload-with-cache は pairs 取得直後の helper 内診断 marker を持つこと");
    let register_pos = body
        .find("reg-result (register-all-pairs")
        .expect("compile-file-functions-payload-with-cache は ftable を自前で登録すること");
    let register_diag_pos = body
        .find("(print 166)")
        .expect("compile-file-functions-payload-with-cache は register 直後の helper 内診断 marker を持つこと");
    let compile_pos = body
        .find("functions (compile-all-src-decl-pairs-chunked")
        .expect("compile-file-functions-payload-with-cache は payload 作成前に functions を直接 compile すること");
    let diag_gate_pos = body
        .find("payload-helper-progress-mode (if (> (string-length (command-line-arg 8)) 0) 1 0)")
        .expect("compile-file-functions-payload-with-cache の helper 内診断は progress arg で gated すること");
    let pre_payload_diag_pos = body
        .find("(print 163)")
        .expect("compile-file-functions-payload-with-cache は payload 構築直前の helper 内診断 marker を持つこと");
    let payload_slot_pos = body.find("payload-slot (root_push payload-base)").expect(
        "compile-file-functions-payload-with-cache は payload vector slot を root すること",
    );
    let payload1_set_pos = body
        .find("(root_set payload-slot payload1)")
        .expect("compile-file-functions-payload-with-cache は payload1 を slot に反映すること");
    let payload2_set_pos = body.find("(root_set payload-slot payload2)").expect(
        "compile-file-functions-payload-with-cache は payload2 を cleanup 前に slot に反映すること",
    );
    let post_payload_diag_pos = body
        .find("(print 164)")
        .expect("compile-file-functions-payload-with-cache は payload 構築後の helper 内診断 marker を持つこと");
    let first_pop_pos = body
        .find("(root_pop)")
        .expect("compile-file-functions-payload-with-cache は cleanup root_pop を持つこと");

    assert!(
        cache_root_pos < parse_count_root_pos
            && parse_count_root_pos < diag_gate_pos
            && diag_gate_pos < all_pairs_pos
            && all_pairs_pos < all_pairs_root_pos
            && all_pairs_root_pos < data_ref_root_pos
            && data_ref_root_pos < pairs_diag_pos
            && pairs_diag_pos < register_pos
            && data_ref_root_pos < register_pos
            && register_pos < register_diag_pos
            && register_diag_pos < compile_pos
            && compile_pos < pre_payload_diag_pos
            && pre_payload_diag_pos < payload_slot_pos
            && payload_slot_pos < payload1_set_pos
            && payload1_set_pos < payload2_set_pos
            && payload2_set_pos < post_payload_diag_pos
            && post_payload_diag_pos < first_pop_pos,
        "compile-file-functions-payload-with-cache は x86 native の payload handoff で compile-file-functions-with-cache の return local を跨がず、functions が root されたまま payload を組むべき"
    );
    assert!(
        !body.contains("compile-file-functions-with-cache path"),
        "compile-file-functions-payload-with-cache は functions return cleanup 経路を跨がず payload を組むべき"
    );
}

#[test]
fn compiler_mode_payload_with_cache_prints_state_helper_ftable_probe() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/App/CompilerMode.ls"))
        .expect("CompilerMode.ls を読めること");
    let body = source
        .split("(defn compile-file-functions-payload-with-cache ")
        .nth(1)
        .and_then(|tail| tail.split("(defn compile-file-mode-cache-probe").next())
        .expect("compile-file-functions-payload-with-cache が存在すること");

    let register_diag_pos = body
        .find("(print 166)")
        .expect("register 直後の既存診断 marker が存在すること");
    let ftable_probe_pos = body
        .find("(print 9000000055)")
        .expect("state helper ftable lookup 診断 marker が存在すること");
    let compile_pos = body
        .find("functions (compile-all-src-decl-pairs-chunked")
        .expect("payload 作成前の直接 compile が存在すること");

    assert!(
        body.contains("helper-hash (name-hash \"make-callable-object-offset-state\" 0 33)")
            && body.contains(
                "local-state-hash (name-hash \"linux-x86-probe-callable-object-offset-state\" 0 44)"
            )
            && body.contains("call-after-hash (name-hash \"linux-x86-call-after-marker\" 0 27)")
            && body.contains("main-hash (name-hash \"main\" 0 4)")
            && body.contains("(print helper-hash)")
            && body.contains("(print (ftable-lookup ftable helper-hash))")
            && body.contains("(print local-state-hash)")
            && body.contains("(print (ftable-lookup ftable local-state-hash))")
            && body.contains("(print call-after-hash)")
            && body.contains("(print (ftable-lookup ftable call-after-hash))")
            && body.contains("(print main-hash)")
            && body.contains("(print (ftable-lookup ftable main-hash))")
            && body.contains("(print (vector-get reg-result 1))")
            && body.contains("(print (vector-length ftable))"),
        "payload helper は register 直後に imported/local/call-after/main の ftable lookup を出すべき"
    );
    assert!(
        register_diag_pos < ftable_probe_pos && ftable_probe_pos < compile_pos,
        "ftable lookup 診断は register-all-pairs 後、compile 前に出すべき"
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
            && body.contains("(root_set functions-slot result)")
            && body.contains("(root_set functions-slot skip-result)")
            && body.contains("next-defn-idx (+ idx 1)")
            && body.contains("(make-compile-step-state 0 next-defn-idx next-functions)")
            && !body.contains("compile-defn-functions-step-finish functions compiled-fn idx"),
        "compile-defn-functions-step-with-source は stage2 x86 native の local 保持崩れを避けるため next-functions と result state を root slot へ戻すべき"
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

    let state_alloc_pos = body
        .find("(let [next-skip-idx (+ idx 1)")
        .expect("skip branch は next idx を local 化してから state allocation するべき");
    let first_pop_after_state = body[state_alloc_pos..]
        .find("(root_pop)")
        .map(|pos| state_alloc_pos + pos)
        .expect("skip branch は state allocation 後に root を解放するべき");

    assert!(
        state_alloc_pos < first_pop_after_state
            && body.contains("(make-compile-step-state 0 next-skip-idx functions)")
            && body.contains("              result))))))))")
            && !body.contains(
                "(root_pop)\n          (root_pop)\n          (root_pop)\n          (root_pop)\n          (root_pop)\n          (make-compile-step-state 0 (+ idx 1) functions)"
            ),
        "compile-defn-functions-step-with-source の non-defn skip branch は functions を root したまま state allocation するべき"
    );
}

#[test]
fn compiler_source_step_body_progress_marks_single_step_state() {
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
            "source-step-progress-mode (if (> (string-length (command-line-arg 8)) 0) 1 0)"
        ) && body.contains("result-root (root_push result)")
            && body.find("result-root (root_push result)") < body.find("(print 9000000077)")
            && body.contains("(print 9000000077)")
            && body.contains("(print (vector-length next-functions))")
            && body.contains("(print (vector-get result 1))")
            && body.contains("(print (vector-length (vector-get result 2)))")
            && body.contains("(print 9000000078)")
            && body.contains("(print (vector-length functions))"),
        "compile-defn-functions-step-with-source-body-impl-3 は source chunk state の破損点を single-step result で観測できるべき"
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
            && body.contains("functions-root (root_push functions)")
            && body.contains("(root_set functions-root result)")
            && body.contains(
                "source-chunk-progress-mode (if (> (string-length (command-line-arg 8)) 0) 1 0)"
            )
            && body.contains("(print 213)")
            && body.contains("(print (vector-length functions))")
            && body.contains("(print (vector-get state0 0))")
            && body.contains("(print (vector-get state0 1))")
            && body.contains("(print (vector-length (vector-get state0 2)))")
            && body.contains("(print 214)")
            && body.contains("(print (vector-get state1 0))")
            && body.contains("(print (vector-get state1 1))")
            && body.contains("(print (vector-length (vector-get state1 2)))")
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
    let continue_times = source
        .split("(defn continue-compile-defn-functions-step-times-with-source")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn compile-defn-functions-step-8-with-source")
                .next()
        })
        .expect("continue-compile-defn-functions-step-times-with-source が存在すること");
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

    assert!(
        continue_step.contains("state-slot (root_push state)")
            && continue_step.contains("(root_set state-slot result)")
            && continue_times.contains("decls-slot (root_push decls)")
            && continue_times.contains("(root_set decls-slot result)")
            && step8.contains("decls-slot (root_push decls)")
            && step8.contains("(root_set decls-slot result)")
            && continue8.contains("decls-slot (root_push decls)")
            && continue8.contains("(root_set decls-slot result)")
            && step64.contains("decls-slot (root_push decls)")
            && step64.contains("(root_set decls-slot result)")
            && continue64.contains("decls-slot (root_push decls)")
            && continue64.contains("(root_set decls-slot result)")
            && !continue_times.contains("next-state-root (root_push next-state)")
            && !step64.contains("state-root (root_push state)")
            && !continue64.contains("next-state-root (root_push next-state)"),
        "source step wrappers は x86 native の cleanup/return 境界で result state を bottom 側 root slot に戻すべき"
    );
}

#[test]
fn compiler_source_chunked_high_markers_split_step_vs_continue_state_loss() {
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
    let continue_times = source
        .split("(defn continue-compile-defn-functions-step-times-with-source")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn compile-defn-functions-step-8-with-source")
                .next()
        })
        .expect("continue-compile-defn-functions-step-times-with-source が存在すること");
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

    assert!(
        continue_step.contains("continue-step-progress-mode")
            && continue_step.contains("(print 9000000070)")
            && continue_step.contains("(print (vector-length next-functions))")
            && continue_step.contains("(print 9000000071)")
            && continue_step.contains("(print (vector-get result 0))")
            && continue_step.contains("(print (vector-length (vector-get result 2)))"),
        "continue-compile-defn-functions-step-with-source は単発 step の入力 accumulator と result state を高い sentinel で分離して観測できるべき"
    );
    assert!(
        continue_times.contains("continue-times-progress-mode")
            && continue_times.contains("(print 9000000072)")
            && continue_times.contains("(print remaining)")
            && continue_times.contains("(print (vector-length (vector-get state 2)))")
            && continue_times.contains("(print (vector-length (vector-get next-state 2)))"),
        "continue-compile-defn-functions-step-times-with-source は times recursion の state handoff を高い sentinel で観測できるべき"
    );
    assert!(
        step64.contains("step64-progress-mode")
            && step64.contains("(print 9000000073)")
            && step64.contains("(print (vector-length (vector-get state 2)))")
            && step64.contains("(print (vector-length (vector-get result 2)))")
            && continue64.contains("continue64-progress-mode")
            && continue64.contains("(print 9000000074)")
            && continue64.contains("(print (vector-length (vector-get next-state 2)))")
            && continue64.contains("(print (vector-length (vector-get result 2)))"),
        "step-64 / continue-64 wrappers は 64 境界で functions accumulator が落ちるかを高い sentinel で観測できるべき"
    );
}

#[test]
fn compiler_source_step_wrapper_chain_marks_return_handoff() {
    let source =
        std::fs::read_to_string(workspace_root().join("selfhost/src/Backend/Wasm/Compiler.ls"))
            .expect("Compiler.ls を読めること");

    for (name, marker_id) in [
        ("compile-defn-functions-step-with-source", "0"),
        ("compile-defn-functions-step-with-source-body", "1"),
        ("compile-defn-functions-step-with-source-body-impl", "2"),
        ("compile-defn-functions-step-with-source-body-impl-2", "3"),
    ] {
        let body = source
            .split(&format!("(defn {name}"))
            .nth(1)
            .and_then(|tail| tail.split("(defn ").next())
            .unwrap_or_else(|| panic!("Compiler.ls に {name} が存在すること"));

        assert!(
            body.contains("source-step-wrapper-progress-mode")
                && body.contains("(print 9000000075)")
                && body.contains(&format!("(print {marker_id})"))
                && body.contains("(print (vector-get result 0))")
                && body.contains("(print (vector-get result 1))")
                && body.contains("(print (vector-length (vector-get result 2)))")
                && body.contains("result-root (root_push result)"),
            "{name} は source step wrapper return handoff を high marker で観測できるべき"
        );
    }
}

#[test]
fn compiler_base_step_state_marks_entry_and_result_handoff() {
    let source =
        std::fs::read_to_string(workspace_root().join("selfhost/src/Backend/Wasm/CompilerBase.ls"))
            .expect("CompilerBase.ls を読めること");
    let body = source
        .split("(defn make-compile-step-state [done next-idx next-value]")
        .nth(1)
        .and_then(|tail| tail.split("(defn ").next())
        .expect("CompilerBase.ls に make-compile-step-state が存在すること");

    assert!(
        body.contains("compile-step-state-progress-mode")
            && body.contains("(print 9000000076)")
            && body.contains("(print 0)")
            && body.contains("(print done)")
            && body.contains("(print next-idx)")
            && body.contains("(print (vector-length next-value))")
            && body.contains("(print 1)")
            && body.contains("(print (vector-get state 0))")
            && body.contains("(print (vector-get state 1))")
            && body.contains("(print (vector-length (vector-get state 2)))")
            && body.contains("state-root (root_push state)"),
        "make-compile-step-state は x86 native の argument handoff と constructed state を同じ high marker で比較できるべき"
    );
}

#[test]
fn compiler_source_step_binds_next_idx_before_state_allocation() {
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
        body.contains("(let [next-defn-idx (+ idx 1)]")
            && body
                .contains("(let [result (make-compile-step-state 0 next-defn-idx next-functions)]")
            && body.contains("(let [next-skip-idx (+ idx 1)]")
            && body
                .contains("(let [skip-result (make-compile-step-state 0 next-skip-idx functions)]")
            && !body.contains("(make-compile-step-state 0 (+ idx 1) next-functions)")
            && !body.contains("(make-compile-step-state 0 (+ idx 1) functions)")
            && !body.contains("next-defn-idx (+ idx 1)\n                      result")
            && !body.contains("next-skip-idx (+ idx 1)\n              skip-result"),
        "compile-defn-functions-step-with-source は x86 native の argument-list value-window 破損を避けるため next idx 計算と state allocation を別 let に分けるべき"
    );
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
        body.contains("(finalize-defn-body-v3 defn-node param-count bodyless-body)")
            && body.contains(
                "(finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count helper-body)"
            )
            && !body.contains("node-with-placeholder"),
        "parse-defn body finalize は x86 stage2 の local 保持崩れを避けるため小さい rooted helper に委譲するべき"
    );
}

#[test]
fn parser_parse_defn_uses_ref_backed_finalize_wrapper_without_tail_refactor() {
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
            && parse_defn.contains("(let [parsed-body (parse-expr-v3 spans pos-ref src)]")
            && parse_defn.contains(
                "(finalize-defn-parsed-body-v3 spans pos-ref defn-node param-count parsed-body)"
            )
            && !parse_defn.contains("(parse-defn-tail-v3 spans pos-ref src defn-node param-count)")
            && !parse_defn.contains("(parse-defn-bodyless-or-body-v3\n")
            && !parse_defn.contains("parsed-ref"),
        "parse-defn-v3 は tail/refactor を戻さず、non-meta body finalize を wrapper に委譲するべき"
    );
}

#[test]
fn parser_finalize_defn_parsed_body_uses_ref_backed_return_after_cleanup() {
    let source = std::fs::read_to_string(workspace_root().join("selfhost/src/Syntax/Parser.ls"))
        .expect("Parser.ls を読めること");
    let finalize_body = source
        .split("(defn finalize-defn-parsed-body-v3 [spans pos-ref defn-node param-count body]")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defn-bodyless-or-body-v3").next())
        .expect("Parser.ls に finalize-defn-parsed-body-v3 が存在すること");

    assert!(
        finalize_body.contains("parsed-ref (ref-new (make-int-node 0))")
            && finalize_body.contains("(root_push parsed-ref)")
            && finalize_body.contains("(ref-set parsed-ref parsed)")
            && finalize_body.contains("(ref-get parsed-ref)")
            && !finalize_body.contains("result-slot")
            && !finalize_body.contains("(root_set result-slot"),
        "finalize-defn-parsed-body-v3 は helper return を parsed-ref に退避し、cleanup 後に ref-get で返すべき"
    );
}
