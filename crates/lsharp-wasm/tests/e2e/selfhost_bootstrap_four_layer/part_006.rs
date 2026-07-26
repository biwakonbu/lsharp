
/// BOOT-04: stage1 が実 path string を伴う read-file program を stage2 Wasm として生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_read_file_path_string_program() {
    let stage2_source =
        r#"(defn main [] (string-length (read-file "fixture.txt")))"#.replace('"', "\\\"");
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
        type-sec (emit-type-section-alloc-print-main)
        import-sec (emit-import-section-alloc-print-read)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 3 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))
        bytes6 (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes6 data-sec 0 (vector-length data-sec))))

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
        .expect("path string read-file program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).unwrap_or_default();
    assert!(
        data_section
            .windows("fixture.txt".len())
            .any(|window| window == b"fixture.txt"),
        "read-file path literal は data section に残ること"
    );
    let (result, printed) = run_exported_i64_with_alloc_print_read_path_imports(
        &modules[0],
        "_start",
        "fixture.txt",
        "hello from file",
    );
    assert_eq!(
        result, 15,
        "path string read-file program を含む stage2 Wasm の戻り値が不正"
    );
    assert!(
        printed.is_empty(),
        "read-file slice では print output は不要"
    );
}

/// BOOT-04: stage1 が同じ source-aware read-file path string source から同一 stage2 Wasm を 2 回生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_identical_read_file_path_stage2_wasm_for_same_source() {
    let stage2_source =
        r#"(defn main [] (string-length (read-file "fixture.txt")))"#.replace('"', "\\\"");
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
        type-sec (emit-type-section-alloc-print-main)
        import-sec (emit-import-section-alloc-print-read)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 3 0)
        code-sec (emit-code-section-functions functions)
        data-sec (emit-data-section data 1024)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))
        bytes6 (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))]
    (bootstrap-append-bytes bytes6 data-sec 0 (vector-length data-sec))))

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
  (let [src "{}"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same path-string read-file source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ source-aware read-file path string source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).unwrap_or_default();
    assert!(
        data_section
            .windows("fixture.txt".len())
            .any(|window| window == b"fixture.txt"),
        "repeatability でも read-file path literal は data section に残ること"
    );
    let (result, printed) = run_exported_i64_with_alloc_print_read_path_imports(
        &modules[0],
        "_start",
        "fixture.txt",
        "hello from file",
    );
    assert_eq!(result, 15);
    assert!(printed.is_empty());
}

/// BOOT-04: stage1 が command-line-arg builtin を含む stage2 Wasm を生成し実行できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_command_line_arg_program() {
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
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-print-main)
        import-sec (emit-import-section-alloc-print-read-arg)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

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
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (string-length (command-line-arg 1)))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("command-line-arg program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "command-line-arg program を含む stage2 Wasm は import section を持つこと"
    );
    let (result, printed) = run_exported_i64_with_alloc_print_read_arg_imports(
        &modules[0],
        "_start",
        "",
        &["cli", "hello-argv"],
    );
    assert_eq!(
        result, 10,
        "command-line-arg program を含む stage2 Wasm の戻り値が不正"
    );
    assert!(
        printed.is_empty(),
        "command-line-arg slice では print output は不要"
    );
}

/// BOOT-04: stage1 が同じ command-line-arg helper source から同一 stage2 Wasm を 2 回生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_identical_arg_helper_stage2_wasm_for_same_source() {
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
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-alloc-print-main)
        import-sec (emit-import-section-alloc-print-read-arg)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

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
  (let [src "(defn main [] (string-length (command-line-arg 1)))"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same arg-helper source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ command-line-arg helper source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let (result, printed) = run_exported_i64_with_alloc_print_read_arg_imports(
        &modules[0],
        "_start",
        "",
        &["cli", "hello-argv"],
    );
    assert_eq!(result, 10);
    assert!(printed.is_empty());
}

/// BOOT-04: stage1 が generalized 4-helper path で alloc+print+read-file+command-line-arg stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_generalized_alloc_print_read_arg_helper_quad() {
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
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-helper-quad-main (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-command-line-arg))
        import-sec (emit-import-section-helper-quad (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-command-line-arg))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

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
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (string-length (command-line-arg 1)))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("generalized alloc+print+read-file+command-line-arg quad program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let (result, printed) = run_exported_i64_with_alloc_print_read_arg_imports(
        &modules[0],
        "_start",
        "",
        &["cli", "hello-argv"],
    );
    assert_eq!(
        result, 10,
        "generalized alloc+print+read-file+command-line-arg quad stage2 Wasm の戻り値が不正"
    );
    assert!(
        printed.is_empty(),
        "generalized alloc+print+read-file+command-line-arg quad slice では print output は不要"
    );
}

/// BOOT-04: stage1 が同じ generalized alloc+print+read-file+command-line-arg source から同一 stage2 Wasm を 2 回生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_identical_arg_helper_quad_stage2_wasm_for_same_source() {
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
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        header (emit-header)
        type-sec (emit-type-section-helper-quad-main (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-command-line-arg))
        import-sec (emit-import-section-helper-quad (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-command-line-arg))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
        code-sec (emit-code-section-functions functions)
        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))
        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))
        bytes2 (bootstrap-append-bytes bytes1 import-sec 0 (vector-length import-sec))
        bytes3 (bootstrap-append-bytes bytes2 function-sec 0 (vector-length function-sec))
        bytes4 (bootstrap-append-bytes bytes3 memory-sec 0 (vector-length memory-sec))
        bytes5 (bootstrap-append-bytes bytes4 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes bytes5 code-sec 0 (vector-length code-sec))))

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
  (let [src "(defn main [] (string-length (command-line-arg 1)))"
        stage2-a (bootstrap-build-stage2 src)
        stage2-b (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module stage2-a)
      (bootstrap-print-module stage2-b)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("same arg-helper quad source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ generalized arg helper quad source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let (result, printed) = run_exported_i64_with_alloc_print_read_arg_imports(
        &modules[0],
        "_start",
        "",
        &["cli", "hello-argv"],
    );
    assert_eq!(result, 10);
    assert!(printed.is_empty());
}

// =============================================================================
// BOOT-04 リグレッション: file-fed stage2 generator self-feed proxy / deep recursive trap
// =============================================================================

/// BOOT-04 リグレッション: bootstrap-append-bytes の末尾再帰トラップ再現
///
/// `bootstrap-append-bytes` はバイト列を 1 バイトずつコピーする直接再帰で実装されており、
/// TCO (末尾呼び出し最適化) なしの Wasm では大きな配列に対してスタックオーバーフローが発生する。
///
/// この問題を最小限の形で再現する:
/// - stage2 ソース = N 個の単純な 0 引数関数からなるプログラム
/// - stage1 (selfhost CLI runtime) がそのプログラムをコンパイルして Wasm を組み立てる
/// - code section が大きくなるほど bootstrap-append-bytes の再帰深度が増す
/// - N が十分に大きいとき、stage1 実行時に Wasm スタックトラップが発生する
#[test]
#[ignore]
fn test_e2e_boot04_bootstrap_append_bytes_deep_recursion_trap_repro() {
    let build_stage2_src = |n_funcs: usize| -> String {
        let mut s = String::new();
        for i in 0..n_funcs {
            s.push_str(&format!("(defn fn{i:04} [] {i}) "));
        }
        s.push_str("(defn main [] 0)");
        s
    };

    let make_harness = |stage2_src: &str| -> String {
        format!(
            concat!(
                "(defn bootstrap-append-bytes [dst src idx count]\n",
                "  (if (>= idx count)\n",
                "    dst\n",
                "    (bootstrap-append-bytes\n",
                "      (vector-push dst (vector-get src idx))\n",
                "      src (+ idx 1) count)))\n",
                "(defn bootstrap-build-stage2 [src]\n",
                "  (let [program (parse-program src)\n",
                "        pair (compile-program-functions program)\n",
                "        functions (vector-get pair 1)\n",
                "        func-count (vector-length functions)\n",
                "        header (emit-header)\n",
                "        type-sec (emit-type-section-functions functions)\n",
                "        function-sec (emit-function-section-functions functions)\n",
                "        export-sec (emit-export-section-main-index (- func-count 1))\n",
                "        code-sec (emit-code-section-functions functions)\n",
                "        bytes0 (bootstrap-append-bytes (vector-new 64) header 0 (vector-length header))\n",
                "        bytes1 (bootstrap-append-bytes bytes0 type-sec 0 (vector-length type-sec))\n",
                "        bytes2 (bootstrap-append-bytes bytes1 function-sec 0 (vector-length function-sec))\n",
                "        bytes3 (bootstrap-append-bytes bytes2 export-sec 0 (vector-length export-sec))]\n",
                "    (bootstrap-append-bytes bytes3 code-sec 0 (vector-length code-sec))))\n",
                "(defn bootstrap-print-module-bytes [bytes idx count]\n",
                "  (if (>= idx count) 0\n",
                "    (do (print (vector-get bytes idx))\n",
                "        (bootstrap-print-module-bytes bytes (+ idx 1) count))))\n",
                "(defn bootstrap-print-module [bytes]\n",
                "  (let [count (vector-length bytes)]\n",
                "    (do (print count) (bootstrap-print-module-bytes bytes 0 count) 0)))\n",
                "(defn main []\n",
                "  (let [stage2 (bootstrap-build-stage2 \"{s2}\")]\n",
                "    (do (bootstrap-print-module stage2) 0)))\n",
            ),
            s2 = stage2_src
        )
    };

    // N=5: code section ~100 bytes → 再帰は浅い → 成功するはず
    {
        let small_src = build_stage2_src(5);
        let harness = make_harness(&small_src);
        let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
        let stage1_wasm = compile_only(&stage1_source);
        assert_valid_wasm(&stage1_wasm);
        let result = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm);
        assert!(
            result.is_ok(),
            "N=5 では bootstrap-append-bytes トラップが発生しないはず: {:?}",
            result.err()
        );
        let output = result.unwrap();
        let modules = parse_emitted_wasm_modules(&output, 1);
        assert_eq!(
            modules.len(),
            1,
            "N=5 では stage2 モジュールが 1 つ生成されるはず"
        );
        assert_valid_wasm(&modules[0]);
    }

    // N=2000: code section ~30,000 bytes
    // BOOT-04 修正済み: self-TCO (自己末尾呼び出し最適化) により再帰がループに変換される
    // lsharp-ir/src/lower/decl.rs の apply_self_tco により、
    // bootstrap-append-bytes のような自己末尾再帰関数がスタックを消費しなくなった
    {
        let large_src = build_stage2_src(2000);
        let harness = make_harness(&large_src);
        let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
        let stage1_wasm = compile_only(&stage1_source);
        assert_valid_wasm(&stage1_wasm);
        let result = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm);
        assert!(
            result.is_ok(),
            "BOOT-04 リグレッション: N=2000 で bootstrap-append-bytes がトラップした。\n\
             self-TCO が正しく動作していない可能性があります。\n\
             エラー: {:?}",
            result.err()
        );
        let output = result.unwrap();
        let modules = parse_emitted_wasm_modules(&output, 1);
        assert_eq!(
            modules.len(),
            1,
            "N=2000 では stage2 モジュールが 1 つ生成されるはず"
        );
        assert_valid_wasm(&modules[0]);
    }
}

#[test]
#[ignore]
fn test_e2e_boot04_selfhost_compile_program_functions_handles_many_defns() {
    let mut stage2_src = String::new();
    for i in 0..2000 {
        stage2_src.push_str(&format!("(defn fn{i:04} [] {i}) "));
    }
    stage2_src.push_str("(defn main [] 0)");

    let harness = format!(
        concat!(
            "(defn main []\n",
            "  (let [program (parse-program \"{s2}\")\n",
            "        pair (compile-program-functions program)\n",
            "        functions (vector-get pair 1)]\n",
            "    (do (print (vector-length functions)) 0)))\n",
        ),
        s2 = stage2_src
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);
    let result = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm);
    assert!(
        result.is_ok(),
        "compile-program-functions が大量 defn でトラップした: {:?}",
        result.err()
    );
    assert_eq!(
        result.unwrap().trim(),
        "2001",
        "2000 個の defn + main の 2001 関数が登録されるべき"
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_selfhost_compile_program_functions_with_source_handles_deep_let_chain() {
    let mut nested_expr = "0".to_string();
    for i in (0..512).rev() {
        nested_expr = format!("(let [v{i:04} {i}] {nested_expr})");
    }
    let stage2_src = format!("(defn main [] {nested_expr})");

    let harness = format!(
        concat!(
            "(defn main []\n",
            "  (let [program (parse-program \"{s2}\")\n",
            "        pair (compile-program-functions-with-source \"{s2}\" program)\n",
            "        functions (vector-get pair 1)]\n",
            "    (do (print (vector-length functions)) 0)))\n",
        ),
        s2 = stage2_src
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);
    let result = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm);
    assert!(
        result.is_ok(),
        "compile-program-functions-with-source が深い let 連鎖でトラップした: {:?}",
        result.err()
    );
    assert_eq!(
        result.unwrap().trim(),
        "1",
        "深い let 連鎖でも 1 関数のメタデータを返すべき"
    );
}
