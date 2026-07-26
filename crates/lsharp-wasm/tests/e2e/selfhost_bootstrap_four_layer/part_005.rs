
/// BOOT-04: stage1 が non-literal string key map builtins を含む stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_non_literal_string_key_map_program() {
    let stage2_source = r#"(defn main [] (do (print (let [key (read-file "fixture.txt")] (let [m0 (map-new)] (let [m1 (map-insert m0 key 42)] (map-get m1 key))))) 0))"#.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [src "{}"
        program (parse-program src)
        pair (compile-program-functions-with-source src program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        stage2 (build-wasm-bytes-wasi functions data)]
    (do
      (print-wasm-module stage2)
      0)))
"#,
        stage2_source
    );
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("non-literal string key map program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).unwrap_or_default();
    assert!(
        data_section
            .windows("fixture.txt".len())
            .any(|window| window == b"fixture.txt"),
        "read-file path literal bytes は data section に配置されること"
    );
    let printed = run_wasm_with_six_imports_compiler_mode(&modules[0], "aa", &[])
        .expect("non-literal string key map builtins を含む stage2 Wasm の実行に失敗");
    assert_eq!(
        printed.trim(),
        "42",
        "non-literal string key map builtins を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が generalized 4-helper path で alloc+print+read-file+__fnv1a_hash stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_generalized_alloc_print_read_hash_helper_quad() {
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
        type-sec (emit-type-section-helper-quad-main (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-runtime-hash))
        import-sec (emit-import-section-helper-quad (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-runtime-hash))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
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
        .expect("generalized alloc+print+read-file+__fnv1a_hash quad program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).unwrap_or_default();
    assert!(
        data_section
            .windows("fixture.txt".len())
            .any(|window| window == b"fixture.txt"),
        "generalized hash quad でも read-file path literal bytes は data section に配置されること"
    );
    assert_eq!(
        run_exported_i64_with_alloc_print_read_hash_imports(&modules[0], "_start", "aa").0,
        2,
        "generalized alloc+print+read-file+__fnv1a_hash quad を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が同じ generalized alloc+print+read-file+__fnv1a_hash source から同一 stage2 Wasm を 2 回生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_identical_hash_helper_quad_stage2_wasm_for_same_source() {
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
        type-sec (emit-type-section-helper-quad-main (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-runtime-hash))
        import-sec (emit-import-section-helper-quad (helper-id-alloc) (helper-id-print) (helper-id-read-file) (helper-id-runtime-hash))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 4 0)
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
        .expect("same hash-helper quad source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ generalized hash helper quad source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).unwrap_or_default();
    assert!(
        data_section
            .windows("fixture.txt".len())
            .any(|window| window == b"fixture.txt"),
        "repeatability でも hash quad の read-file path literal bytes は data section に配置されること"
    );
    assert_eq!(
        run_exported_i64_with_alloc_print_read_hash_imports(&modules[0], "_start", "aa").0,
        2
    );
}

/// BOOT-04: stage1 が alloc+print import を伴う print program を stage2 Wasm として生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_print_program() {
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
        import-sec (emit-import-section-alloc-print)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 2)
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
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (do (print 42) (print 7) 0))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("print program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "print program を含む stage2 Wasm は import section を持つこと"
    );
    let (result, printed) = run_exported_i64_with_alloc_print_imports(&modules[0], "_start");
    assert_eq!(result, 0, "print program を含む stage2 Wasm の戻り値が不正");
    assert_eq!(printed, "42\n7\n", "stage2 print output が不正");
}

/// BOOT-04: stage1 が generalized 2-helper pair で alloc+print stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_generalized_alloc_print_helper_pair() {
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
        type-sec (emit-type-section-helper-pair-main (helper-id-alloc) (helper-id-print))
        import-sec (emit-import-section-helper-pair (helper-id-alloc) (helper-id-print))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 2)
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
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (do (print 42) (print 7) 0))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("generalized alloc+print pair program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let (result, printed) = run_exported_i64_with_alloc_print_imports(&modules[0], "_start");
    assert_eq!(
        result, 0,
        "generalized alloc+print pair stage2 Wasm の戻り値が不正"
    );
    assert_eq!(
        printed, "42\n7\n",
        "generalized alloc+print pair stage2 print output が不正"
    );
}

/// BOOT-04: stage1 が同じ generalized alloc+print pair source から同一 stage2 Wasm を 2 回生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_identical_alloc_print_pair_stage2_wasm_for_same_source() {
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
        type-sec (emit-type-section-helper-pair-main (helper-id-alloc) (helper-id-print))
        import-sec (emit-import-section-helper-pair (helper-id-alloc) (helper-id-print))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-index 2)
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
  (let [src "(defn main [] (do (print 42) (print 7) 0))"
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
        .expect("same generalized alloc+print pair source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ generalized alloc+print pair source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let (result, printed) = run_exported_i64_with_alloc_print_imports(&modules[0], "_start");
    assert_eq!(result, 0);
    assert_eq!(printed, "42\n7\n");
}

/// BOOT-04: stage1 が alloc+print+read-file import を伴う read-file program を stage2 Wasm として生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_read_file_program() {
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
        import-sec (emit-import-section-alloc-print-read)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 3 0)
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
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (string-length (read-file 0)))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("read-file program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "read-file program を含む stage2 Wasm は import section を持つこと"
    );
    let (result, printed) =
        run_exported_i64_with_alloc_print_read_imports(&modules[0], "_start", "hello from file");
    assert_eq!(
        result, 15,
        "read-file program を含む stage2 Wasm の戻り値が不正"
    );
    assert!(
        printed.is_empty(),
        "read-file slice では print output は不要"
    );
}

/// BOOT-04: stage1 が generalized 3-helper triple で alloc+print+read-file stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_generalized_alloc_print_read_helper_triple() {
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
        type-sec (emit-type-section-helper-triple-main (helper-id-alloc) (helper-id-print) (helper-id-read-file))
        import-sec (emit-import-section-helper-triple (helper-id-alloc) (helper-id-print) (helper-id-read-file))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 3 0)
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
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (string-length (read-file 0)))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("generalized alloc+print+read-file triple program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let (result, printed) =
        run_exported_i64_with_alloc_print_read_imports(&modules[0], "_start", "hello from file");
    assert_eq!(
        result, 15,
        "generalized alloc+print+read-file triple stage2 Wasm の戻り値が不正"
    );
    assert!(
        printed.is_empty(),
        "generalized alloc+print+read-file triple slice では print output は不要"
    );
}

/// BOOT-04: stage1 が同じ generalized alloc+print+read-file triple source から同一 stage2 Wasm を 2 回生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_identical_read_helper_triple_stage2_wasm_for_same_source() {
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
        type-sec (emit-type-section-helper-triple-main (helper-id-alloc) (helper-id-print) (helper-id-read-file))
        import-sec (emit-import-section-helper-triple (helper-id-alloc) (helper-id-print) (helper-id-read-file))
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 3 0)
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
  (let [src "(defn main [] (string-length (read-file 0)))"
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
        .expect("same generalized read-helper triple source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ generalized alloc+print+read-file triple source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let (result, printed) =
        run_exported_i64_with_alloc_print_read_imports(&modules[0], "_start", "hello from file");
    assert_eq!(result, 15);
    assert!(printed.is_empty());
}

/// BOOT-04: stage1 が同じ read-file helper source から同一 stage2 Wasm を 2 回生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_identical_read_helper_stage2_wasm_for_same_source() {
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
        import-sec (emit-import-section-alloc-print-read)
        function-sec (emit-function-section-main-type-index 2)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 3 0)
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
  (let [src "(defn main [] (string-length (read-file 0)))"
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
        .expect("same read-helper source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ read-file helper source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    let (result, printed) =
        run_exported_i64_with_alloc_print_read_imports(&modules[0], "_start", "hello from file");
    assert_eq!(result, 15);
    assert!(printed.is_empty());
}
