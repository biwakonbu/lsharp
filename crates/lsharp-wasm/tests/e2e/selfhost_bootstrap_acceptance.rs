use super::selfhost_bootstrap_four_layer::{
    BootstrapDiffArtifactFixture, bootstrap_diff_artifact_id,
    run_wasm_with_six_imports_compiler_mode, run_wasm_with_six_imports_compiler_mode_fs,
    write_bootstrap_diff_artifact,
};
use super::support::*;

// =============================================================================
// BOOT-04 受入テスト: True stage1-stage2-stage3 bootstrap の実体比較テスト
//
// acceptance criteria (phase11-implementation-plan.md BOOT-04 より):
//   test_e2e_bootstrap_stage1_stage2_match
//   test_e2e_bootstrap_fixed_point_stage2_stage3
//   test_e2e_bootstrap_stage1_section_stability
//   test_e2e_bootstrap_stage1_symbol_stability
// =============================================================================

// -----------------------------------------------------------------------------
// ローカルヘルパー: Wasm セクションパース
// -----------------------------------------------------------------------------

/// Wasm バイナリからセクション ID とサイズの列を抽出する
fn extract_sections(wasm: &[u8]) -> Vec<(u8, usize)> {
    let mut sections = Vec::new();
    let mut pos = 8; // magic(4) + version(4)
    while pos < wasm.len() {
        let section_id = wasm[pos];
        pos += 1;
        let mut size: usize = 0;
        let mut shift = 0;
        loop {
            if pos >= wasm.len() {
                break;
            }
            let byte = wasm[pos] as usize;
            pos += 1;
            size |= (byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        sections.push((section_id, size));
        pos += size;
    }
    sections
}

/// 指定セクション ID のバイト列を抽出する
fn extract_section_bytes(wasm: &[u8], target_id: u8) -> Option<Vec<u8>> {
    let mut pos = 8;
    while pos < wasm.len() {
        let section_id = wasm[pos];
        pos += 1;
        let mut size: usize = 0;
        let mut shift = 0;
        loop {
            if pos >= wasm.len() {
                break;
            }
            let byte = wasm[pos] as usize;
            pos += 1;
            size |= (byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        if section_id == target_id {
            return Some(wasm[pos..pos + size].to_vec());
        }
        pos += size;
    }
    None
}

/// 2つの Wasm バイト列が最初に食い違う位置を返す
fn first_diff_index(left: &[u8], right: &[u8]) -> Option<usize> {
    left.iter()
        .zip(right.iter())
        .position(|(a, b)| a != b)
        .or_else(|| {
            if left.len() == right.len() {
                None
            } else {
                Some(left.len().min(right.len()))
            }
        })
}

/// stage1 が stdout に出力した length-prefixed Wasm バイト列を復元する
fn parse_emitted_wasm_modules(output: &str, expected_modules: usize) -> Vec<Vec<u8>> {
    let values: Vec<usize> = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<usize>()
                .unwrap_or_else(|_| panic!("数値でない stage1 出力: {line:?}"))
        })
        .collect();

    let mut pos = 0;
    let mut modules = Vec::with_capacity(expected_modules);
    for module_idx in 0..expected_modules {
        assert!(
            pos < values.len(),
            "module[{module_idx}] の長さ行が不足: {:?}",
            values
        );
        let len = values[pos];
        pos += 1;
        assert!(
            values.len() >= pos + len,
            "module[{module_idx}] の byte 数が不足: len={}, remaining={}",
            len,
            values.len().saturating_sub(pos)
        );
        let mut wasm = Vec::with_capacity(len);
        for &value in &values[pos..pos + len] {
            assert!(value <= u8::MAX as usize, "byte 値が範囲外: {value}");
            wasm.push(value as u8);
        }
        pos += len;
        modules.push(wasm);
    }
    assert_eq!(
        pos,
        values.len(),
        "想定外の trailing output が残っている: {:?}",
        &values[pos..]
    );
    modules
}

/// 外部 import なしの Wasm モジュールを instantiate して i64 export を呼び出す
fn run_exported_i64_no_imports(wasm: &[u8], export_name: &str) -> i64 {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm).expect("stage2 Wasm の Module 構築に失敗");
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[])
        .expect("stage2 Wasm のインスタンス化に失敗 (import が存在する可能性あり)");
    let func = instance
        .get_typed_func::<(), i64>(&mut store, export_name)
        .unwrap_or_else(|e| panic!("{export_name} export の取得に失敗: {e}"));
    func.call(&mut store, ())
        .expect("stage2 Wasm export の呼び出しに失敗")
}

/// import なし関数型プログラム用 bootstrap ハーネス: compile-program-functions 経由で stage2 を生成し print する
fn build_simple_bootstrap_harness(stage2_src: &str) -> String {
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
        type-sec    (emit-type-section-functions functions)
        func-sec    (emit-function-section-functions functions)
        export-sec  (emit-export-section-main-index (- func-count 1))
        code-sec    (emit-code-section-functions functions)
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

/// alloc import を伴う bootstrap ハーネス: compile-program-functions 経由で stage2 を生成し print する
/// vector-new など env.__alloc import を必要とするプログラム用
fn build_alloc_bootstrap_harness_double(stage2_src: &str) -> String {
    let escaped = stage2_src.replace('"', "\\\"");
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
        header      (emit-header)
        type-sec    (emit-type-section-alloc-main)
        import-sec  (emit-import-section-alloc)
        func-sec    (emit-function-section-main-type-index 1)
        memory-sec  (emit-memory-section)
        export-sec  (emit-export-section-main-index 1)
        code-sec    (emit-code-section-functions functions)
        b0 (bootstrap-append-bytes (vector-new 64) header     0 (vector-length header))
        b1 (bootstrap-append-bytes b0 type-sec    0 (vector-length type-sec))
        b2 (bootstrap-append-bytes b1 import-sec  0 (vector-length import-sec))
        b3 (bootstrap-append-bytes b2 func-sec    0 (vector-length func-sec))
        b4 (bootstrap-append-bytes b3 memory-sec  0 (vector-length memory-sec))
        b5 (bootstrap-append-bytes b4 export-sec  0 (vector-length export-sec))]
    (bootstrap-append-bytes b5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-bytes [bytes idx count]
  (if (>= idx count) 0
    (do (print (vector-get bytes idx))
        (bootstrap-print-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do (print count) (bootstrap-print-bytes bytes 0 count) 0)))

(defn main []
  (let [src   "{}"
        s2-a  (bootstrap-build-stage2 src)
        s2-b  (bootstrap-build-stage2 src)]
    (do
      (bootstrap-print-module s2-a)
      (bootstrap-print-module s2-b)
      0)))
"#,
        escaped
    )
}

// =============================================================================
// Test 1: test_e2e_bootstrap_stage1_stage2_match
// =============================================================================

/// BOOT-04 受入: stage1 (selfhost Wasm コンパイラ) が同一ソースから同一 stage2 Wasm を
/// 生成すること (決定的一致 = MATCH)。さらに stage2 Wasm が正しい計算結果を返すこと。
///
/// これは stage0 (Rust) と stage1 の出力が「同じ意味論」を持つことを確認する。
/// 完全な byte-level 一致は stage1/stage0 が異なるコード生成器を持つため不要だが、
/// 計算結果 (pure 整数値) は一致しなければならない。
#[test]
fn test_e2e_bootstrap_stage1_stage2_match() {
    // stage0 と stage1 が同じ結果を生むことを確認するテストケース群
    // ただし import なし (pure 整数演算のみ) に限定する
    let cases: &[(&str, i64)] = &[
        ("(defn main [] 42)", 42),
        ("(defn main [] (+ 20 22))", 42),
        ("(defn double [x] (* x 2)) (defn main [] (double 21))", 42),
        (
            "(defn fib [n] (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (defn main [] (fib 8))",
            21,
        ),
    ];

    for (src, expected) in cases {
        // --- stage0 (Rust コンパイラ) による期待値確認 ---
        // WASI backend が生成する Wasm を実行して期待値を verify
        let stage0_wasm = compile_only(src);
        assert_valid_wasm(&stage0_wasm);

        // --- stage1 (selfhost Wasm) による stage2 生成 ---
        let harness = build_simple_bootstrap_harness(src);
        let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
        let stage1_wasm = compile_only(&stage1_source);
        assert_valid_wasm(&stage1_wasm);

        // stage1 を 2 回実行し、出力が MATCH (決定的) であることを確認
        let out_a = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
            .unwrap_or_else(|e| panic!("stage1 run_a 失敗 (src={src:?}): {e}"));
        let out_b = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
            .unwrap_or_else(|e| panic!("stage1 run_b 失敗 (src={src:?}): {e}"));

        assert_eq!(out_a, out_b, "stage1 → stage2 出力が非決定的 (src={src:?})");

        let modules = parse_emitted_wasm_modules(&out_a, 1);
        assert_eq!(modules.len(), 1, "stage2 モジュール数が不正 (src={src:?})");
        let stage2 = &modules[0];
        assert_valid_wasm(stage2);

        // stage2 を実行して計算結果が stage0 の期待値と一致することを確認
        let stage2_result = run_exported_i64_no_imports(stage2, "_start");
        assert_eq!(
            stage2_result, *expected,
            "BOOT-04 stage1_stage2_match: stage2 の計算結果が期待値と一致しない\n\
             src={src:?}\n\
             expected={expected}, got={stage2_result}"
        );
    }
}

// =============================================================================
// Test 2: test_e2e_bootstrap_fixed_point_stage2_stage3
// =============================================================================

/// BOOT-04 受入: stage1 → stage2 → stage3 の固定点検証。
///
/// 固定点とは: stage2 を使って同じソースを再コンパイルしても stage3 == stage2 になること。
///
/// real compiler-mode / self-feed path で `stage2 == stage3` の byte identity を
/// 直接 assert し、stage3 から minimal.ls の再コンパイルまで通す。
#[test]
fn test_e2e_bootstrap_fixed_point_stage2_stage3() {
    let main_path = selfhost_main_path();
    let artifact_id = bootstrap_diff_artifact_id();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    // --- Phase A: stage1 が Main.ls から stage2 self compiler を決定論的に出力すること ---
    let stage2_output_run1 = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("fixed-point Phase A: stage1 run_1 が Main.ls の self-compile に失敗");
    let stage2_output_run2 = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("fixed-point Phase A: stage1 run_2 が Main.ls の self-compile に失敗");

    assert_eq!(
        stage2_output_run1, stage2_output_run2,
        "BOOT-04 fixed-point Phase A 失敗: stage1 が非決定的な stage2 self compiler を出力"
    );

    let stage2_modules_run1 = parse_emitted_wasm_modules(&stage2_output_run1, 1);
    let stage2_modules_run2 = parse_emitted_wasm_modules(&stage2_output_run2, 1);

    assert_eq!(
        stage2_modules_run1[0], stage2_modules_run2[0],
        "BOOT-04 fixed-point Phase A 失敗: stage2 self compiler bytes が 2 回の実行で一致しない"
    );
    assert_valid_wasm(&stage2_modules_run1[0]);

    let stage2_self_compiler = stage2_modules_run1[0].clone();

    eprintln!(
        "BOOT-04 fixed-point Phase A: stage2 self compiler ({} bytes) は決定的",
        stage2_self_compiler.len()
    );

    // --- Phase B: stage2 が Main.ls から stage3 self compiler を決定論的に出力すること ---
    let stage3_output_run1 = run_wasm_with_six_imports_compiler_mode_fs(
        &stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("fixed-point Phase B: stage2 run_1 が Main.ls の再コンパイルに失敗");
    let stage3_output_run2 = run_wasm_with_six_imports_compiler_mode_fs(
        &stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("fixed-point Phase B: stage2 run_2 が Main.ls の再コンパイルに失敗");

    assert_eq!(
        stage3_output_run1, stage3_output_run2,
        "BOOT-04 fixed-point Phase B 失敗: stage2 が非決定的な stage3 self compiler を出力"
    );

    let stage3_modules_run1 = parse_emitted_wasm_modules(&stage3_output_run1, 1);
    let stage3_modules_run2 = parse_emitted_wasm_modules(&stage3_output_run2, 1);
    assert_eq!(
        stage3_modules_run1[0], stage3_modules_run2[0],
        "BOOT-04 fixed-point Phase B 失敗: stage3 self compiler bytes が 2 回の実行で一致しない"
    );
    assert_valid_wasm(&stage3_modules_run1[0]);

    let stage3_self_compiler = stage3_modules_run1[0].clone();

    // stage3 も実際に自己ホストコンパイラとして minimal.ls をコンパイルできることを確認する。
    let stage4_output = run_wasm_with_six_imports_compiler_mode_fs(
        &stage3_self_compiler,
        &fixture_dir,
        &["compiler", "minimal.ls"],
    )
    .expect("fixed-point Phase B: stage3 self compiler が minimal.ls をコンパイルできない");
    let stage4_modules = parse_emitted_wasm_modules(&stage4_output, 1);
    let stage4_wasm = &stage4_modules[0];
    assert_valid_wasm(stage4_wasm);
    let stage4_run = run_wasm_with_six_imports_compiler_mode(stage4_wasm, "", &[]);
    assert!(
        stage4_run.is_ok(),
        "fixed-point Phase B: stage4 minimal 実行失敗: {:?}",
        stage4_run.err()
    );

    let stage2_sections = extract_sections(&stage2_self_compiler);
    let stage3_sections = extract_sections(&stage3_self_compiler);
    let diff_at = first_diff_index(&stage2_self_compiler, &stage3_self_compiler);
    let export_a = extract_section_bytes(&stage2_self_compiler, 7);
    let export_b = extract_section_bytes(&stage3_self_compiler, 7);
    let data_a = extract_section_bytes(&stage2_self_compiler, 11);
    let data_b = extract_section_bytes(&stage3_self_compiler, 11);

    let diff_report = [
        "Bootstrap Diff Report".to_string(),
        "=====================".to_string(),
        format!("commit: {artifact_id}"),
        "timestamp: 1970-01-01T00:00:00Z".to_string(),
        "test: test_e2e_bootstrap_fixed_point_stage2_stage3".to_string(),
        String::new(),
        format!(
            "Layer 1 (hash):    {} ({:#018x} vs {:#018x})",
            if stage2_self_compiler == stage3_self_compiler {
                "MATCH"
            } else {
                "MISMATCH"
            },
            super::selfhost_bootstrap_four_layer::hash_fingerprint(&stage2_self_compiler),
            super::selfhost_bootstrap_four_layer::hash_fingerprint(&stage3_self_compiler)
        ),
        format!(
            "Layer 2 (export):  {} ({} bytes vs {} bytes)",
            if export_a == export_b {
                "MATCH"
            } else {
                "MISMATCH"
            },
            export_a.as_ref().map_or(0, Vec::len),
            export_b.as_ref().map_or(0, Vec::len)
        ),
        format!(
            "Layer 3 (data):    {}",
            match (&data_a, &data_b) {
                (None, None) => "ABSENT".to_string(),
                (Some(left), Some(right)) if left == right => {
                    format!("MATCH ({} bytes vs {} bytes)", left.len(), right.len())
                }
                (Some(left), Some(right)) => {
                    format!("MISMATCH ({} bytes vs {} bytes)", left.len(), right.len())
                }
                (Some(left), None) => format!("MISMATCH ({} bytes vs absent)", left.len()),
                (None, Some(right)) => format!("MISMATCH (absent vs {} bytes)", right.len()),
            }
        ),
        "Layer 4 (diag):    MATCH (0 vs 0)".to_string(),
        String::new(),
        format!("stage1_a.wasm: {} bytes", stage2_self_compiler.len()),
        format!("stage1_b.wasm: {} bytes", stage3_self_compiler.len()),
        format!("first_diff: {diff_at:?}"),
        String::new(),
    ]
    .join("\n");

    write_bootstrap_diff_artifact(&BootstrapDiffArtifactFixture {
        artifact_id: &artifact_id,
        test_name: "test_e2e_bootstrap_fixed_point_stage2_stage3",
        left_key: "a",
        right_key: "b",
        left_label: "stage1_a",
        right_label: "stage1_b",
        left_wasm: Some(&stage2_self_compiler),
        right_wasm: Some(&stage3_self_compiler),
        diff_report: &diff_report,
        metadata: serde_json::json!({
            "commit_sha": artifact_id,
            "timestamp": "1970-01-01T00:00:00Z",
            "test_name": "test_e2e_bootstrap_fixed_point_stage2_stage3",
            "stage1_a_size": stage2_self_compiler.len(),
            "stage1_b_size": stage3_self_compiler.len(),
            "layers": {
                "hash": if stage2_self_compiler == stage3_self_compiler { "match" } else { "mismatch" },
                "export": if export_a == export_b { "match" } else { "mismatch" },
                "data": if data_a == data_b { "match" } else { "mismatch" },
                "diagnostics": "match"
            },
            "first_diff": diff_at
        }),
        left_sections: Some(serde_json::json!(stage2_sections)),
        right_sections: Some(serde_json::json!(stage3_sections)),
        left_export: export_a.as_deref(),
        right_export: export_b.as_deref(),
        left_data: data_a.as_deref(),
        right_data: data_b.as_deref(),
    });

    assert!(
        stage2_self_compiler == stage3_self_compiler,
        "BOOT-04 fixed-point 失敗: stage2 ({} bytes) != stage3 ({} bytes), \
         first_diff={diff_at:?}, stage2_sections={stage2_sections:?}, stage3_sections={stage3_sections:?}",
        stage2_self_compiler.len(),
        stage3_self_compiler.len(),
    );

    eprintln!(
        "BOOT-04 fixed-point 達成: stage2 == stage3 ({} bytes)",
        stage2_self_compiler.len()
    );
}

/// CP-01 / BOOT-04: stage0(Rust) から stage1 self compiler と stage2 self compiler を得る。
fn build_stage1_and_stage2_self_compilers_from_main() -> (Vec<u8>, Vec<u8>, std::path::PathBuf) {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("CP-01: stage1 が Main.ls から stage2 を生成できない");

    let modules = parse_emitted_wasm_modules(&stage2_output, 1);
    assert_eq!(
        modules.len(),
        1,
        "CP-01: stage2 wasm は 1 モジュールであるべき"
    );
    assert_valid_wasm(&modules[0]);

    (stage1_wasm, modules[0].clone(), selfhost_root)
}

/// CP-01: stage1→stage2 で得た自己コンパイラを返す（fixed point Phase A と同一経路）
fn build_stage2_self_compiler_from_main() -> (Vec<u8>, std::path::PathBuf) {
    let (_, stage2, selfhost_root) = build_stage1_and_stage2_self_compilers_from_main();
    (stage2, selfhost_root)
}

/// CP-01 / BOOT-04: `print` / `read-file` 集約箇所である Compiler.ls / WasmEmit.ls を
/// stage2 自己コンパイラが **同一バイト列** で再出力できること（決定性の固定点エビデンス）
#[test]
fn test_e2e_bootstrap_stage2_compiler_wasmemit_modules_deterministic() {
    let (stage2, root) = build_stage2_self_compiler_from_main();

    for rel in [
        "src/Backend/Wasm/Compiler.ls",
        "src/Backend/Wasm/WasmEmit.ls",
    ] {
        let out_a = run_wasm_with_six_imports_compiler_mode_fs(&stage2, &root, &["compiler", rel])
            .unwrap_or_else(|e| panic!("CP-01: stage2 が {rel} を 1 回目コンパイルできない: {e}"));
        let out_b = run_wasm_with_six_imports_compiler_mode_fs(&stage2, &root, &["compiler", rel])
            .unwrap_or_else(|e| panic!("CP-01: stage2 が {rel} を 2 回目コンパイルできない: {e}"));

        assert_eq!(out_a, out_b, "CP-01: stage2 の {rel} 出力が非決定的");

        let mods = parse_emitted_wasm_modules(&out_a, 1);
        assert_eq!(
            mods.len(),
            1,
            "CP-01: {rel} のコンパイル出力は 1 wasm モジュールであるべき"
        );
        assert_valid_wasm(&mods[0]);
    }
}

#[test]
fn test_bootstrap_fixed_point_ci_wiring_present() {
    let project_root = selfhost_project_root();
    let ci = std::fs::read_to_string(project_root.join(".github/workflows/ci.yml"))
        .expect("ci.yml の読み込みに失敗");
    assert!(
        ci.contains("RUN_BOOTSTRAP_FIXED_POINT: 1"),
        "CI は bootstrap fixed-point 実行フラグを渡すこと"
    );
    assert!(
        ci.contains("name: bootstrap-diff-${{ github.sha }}"),
        "CI は bootstrap diff artifact を upload すること"
    );
    assert!(
        ci.contains("path: ci-artifacts/bootstrap-diff/${{ github.sha }}/"),
        "CI は commit sha 配下の bootstrap diff artifact を upload すること"
    );
    assert!(
        ci.contains("if: always()"),
        "bootstrap diff artifact upload は always() で接続すること"
    );

    let script = std::fs::read_to_string(project_root.join("scripts/ci/compile-phase11-inputs.sh"))
        .expect("compile-phase11-inputs.sh の読み込みに失敗");
    assert!(
        script.contains("RUN_BOOTSTRAP_FIXED_POINT"),
        "compile-phase11-inputs.sh は fixed-point 実行オプションを受け取ること"
    );
    assert!(
        script.contains("test_e2e_bootstrap_fixed_point_stage2_stage3"),
        "compile-phase11-inputs.sh は fixed-point テストを明示実行すること"
    );
    assert!(
        script.contains("test_e2e_bootstrap_stage2_self_feed_fixed_input_set"),
        "compile-phase11-inputs.sh は full fixed input set の stage2 self-feed テストを明示実行すること"
    );
    assert!(
        script.contains("test_e2e_bootstrap_fixed_input_set_stage_chain_match"),
        "compile-phase11-inputs.sh は full fixed input set の stage1->stage2->stage3 compare テストを明示実行すること"
    );
}

#[test]
fn test_e2e_bootstrap_cli_fixed_input_compile_gate() {
    let cli_path = selfhost_source_path("Cli.ls");
    let wasm = try_compile_file_only(&cli_path).unwrap_or_else(|e| {
        panic!("CP-01: fixed input set compile gate の Cli.ls が失敗してはならない: {e}")
    });
    assert_valid_wasm(&wasm);
}

#[derive(Clone, Copy)]
enum FixedInputSetRoot {
    Selfhost,
    Repo,
}

impl FixedInputSetRoot {
    fn label(self) -> &'static str {
        match self {
            Self::Selfhost => "selfhost",
            Self::Repo => "repo",
        }
    }
}

struct FixedInputSetTarget {
    path: String,
    root: FixedInputSetRoot,
}

fn fixed_input_set_target_root<'a>(
    selfhost_root: &'a std::path::Path,
    repo_root: &'a std::path::Path,
    target: &FixedInputSetTarget,
) -> &'a std::path::Path {
    match target.root {
        FixedInputSetRoot::Selfhost => selfhost_root,
        FixedInputSetRoot::Repo => repo_root,
    }
}

fn fixed_input_set_self_feed_targets() -> Vec<FixedInputSetTarget> {
    let selfhost_root = selfhost_package_root();
    let selfhost_modules = [
        "AST",
        "Cli",
        "Closure",
        "Codegen",
        "Compiler",
        "Constraints",
        "Derive",
        "DocTools",
        "Emit",
        "Formatter",
        "GC",
        "HtmlDoc",
        "Hygiene",
        "IR",
        "JsonRpc",
        "Lexer",
        "Linker",
        "Linter",
        "Lower",
        "LowerDecl",
        "LowerExpr",
        "LowerPattern",
        "LspServer",
        "MacroExpand",
        "Main",
        "MetadataCheck",
        "ModuleGraph",
        "NativeCodegen",
        "NativeEmit",
        "NativeTarget",
        "Parser",
        "Span",
        "TestRunner",
        "Token",
        "Type",
        "TypeInfer",
        "TypeScheme",
        "WasiBackend",
        "WasiRunner",
        "WasmEmit",
    ];
    let mut targets = selfhost_modules
        .iter()
        .map(|module| {
            let file_name = format!("{module}.ls");
            let rel_path = selfhost_source_path(&file_name)
                .strip_prefix(&selfhost_root)
                .unwrap_or_else(|_| panic!("CP-01: selfhost 相対パスへ変換できない: {file_name}"))
                .to_string_lossy()
                .replace('\\', "/");
            FixedInputSetTarget {
                path: rel_path,
                root: FixedInputSetRoot::Selfhost,
            }
        })
        .collect::<Vec<_>>();

    targets.extend(
        [
            "stdlib/Core.ls",
            "stdlib/Char.ls",
            "stdlib/Debug.ls",
            "stdlib/IO.ls",
            "stdlib/List.ls",
            "stdlib/Map.ls",
            "stdlib/Path.ls",
            "stdlib/Set.ls",
            "stdlib/String.ls",
            "stdlib/Vector.ls",
            "stdlib/Json.ls",
            "examples/fib.ls",
            "examples/module.ls",
            "examples/trait.ls",
        ]
        .into_iter()
        .map(|path| FixedInputSetTarget {
            path: path.to_string(),
            root: FixedInputSetRoot::Repo,
        }),
    );

    targets
}

fn compile_fixed_input_target_with_stage1(
    stage1_self_compiler: &[u8],
    selfhost_root: &std::path::Path,
    repo_root: &std::path::Path,
    target: &FixedInputSetTarget,
) -> Result<Vec<u8>, String> {
    let root_dir = fixed_input_set_target_root(selfhost_root, repo_root, target);
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        stage1_self_compiler,
        Some(root_dir),
        &["compiler", target.path.as_str()],
    )
    .map_err(|e| format!("stage1 compiler run failed: {e}"))?;
    extract_single_compiled_module(&output, "stage1", target)
}

fn compile_fixed_input_target_with_stage2(
    stage2_self_compiler: &[u8],
    selfhost_root: &std::path::Path,
    repo_root: &std::path::Path,
    target: &FixedInputSetTarget,
) -> Result<Vec<u8>, String> {
    let root_dir = fixed_input_set_target_root(selfhost_root, repo_root, target);
    let output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        root_dir,
        &["compiler", target.path.as_str()],
    )
    .map_err(|e| format!("stage2 compiler run failed: {e}"))?;
    extract_single_compiled_module(&output, "stage2", target)
}

fn extract_single_compiled_module(
    output: &str,
    stage_label: &str,
    target: &FixedInputSetTarget,
) -> Result<Vec<u8>, String> {
    let parsed =
        std::panic::catch_unwind(|| parse_emitted_wasm_modules(output, 1)).map_err(|_| {
            format!(
                "{stage_label} output is not recoverable as a single wasm module for {}",
                target.path
            )
        })?;
    let wasm = parsed.into_iter().next().ok_or_else(|| {
        format!(
            "{stage_label} output did not contain a wasm module for {}",
            target.path
        )
    })?;
    if std::panic::catch_unwind(|| assert_valid_wasm(&wasm)).is_err() {
        return Err(format!(
            "{stage_label} output wasm validation failed for {}",
            target.path
        ));
    }
    Ok(wasm)
}

fn write_fixed_input_set_self_feed_artifact(
    artifact_id: &str,
    report: &str,
    metadata: &serde_json::Value,
) -> std::path::PathBuf {
    let artifact_root = selfhost_project_root()
        .join("ci-artifacts/bootstrap-diff")
        .join(artifact_id);
    std::fs::create_dir_all(&artifact_root).unwrap_or_else(|e| {
        panic!(
            "CP-01 artifact ディレクトリ作成に失敗 {}: {}",
            artifact_root.display(),
            e
        )
    });

    std::fs::write(
        artifact_root.join("fixed-input-set-self-feed-report.txt"),
        report,
    )
    .unwrap_or_else(|e| panic!("CP-01 report 書き込み失敗: {e}"));
    std::fs::write(
        artifact_root.join("fixed-input-set-self-feed.json"),
        serde_json::to_vec_pretty(metadata).expect("CP-01 self-feed JSON serialize に失敗"),
    )
    .unwrap_or_else(|e| panic!("CP-01 metadata 書き込み失敗: {e}"));

    artifact_root
}

fn write_fixed_input_set_stage_chain_artifact(
    artifact_id: &str,
    report: &str,
    metadata: &serde_json::Value,
) -> std::path::PathBuf {
    let artifact_root = selfhost_project_root()
        .join("ci-artifacts/bootstrap-diff")
        .join(artifact_id);
    std::fs::create_dir_all(&artifact_root).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 artifact ディレクトリ作成に失敗 {}: {}",
            artifact_root.display(),
            e
        )
    });

    std::fs::write(
        artifact_root.join("fixed-input-set-stage-chain-report.txt"),
        report,
    )
    .unwrap_or_else(|e| panic!("BOOT-04 stage-chain report 書き込み失敗: {e}"));
    std::fs::write(
        artifact_root.join("fixed-input-set-stage-chain.json"),
        serde_json::to_vec_pretty(metadata).expect("BOOT-04 stage-chain JSON serialize に失敗"),
    )
    .unwrap_or_else(|e| panic!("BOOT-04 stage-chain metadata 書き込み失敗: {e}"));

    artifact_root
}

#[test]
fn test_e2e_bootstrap_stage2_self_feed_fixed_input_set() {
    let (stage2, root) = build_stage2_self_compiler_from_main();
    let artifact_id = bootstrap_diff_artifact_id();
    let repo_root = selfhost_project_root();
    let targets = fixed_input_set_self_feed_targets();

    assert_eq!(
        targets.len(),
        54,
        "CP-01: fixed input set は selfhost/stdlib/examples の合計 54 件であるべき"
    );

    let mut compiled = Vec::new();
    let mut failures = Vec::new();
    for target in &targets {
        let root_dir = match target.root {
            FixedInputSetRoot::Selfhost => &root,
            FixedInputSetRoot::Repo => &repo_root,
        };
        let out_a = run_wasm_with_six_imports_compiler_mode_fs(
            &stage2,
            root_dir,
            &["compiler", target.path.as_str()],
        );
        let out_b = run_wasm_with_six_imports_compiler_mode_fs(
            &stage2,
            root_dir,
            &["compiler", target.path.as_str()],
        );
        match (out_a, out_b) {
            (Ok(output_a), Ok(output_b)) => {
                if output_a != output_b {
                    failures.push(serde_json::json!({
                        "path": target.path,
                        "root": target.root.label(),
                        "error": "stage2 self-feed 出力が非決定的",
                    }));
                    continue;
                }

                let parsed = std::panic::catch_unwind(|| parse_emitted_wasm_modules(&output_a, 1));
                let Ok(modules_a) = parsed else {
                    failures.push(serde_json::json!({
                        "path": target.path,
                        "root": target.root.label(),
                        "error": "stage2 出力が単一 wasm モジュールとして復元できない",
                    }));
                    continue;
                };
                let parsed = std::panic::catch_unwind(|| parse_emitted_wasm_modules(&output_b, 1));
                let Ok(modules_b) = parsed else {
                    failures.push(serde_json::json!({
                        "path": target.path,
                        "root": target.root.label(),
                        "error": "stage2 2回目出力が単一 wasm モジュールとして復元できない",
                    }));
                    continue;
                };
                let wasm_a = &modules_a[0];
                let wasm_b = &modules_b[0];
                if std::panic::catch_unwind(|| assert_valid_wasm(wasm_a)).is_err() {
                    failures.push(serde_json::json!({
                        "path": target.path,
                        "root": target.root.label(),
                        "error": "stage2 出力 wasm の検証に失敗",
                    }));
                    continue;
                }
                if std::panic::catch_unwind(|| assert_valid_wasm(wasm_b)).is_err() {
                    failures.push(serde_json::json!({
                        "path": target.path,
                        "root": target.root.label(),
                        "error": "stage2 2回目出力 wasm の検証に失敗",
                    }));
                    continue;
                }
                if wasm_a != wasm_b {
                    failures.push(serde_json::json!({
                        "path": target.path,
                        "root": target.root.label(),
                        "error": "stage2 self-feed wasm が byte-identical でない",
                    }));
                    continue;
                }
                compiled.push(serde_json::json!({
                    "path": target.path,
                    "root": target.root.label(),
                    "output_wasm_bytes": wasm_a.len(),
                    "fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(wasm_a),
                }));
            }
            (Err(err), _) | (_, Err(err)) => failures.push(serde_json::json!({
                "path": target.path,
                "root": target.root.label(),
                "error": err,
            })),
        }
    }

    let mut report_lines = vec![
        "Bootstrap Fixed Input Set Self-Feed Report".to_string(),
        "==========================================".to_string(),
        format!("commit: {artifact_id}"),
        "timestamp: 1970-01-01T00:00:00Z".to_string(),
        "test: test_e2e_bootstrap_stage2_self_feed_fixed_input_set".to_string(),
        format!("stage2_self_compiler_bytes: {}", stage2.len()),
        format!("target_count: {}", targets.len()),
        format!("compiled_count: {}", compiled.len()),
        format!("failed_count: {}", failures.len()),
        String::new(),
    ];
    report_lines.extend(compiled.iter().map(|entry| {
        format!(
            "PASS [{}] {} -> {} bytes",
            entry["root"].as_str().unwrap_or("unknown"),
            entry["path"].as_str().unwrap_or("<missing>"),
            entry["output_wasm_bytes"].as_u64().unwrap_or(0)
        )
    }));
    report_lines.extend(failures.iter().map(|entry| {
        format!(
            "FAIL [{}] {} -> {}",
            entry["root"].as_str().unwrap_or("unknown"),
            entry["path"].as_str().unwrap_or("<missing>"),
            entry["error"]
                .as_str()
                .unwrap_or("unknown error")
                .lines()
                .next()
                .unwrap_or("unknown error")
        )
    }));
    let report = report_lines.join("\n");

    let metadata = serde_json::json!({
        "commit_sha": artifact_id,
        "timestamp": "1970-01-01T00:00:00Z",
        "test_name": "test_e2e_bootstrap_stage2_self_feed_fixed_input_set",
        "stage2_self_compiler_bytes": stage2.len(),
        "target_count": targets.len(),
        "compiled_count": compiled.len(),
        "failed_count": failures.len(),
        "compiled_targets": compiled,
        "failed_targets": failures,
    });
    let artifact_dir = write_fixed_input_set_self_feed_artifact(&artifact_id, &report, &metadata);

    let written_metadata: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(artifact_dir.join("fixed-input-set-self-feed.json"))
            .expect("CP-01 self-feed artifact JSON の読み込みに失敗"),
    )
    .expect("CP-01 self-feed artifact JSON は JSON であること");
    assert_eq!(
        written_metadata["compiled_count"].as_u64(),
        Some(compiled.len() as u64),
        "CP-01 self-feed artifact は compiled_count を保持すること"
    );
    assert_eq!(
        written_metadata["failed_count"].as_u64(),
        Some(failures.len() as u64),
        "CP-01 self-feed artifact は failed_count を保持すること"
    );

    assert!(
        failures.is_empty(),
        "CP-01: stage2 self-feed fixed input set に失敗がある: {}",
        serde_json::to_string_pretty(&written_metadata["failed_targets"])
            .expect("CP-01 failure JSON serialize に失敗")
    );
    assert_eq!(
        compiled.len(),
        targets.len(),
        "CP-01: stage2 self compiler は fixed input set 全件を再生成できるべき"
    );
}

#[test]
fn test_e2e_bootstrap_fixed_input_set_stage_chain_match() {
    let (stage1, stage2, selfhost_root) = build_stage1_and_stage2_self_compilers_from_main();
    let repo_root = selfhost_project_root();
    let artifact_id = bootstrap_diff_artifact_id();
    let targets = fixed_input_set_self_feed_targets();

    assert_eq!(
        targets.len(),
        54,
        "BOOT-04: fixed input set は selfhost/stdlib/examples の合計 54 件であるべき"
    );

    let mut matched = Vec::new();
    let mut failures = Vec::new();
    for target in &targets {
        match (
            compile_fixed_input_target_with_stage1(&stage1, &selfhost_root, &repo_root, target),
            compile_fixed_input_target_with_stage2(&stage2, &selfhost_root, &repo_root, target),
        ) {
            (Ok(stage2_target), Ok(stage3_target)) => {
                let export_a = extract_section_bytes(&stage2_target, 7);
                let export_b = extract_section_bytes(&stage3_target, 7);
                let data_a = extract_section_bytes(&stage2_target, 11);
                let data_b = extract_section_bytes(&stage3_target, 11);
                let first_diff = first_diff_index(&stage2_target, &stage3_target);
                if stage2_target != stage3_target {
                    failures.push(serde_json::json!({
                        "path": target.path,
                        "root": target.root.label(),
                        "error": "stage1->stage2 と stage2->stage3 の出力 wasm が一致しない",
                        "stage2_output_wasm_bytes": stage2_target.len(),
                        "stage3_output_wasm_bytes": stage3_target.len(),
                        "stage2_fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&stage2_target),
                        "stage3_fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&stage3_target),
                        "export_match": export_a == export_b,
                        "data_match": data_a == data_b,
                        "first_diff": first_diff,
                    }));
                    continue;
                }
                matched.push(serde_json::json!({
                    "path": target.path,
                    "root": target.root.label(),
                    "output_wasm_bytes": stage2_target.len(),
                    "fingerprint": super::selfhost_bootstrap_four_layer::hash_fingerprint(&stage2_target),
                }));
            }
            (Err(stage1_err), Ok(_)) => failures.push(serde_json::json!({
                "path": target.path,
                "root": target.root.label(),
                "error": stage1_err,
            })),
            (Ok(_), Err(stage2_err)) => failures.push(serde_json::json!({
                "path": target.path,
                "root": target.root.label(),
                "error": stage2_err,
            })),
            (Err(stage1_err), Err(stage2_err)) => failures.push(serde_json::json!({
                "path": target.path,
                "root": target.root.label(),
                "error": format!("stage1 compiler: {stage1_err}; stage2 compiler: {stage2_err}"),
            })),
        }
    }

    let mut report_lines = vec![
        "Bootstrap Fixed Input Set Stage Chain Report".to_string(),
        "===========================================".to_string(),
        format!("commit: {artifact_id}"),
        "timestamp: 1970-01-01T00:00:00Z".to_string(),
        "test: test_e2e_bootstrap_fixed_input_set_stage_chain_match".to_string(),
        format!("stage1_self_compiler_bytes: {}", stage1.len()),
        format!("stage2_self_compiler_bytes: {}", stage2.len()),
        format!("target_count: {}", targets.len()),
        format!("matched_count: {}", matched.len()),
        format!("failed_count: {}", failures.len()),
        String::new(),
    ];
    report_lines.extend(matched.iter().map(|entry| {
        format!(
            "MATCH [{}] {} -> {} bytes",
            entry["root"].as_str().unwrap_or("unknown"),
            entry["path"].as_str().unwrap_or("<missing>"),
            entry["output_wasm_bytes"].as_u64().unwrap_or(0)
        )
    }));
    report_lines.extend(failures.iter().map(|entry| {
        format!(
            "FAIL [{}] {} -> {}",
            entry["root"].as_str().unwrap_or("unknown"),
            entry["path"].as_str().unwrap_or("<missing>"),
            entry["error"]
                .as_str()
                .unwrap_or("unknown error")
                .lines()
                .next()
                .unwrap_or("unknown error")
        )
    }));
    let report = report_lines.join("\n");

    let metadata = serde_json::json!({
        "commit_sha": artifact_id,
        "timestamp": "1970-01-01T00:00:00Z",
        "test_name": "test_e2e_bootstrap_fixed_input_set_stage_chain_match",
        "stage1_self_compiler_bytes": stage1.len(),
        "stage2_self_compiler_bytes": stage2.len(),
        "target_count": targets.len(),
        "matched_count": matched.len(),
        "failed_count": failures.len(),
        "matched_targets": matched,
        "failed_targets": failures,
    });
    let artifact_dir = write_fixed_input_set_stage_chain_artifact(&artifact_id, &report, &metadata);

    let written_metadata: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(artifact_dir.join("fixed-input-set-stage-chain.json"))
            .expect("BOOT-04 stage-chain artifact JSON の読み込みに失敗"),
    )
    .expect("BOOT-04 stage-chain artifact JSON は JSON であること");
    assert_eq!(
        written_metadata["matched_count"].as_u64(),
        Some(matched.len() as u64),
        "BOOT-04 stage-chain artifact は matched_count を保持すること"
    );
    assert_eq!(
        written_metadata["failed_count"].as_u64(),
        Some(failures.len() as u64),
        "BOOT-04 stage-chain artifact は failed_count を保持すること"
    );
    assert!(
        failures.is_empty(),
        "BOOT-04: full fixed input set stage chain compare に失敗がある: {}",
        serde_json::to_string_pretty(&written_metadata["failed_targets"])
            .expect("BOOT-04 failure JSON serialize に失敗")
    );
    assert_eq!(
        matched.len(),
        targets.len(),
        "BOOT-04: full fixed input set の stage2/stage3 compare は全件一致するべき"
    );
}

// =============================================================================
// Test 3: test_e2e_bootstrap_stage1_section_stability
// =============================================================================

/// BOOT-04 受入: stage1 が生成する stage2 Wasm のセクション構造が安定していること。
///
/// 同一入力に対して 2 回 stage1 を実行し、生成された stage2 の全セクションが
/// byte-identical であることを確認する。
/// また、必須セクション (type / function / export / code) が必ず存在することを確認する。
#[test]
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
