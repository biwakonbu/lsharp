
/// BOOT-04: stage1 が narrow subset を実際に stage2 Wasm へコンパイルできること
///
/// true fixed-point そのものではないが、Rust stage0 が生成した stage1 が
/// selfhost の Parser/Compiler/WasmEmit を使って実体の Wasm bytes を出力し、
/// その stage2 を実行できる最小 bootstrap 経路を固定する。
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_minimal_subset() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        ir (lower program)
        header (emit-header)
        type-sec (emit-type-section-main)
        function-sec (emit-function-section)
        export-sec (emit-export-section)
        code-sec (emit-code-section ir)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2-a (bootstrap-build-stage2 "(defn main [] 42)")
        stage2-b (bootstrap-build-stage2 "(defn main [] 7)")]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#;
    let stage1_source = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let first_output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("stage1 wasm の 1 回目実行に失敗");
    let second_output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("stage1 wasm の 2 回目実行に失敗");
    assert_eq!(
        first_output, second_output,
        "stage1 の stage2 生成結果が非決定的"
    );

    let modules = parse_emitted_wasm_modules(&first_output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_ne!(
        modules[0], modules[1],
        "異なる入力ソースから同一 stage2 Wasm が出力された"
    );

    for (idx, wasm) in modules.iter().enumerate() {
        assert_valid_wasm(wasm);
        assert!(
            wasm.len() > 8,
            "module[{idx}] の stage2 Wasm が短すぎる: {} bytes",
            wasm.len()
        );
    }

    assert_eq!(run_exported_i64(&modules[0], "_start"), 42);
    assert_eq!(run_exported_i64(&modules[1], "_start"), 7);
}

/// BOOT-04: stage1 が同じ tiny source から同一 stage2 Wasm を 2 回生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_identical_stage2_wasm_for_same_tiny_source() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        ir (lower program)
        header (emit-header)
        type-sec (emit-type-section-main)
        function-sec (emit-function-section)
        export-sec (emit-export-section)
        code-sec (emit-code-section ir)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [src "(defn main [] 42)"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#;
    let stage1_source = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same-source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ tiny source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    assert_eq!(run_exported_i64(&modules[0], "_start"), 42);
}

/// BOOT-04: stage1 が extended do block を含む stage2 Wasm も生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_extended_do_block() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        ir (lower program)
        header (emit-header)
        type-sec (emit-type-section-main)
        function-sec (emit-function-section)
        export-sec (emit-export-section)
        code-sec (emit-code-section ir)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (do 11 22 33 44 55 66 77))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("extended do block を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        77,
        "stage1 は do block の最終式まで含む stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が zero-arg 2 関数 + call を含む stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_zero_arg_call_program() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program program)
        ir-list (vector-get pair 1)
        func-count (vector-length ir-list)
        header (emit-header)
        type-sec (emit-type-section-main)
        function-sec (emit-function-section-count func-count)
        export-sec (emit-export-section-main-index (- func-count 1))
        code-sec (emit-code-section-list ir-list)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn helper [] 42) (defn main [] (helper))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("zero-arg call program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        42,
        "stage1 は helper→main call を含む stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が 1 引数関数呼出しを含む stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_single_param_call_program() {
    let stage2_src = r#"(defn add1 [x] (+ x 1)) (defn main [] (add1 41))"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("single-param call program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        42,
        "stage1 は 1 引数関数呼出しを含む stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が let local を含む stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_let_local_program() {
    let stage2_src = r#"(defn main [] (let [x 42] x))"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("let local program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        42,
        "stage1 は let local を含む stage2 Wasm を生成すること"
    );
}

// =============================================================================
// BOOT-04: 再帰・多関数プログラムの stage1→stage2 検証
// =============================================================================

/// BOOT-04: stage1 が自己再帰フィボナッチを含む stage2 Wasm を生成・実行できること
///
/// (defn fib [n] ...) + (defn main [] (fib 8)) → stage2 が 21 を返す
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_recursive_fibonacci() {
    let stage2_src =
        r#"(defn fib [n] (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (defn main [] (fib 8))"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("再帰フィボナッチを含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        21,
        "stage1 は fib(8)=21 を返す stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が自己再帰階乗を含む stage2 Wasm を生成・実行できること
///
/// (defn fact [n] ...) + (defn main [] (fact 5)) → stage2 が 120 を返す
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_recursive_factorial() {
    let stage2_src =
        r#"(defn fact [n] (if (<= n 1) 1 (* n (fact (- n 1))))) (defn main [] (fact 5))"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("再帰階乗を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        120,
        "stage1 は fact(5)=120 を返す stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が多関数ヘルパー再帰を含む stage2 Wasm を生成・実行できること
///
/// sum(n) を呼ぶ helper(x) + main の 3 関数構成で stage2 が 55 を返す
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_multi_function_helper_recursion() {
    let stage2_src = r#"(defn sum [n] (if (<= n 0) 0 (+ n (sum (- n 1))))) (defn helper [x] (sum x)) (defn main [] (helper 10))"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("多関数ヘルパー再帰を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        55,
        "stage1 は sum(10)=55 を経由する helper→main を含む stage2 Wasm を生成すること"
    );
}

/// BOOT-04: stage1 が string-char-at builtin を含む stage2 Wasm を valid module として生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_string_char_at_helper_program() {
    let stage2_src = r#"(defn first [s] (string-char-at s 0)) (defn main [] 0)"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("string-char-at helper program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 5),
        "string-char-at helper を含む stage2 Wasm は memory section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        0,
        "helper 未使用でも string-char-at builtin を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が string-length builtin を含む stage2 Wasm を valid module として生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_string_length_helper_program() {
    let stage2_src = r#"(defn len1 [s] (string-length s)) (defn main [] 0)"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("string-length helper program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 5),
        "string-length helper を含む stage2 Wasm は memory section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        0,
        "helper 未使用でも string-length builtin を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が string literal を data section に落とし込んだ stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_string_literal_data_section() {
    let harness = r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 memory-sec 0 (vector-length memory-sec))
        bytes4 (bootstrap-append-bytes bytes3 export-sec 0 (vector-length export-sec))
        bytes5 (bootstrap-append-bytes bytes4 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes5 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "(defn main [] \"abc\")")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("string literal data section program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let sections = extract_sections(&modules[0]);
    assert!(
        sections.iter().any(|(id, _)| *id == 11),
        "string literal を含む stage2 Wasm は data section を持つこと"
    );
    let data_section = extract_section_bytes(&modules[0], 11).expect("data section が見つからない");
    let expected_data = selfhost_string_object_bytes("abc");
    assert!(
        data_section
            .windows(expected_data.len())
            .any(|window| window == expected_data),
        "data section に string object header + bytes が含まれていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        1024,
        "string literal lowering の data base offset が不正"
    );
}

/// BOOT-04: stage1 が nested string literal を distinct offsets 付きで stage2 Wasm に落とし込めること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_nested_string_literal_data_section() {
    let stage2_source = r#"(defn main [] (do "ab" "cde"))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        header (emit-header)
        type-sec (emit-type-section-functions functions)
        function-sec (emit-function-section-functions functions)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))
        bytes3 (bootstrap-append-bytes bytes2 memory-sec 0 (vector-length memory-sec))
        bytes4 (bootstrap-append-bytes bytes3 export-sec 0 (vector-length export-sec))
        bytes5 (bootstrap-append-bytes bytes4 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes5 data-sec 0 (vector-length data-sec))))

(defn bootstrap-print-module-bytes [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (bootstrap-print-module-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do
      (print count)
      (bootstrap-print-module-bytes bytes 0 count)
      0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "{}")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("nested string literal data section program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).expect("data section が見つからない");
    let expected_data = selfhost_string_object_sequence(&["ab", "cde"]);
    assert!(
        data_section
            .windows(expected_data.len())
            .any(|window| window == expected_data),
        "nested string literal objects が data section に連結配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        selfhost_string_object_offset(1024, &["ab"]),
        "nested string literal の最終 offset が前段 object header + bytes を考慮していない"
    );
}
