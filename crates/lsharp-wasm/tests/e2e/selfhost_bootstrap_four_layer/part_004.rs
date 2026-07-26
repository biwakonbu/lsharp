
/// BOOT-04: stage1 が 5 式以上の do に含まれる source-aware string literal も stage2 Wasm に落とし込めること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_extended_do_string_literal_data_section() {
    let stage2_source = r#"(defn main [] (do "ab" "c" "de" "fgh" "ijk"))"#.replace('"', "\\\"");
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
        .expect("extended do string literal program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).expect("data section が見つからない");
    let expected_data = selfhost_string_object_sequence(&["ab", "c", "de", "fgh", "ijk"]);
    assert!(
        data_section
            .windows(expected_data.len())
            .any(|window| window == expected_data),
        "extended do string literal objects が data section に連結配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        selfhost_string_object_offset(1024, &["ab", "c", "de", "fgh"]),
        "extended do string literal の最終 offset が前段 object header + bytes を考慮していない"
    );
}

/// BOOT-04: stage1 が if branch 内の source-aware string literal を stage2 Wasm に落とし込めること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_if_string_literal_data_section() {
    let stage2_source = r#"(defn main [] (if (= 1 1) "hello" "world"))"#.replace('"', "\\\"");
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
        .expect("if string literal program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).expect("data section が見つからない");
    let expected_data = selfhost_string_object_sequence(&["hello", "world"]);
    assert!(
        data_section
            .windows(expected_data.len())
            .any(|window| window == expected_data),
        "if string literal objects が data section に連結配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        1024,
        "if string literal の then branch offset が不正"
    );
}

/// BOOT-04: stage1 が match arm 内の source-aware string literal を stage2 Wasm に落とし込めること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_match_string_literal_data_section() {
    let stage2_source = r#"(defn main [] (match 2 [1 "one"] [2 "two"]))"#.replace('"', "\\\"");
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
        .expect("match string literal program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).expect("data section が見つからない");
    let expected_data = selfhost_string_object_sequence(&["one", "two"]);
    assert!(
        data_section
            .windows(expected_data.len())
            .any(|window| window == expected_data),
        "match string literal objects が data section に連結配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        selfhost_string_object_offset(1024, &["one"]),
        "match string literal の selected branch offset が前段 object header + bytes を考慮していない"
    );
}

/// BOOT-04: stage1 が lambda body 内の source-aware string literal を stage2 Wasm に落とし込めること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_lambda_string_literal_data_section() {
    let stage2_source = r#"(defn main [] (fn [x] "ok"))"#.replace('"', "\\\"");
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
        .expect("lambda string literal program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).expect("data section が見つからない");
    let expected_data = selfhost_string_object_bytes("ok");
    assert!(
        data_section
            .windows(expected_data.len())
            .any(|window| window == expected_data),
        "lambda string literal object が data section に配置されていない"
    );
    assert_eq!(
        run_exported_i64(&modules[0], "_start"),
        1024,
        "lambda string literal の offset が不正"
    );
}

/// BOOT-04: stage1 が vector-length builtin を含む stage2 Wasm を valid module として生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_vector_length_helper_program() {
    let stage2_src = r#"(defn vlen [v] (vector-length v)) (defn main [] 0)"#;
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
        .expect("vector-length helper program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 5),
        "vector-length helper を含む stage2 Wasm は memory section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        0,
        "helper 未使用でも vector-length builtin を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が vector-get builtin を含む stage2 Wasm を valid module として生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_vector_get_helper_program() {
    let stage2_src = r#"(defn vget0 [v] (vector-get v 0)) (defn main [] 0)"#;
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
        .expect("vector-get helper program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "stage2 は selfhost ランタイム import section を持つこと"
    );
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 5),
        "vector-get helper を含む stage2 Wasm は memory section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        0,
        "helper 未使用でも vector-get builtin を含む stage2 Wasm が実行可能であること"
    );
}

/// BOOT-04: stage1 が __alloc import を伴う vector-new program を stage2 Wasm として生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_vector_new_program() {
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
        type-sec (emit-type-section-alloc-main)
        import-sec (emit-import-section-alloc)
        function-sec (emit-function-section-main-type-index 1)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 1 0)
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
  (let [stage2 (bootstrap-build-stage2 "(defn main [] (vector-length (vector-new 4)))")]
    (do
      (bootstrap-print-module stage2)
      0)))
"#;
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("vector-new program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "vector-new program を含む stage2 Wasm は alloc import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_alloc_import(&modules[0], "_start"),
        0,
        "vector-new + vector-length を含む stage2 Wasm が alloc import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が同じ alloc-import tiny source から同一 stage2 Wasm を 2 回生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_identical_alloc_stage2_wasm_for_same_source() {
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
        type-sec (emit-type-section-alloc-main)
        import-sec (emit-import-section-alloc)
        function-sec (emit-function-section-main-type-index 1)
        memory-sec (emit-memory-section)
        export-sec (emit-export-section-main-memory-index 1 0)
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
  (let [src "(defn main [] (vector-length (vector-new 4)))"
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
        .expect("same alloc-source stage1 実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");
    assert_eq!(
        modules[0], modules[1],
        "同じ alloc-import tiny source から stage2 Wasm が非決定的に変化した"
    );
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "repeatability 対象 stage2 Wasm は alloc import section を持つこと"
    );
    assert_eq!(run_exported_i64_with_alloc_import(&modules[0], "_start"), 0);
}

/// BOOT-04: stage1 が vector-push の in-place + growth を含む stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_vector_push_program() {
    let stage1_source = stage1_source_emitting_wasi_stage2(
        "(defn main [] (let [v0 (vector-new 1)] (let [v1 (vector-push v0 10)] (vector-length (vector-push v1 20)))))",
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("vector-push program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "vector-push program を含む stage2 Wasm は selfhost runtime import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        2,
        "vector-push の in-place + growth を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が ref-new/ref-set/ref-get を含む stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_ref_program() {
    let stage1_source = stage1_source_emitting_wasi_stage2(
        "(defn main [] (let [r (ref-new 1)] (do (ref-set r 42) (ref-get r))))",
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("ref program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "ref program を含む stage2 Wasm は selfhost runtime import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        42,
        "ref-new/ref-set/ref-get を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が整数 key の map-new/map-insert/map-get/map-size を含む stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_map_program() {
    let stage1_source = stage1_source_emitting_wasi_stage2(
        "(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 1 10)] (let [m2 (map-insert m1 2 20)] (+ (+ (map-get m2 1) (map-get m2 2)) (map-size m2))))))",
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("map program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "map program を含む stage2 Wasm は selfhost runtime import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        32,
        "整数 key の map builtins を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が整数 key subset の map-contains? を含む stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_map_contains_program() {
    let stage1_source = stage1_source_emitting_wasi_stage2(
        "(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 7 70)] (+ (* 10 (map-contains? m1 7)) (map-contains? m1 99)))))",
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("map-contains? program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "map-contains? program を含む stage2 Wasm は selfhost runtime import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        10,
        "整数 key subset の map-contains? を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が整数 key subset の map-remove を含む stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_map_remove_program() {
    let stage1_source = stage1_source_emitting_wasi_stage2(
        "(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 1 10)] (let [m2 (map-insert m1 2 20)] (let [m3 (map-remove m2 1)] (+ (map-get m3 1) (+ (* 10 (map-size m3)) (map-get m3 2))))))))",
    );
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("map-remove program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    assert!(
        extract_sections(&modules[0]).iter().any(|(id, _)| *id == 2),
        "map-remove program を含む stage2 Wasm は selfhost runtime import section を持つこと"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        30,
        "整数 key subset の map-remove を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}

/// BOOT-04: stage1 が source-aware string key subset の map builtins を含む stage2 Wasm を生成できること
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_stage2_wasm_for_string_key_map_program() {
    let stage2_source = r#"(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 "aa" 10)] (let [m2 (map-insert m1 "bb" 20)] (let [m3 (map-remove m2 "aa")] (+ (* 10 (map-size m3)) (map-get m3 "bb")))))))"#.replace('"', "\\\"");
    let stage1_source = stage1_source_emitting_wasi_stage2_with_source(&stage2_source);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("string key map program を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    let data_section = extract_section_bytes(&modules[0], 11).unwrap_or_default();
    assert!(
        !data_section
            .windows(2)
            .any(|window| window == [97, 97] || window == [98, 98]),
        "string key literal bytes は data section に残らず hash const 化されること"
    );
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        30,
        "string key subset の map builtins を含む stage2 Wasm が runtime 10-import 付きで実行可能であること"
    );
}
