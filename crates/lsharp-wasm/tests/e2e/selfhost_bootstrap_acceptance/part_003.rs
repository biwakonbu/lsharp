/// BOOT-04 受入: stage1 が生成する stage2 Wasm のセクション構造が安定していること。
///
/// 同一入力に対して 2 回 stage1 を実行し、生成された stage2 の全セクションが
/// byte-identical であることを確認する。
/// また、必須セクション (type / function / export / code) が必ず存在することを確認する。
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_section_stability() {
    // alloc import を使うプログラムで section stability を検証 (より多くのセクションを持つ)
    let stage2_src = "(defn main [] (vector-length (vector-new 3)))";
    let harness = build_alloc_bootstrap_harness_double(stage2_src);
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("stage1 section stability: WASI 実行失敗");
    let modules = parse_emitted_wasm_modules(&output, 2);
    assert_eq!(modules.len(), 2, "stage2 モジュール数が不正");

    let stage2_a = &modules[0];
    let stage2_b = &modules[1];
    assert_valid_wasm(stage2_a);
    assert_valid_wasm(stage2_b);

    // セクション構造が byte-identical であることを確認 (section stability)
    let sections_a = extract_sections(stage2_a);
    let sections_b = extract_sections(stage2_b);

    assert_eq!(
        sections_a, sections_b,
        "BOOT-04 section stability 失敗: セクション構造が 2 回の stage1 実行で一致しない\n\
         run1 sections: {:?}\n\
         run2 sections: {:?}",
        sections_a, sections_b
    );

    // 必須セクションが存在することを確認
    let section_ids: Vec<u8> = sections_a.iter().map(|(id, _)| *id).collect();

    // type section (id=1) は必須
    assert!(
        section_ids.contains(&1),
        "BOOT-04 section stability: type section (id=1) が存在しない\n\
         実際のセクション: {:?}",
        sections_a
    );
    // function section (id=3) は必須
    assert!(
        section_ids.contains(&3),
        "BOOT-04 section stability: function section (id=3) が存在しない"
    );
    // export section (id=7) は必須
    assert!(
        section_ids.contains(&7),
        "BOOT-04 section stability: export section (id=7) が存在しない"
    );
    // code section (id=10) は必須
    assert!(
        section_ids.contains(&10),
        "BOOT-04 section stability: code section (id=10) が存在しない"
    );

    // 各セクションのバイト列が一致することを確認 (全セクションのバイト一致)
    assert_eq!(
        stage2_a,
        stage2_b,
        "BOOT-04 section stability: stage2 bytes が全体として一致しない\n\
         run1: {} bytes, run2: {} bytes",
        stage2_a.len(),
        stage2_b.len()
    );

    eprintln!(
        "BOOT-04 section stability: {} セクション, {} bytes (stable)",
        sections_a.len(),
        stage2_a.len()
    );
}

// =============================================================================
// Test 4: test_e2e_bootstrap_stage1_symbol_stability
// =============================================================================

/// BOOT-04 受入: stage1 が生成する stage2 Wasm の export シンボルが安定していること。
///
/// 同一入力に対して 2 回 stage1 を実行し、生成された stage2 の export section が
/// byte-identical であることを確認する。
/// また、`_start` シンボルが必ず export されていることを確認する。
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_symbol_stability() {
    // 2 種類のプログラムでシンボル安定性を検証
    let test_cases: &[&str] = &[
        // シンプルな pure 整数プログラム (import なし)
        "(defn helper [x] (+ x 1)) (defn main [] (helper 99))",
        // alloc を使うプログラム (import section あり)
        "(defn main [] (vector-length (vector-push (vector-new 2) 42)))",
    ];

    for src in test_cases {
        let harness = if src.contains("vector-") {
            build_alloc_bootstrap_harness_double(src)
        } else {
            // simple 版を 2 回出力するよう main を書き換え
            let escaped = src.replace('"', "\\\"");
            format!(
                r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count) dst
    (bootstrap-append-bytes (vector-push dst (vector-get src idx)) src (+ idx 1) count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair    (compile-program-functions program)
        functions   (vector-get pair 1)
        func-count  (vector-length functions)
        header      (emit-header)
        type-sec    (emit-type-section-functions functions)
        func-sec    (emit-function-section-functions functions)
        export-sec  (emit-export-section-main-index (- func-count 1))
        code-sec    (emit-code-section-functions functions)
        b0 (bootstrap-append-bytes (vector-new 64) header   0 (vector-length header))
        b1 (bootstrap-append-bytes b0 type-sec   0 (vector-length type-sec))
        b2 (bootstrap-append-bytes b1 func-sec   0 (vector-length func-sec))
        b3 (bootstrap-append-bytes b2 export-sec 0 (vector-length export-sec))]
    (bootstrap-append-bytes b3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-bytes [bytes idx count]
  (if (>= idx count) 0
    (do (print (vector-get bytes idx)) (bootstrap-print-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do (print count) (bootstrap-print-bytes bytes 0 count) 0)))

(defn main []
  (let [s "{}"
        s2-a (bootstrap-build-stage2 s)
        s2-b (bootstrap-build-stage2 s)]
    (do (bootstrap-print-module s2-a) (bootstrap-print-module s2-b) 0)))
"#,
                escaped
            )
        };

        let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
        let stage1_wasm = compile_only(&stage1_source);
        assert_valid_wasm(&stage1_wasm);

        let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
            .unwrap_or_else(|e| panic!("stage1 symbol stability 実行失敗 (src={src:?}): {e}"));
        let modules = parse_emitted_wasm_modules(&output, 2);
        assert_eq!(modules.len(), 2, "stage2 モジュール数が不正 (src={src:?})");

        let stage2_a = &modules[0];
        let stage2_b = &modules[1];
        assert_valid_wasm(stage2_a);
        assert_valid_wasm(stage2_b);

        // export section が byte-identical であること (symbol stability)
        let export_a = extract_section_bytes(stage2_a, 7);
        let export_b = extract_section_bytes(stage2_b, 7);

        assert_eq!(
            export_a, export_b,
            "BOOT-04 symbol stability 失敗: export section が 2 回の stage1 実行で一致しない\n\
             src={src:?}"
        );

        // `_start` シンボルが export section に存在すること
        let export_bytes = export_a.expect("export section が存在しない");
        let start_sym = b"_start";
        assert!(
            export_bytes
                .windows(start_sym.len())
                .any(|w| w == start_sym),
            "BOOT-04 symbol stability: '_start' シンボルが export section に存在しない\n\
             src={src:?}\n\
             export section bytes ({} bytes): {:?}",
            export_bytes.len(),
            export_bytes
        );

        eprintln!(
            "BOOT-04 symbol stability: src={src:?}, export section {} bytes (stable, '_start' 確認済み)",
            export_bytes.len()
        );
    }
}

// =============================================================================
// Test 5: test_e2e_wasi_start_signature  (BOOT-04: _start WASI シグネチャ修正)
// =============================================================================

/// stage2 生成に WASI 互換 _start wrapper を使う bootstrap ハーネス。
///
/// 通常関数を `() -> i64` で保持しつつ、`_start: () -> ()` のラッパー関数を
/// 追加する3つの新関数 (`emit-type-section-functions-wasi`,
/// `emit-function-section-functions-wasi`, `emit-code-section-functions-wasi`)
/// を呼び出す。
fn build_wasi_start_bootstrap_harness(stage2_src: &str) -> String {
    format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src (+ idx 1) count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair    (compile-program-functions program)
        functions   (vector-get pair 1)
        func-count  (vector-length functions)
        header      (emit-header)
        type-sec    (emit-type-section-functions-wasi functions)
        func-sec    (emit-function-section-functions-wasi functions)
        export-sec  (emit-export-section-main-index func-count)
        code-sec    (emit-code-section-functions-wasi functions)
        b0 (bootstrap-append-bytes (vector-new 64) header    0 (vector-length header))
        b1 (bootstrap-append-bytes b0 type-sec    0 (vector-length type-sec))
        b2 (bootstrap-append-bytes b1 func-sec    0 (vector-length func-sec))
        b3 (bootstrap-append-bytes b2 export-sec  0 (vector-length export-sec))]
    (bootstrap-append-bytes b3 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-bytes [bytes idx count]
  (if (>= idx count) 0
    (do (print (vector-get bytes idx))
        (bootstrap-print-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do (print count) (bootstrap-print-bytes bytes 0 count) 0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "{}")]
    (do (bootstrap-print-module stage2) 0)))
"#,
        stage2_src.replace('"', "\\\"")
    )
}

/// type section バイト列に `() -> ()` 型エントリが含まれるか検証する。
///
/// `() -> ()` のバイトパターンは `[0x60, 0x00, 0x00]`
/// (functype マーカー, 0 params, 0 returns)。
/// 通常関数型 `() -> i64` は `[0x60, 0x00, 0x01, 0x7E]` で第3バイトが `0x01` なので
/// 誤検出しない。
fn type_section_has_void_void(type_section: &[u8]) -> bool {
    let pattern = [0x60u8, 0x00, 0x00];
    type_section.windows(3).any(|w| w == pattern)
}

/// BOOT-04: stage2 の `_start` が WASI 互換型 `() -> ()` で生成されること。
///
/// selfhost WasmEmit.ls の `emit-type-section-functions-wasi`,
/// `emit-function-section-functions-wasi`, `emit-code-section-functions-wasi`
/// が正しく機能し、生成された stage2 Wasm を wasmtime WASI ランタイムで
/// 直接実行できることを検証する。
#[test]
#[ignore]
fn test_e2e_wasi_start_signature() {
    let cases: &[&str] = &[
        "(defn main [] 42)",
        "(defn double [x] (* x 2)) (defn main [] (double 21))",
    ];

    for src in cases {
        let harness = build_wasi_start_bootstrap_harness(src);
        let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
        let stage1_wasm = compile_only(&stage1_source);
        assert_valid_wasm(&stage1_wasm);

        let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
            .unwrap_or_else(|e| panic!("stage1 wasi_start 実行失敗 (src={src:?}): {e}"));
        let modules = parse_emitted_wasm_modules(&output, 1);
        let stage2 = &modules[0];
        assert_valid_wasm(stage2);

        // type section が () -> () 型を含むこと (WASI _start 用ラッパー型)
        let type_bytes =
            extract_section_bytes(stage2, 1).expect("stage2 type section が存在しない");
        assert!(
            type_section_has_void_void(&type_bytes),
            "BOOT-04 wasi_start: type section に () -> () 型が存在しない\n\
             src={src:?}\n\
             type section bytes ({} bytes): {:?}",
            type_bytes.len(),
            type_bytes
        );

        // stage2 を wasmtime WASI ランタイムで直接実行できること
        let wasi_result = lsharp_wasm::wasi_runner::run_wasm_wasi(stage2);
        assert!(
            wasi_result.is_ok(),
            "BOOT-04 wasi_start: stage2 の WASI 実行に失敗\n\
             src={src:?}\n\
             エラー: {:?}",
            wasi_result.err()
        );

        eprintln!(
            "BOOT-04 wasi_start: src={src:?}, stage2 {} bytes, _start: () -> () 確認済み, WASI 実行 OK",
            stage2.len()
        );
    }
}
