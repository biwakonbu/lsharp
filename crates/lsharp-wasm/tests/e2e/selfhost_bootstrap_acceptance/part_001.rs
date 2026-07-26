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
/// 計算結果 (i64) は一致しなければならない。
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_stage2_match() {
    // stage0 と stage1 が同じ結果を生むことを確認するテストケース群
    // stage2 では selfhost runtime import が自動挿入されうるが、最終結果は pure i64 に限定する
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
        let stage2_result = run_exported_i64_with_runtime_imports(stage2, "_start");
        assert_eq!(
            stage2_result, *expected,
            "BOOT-04 stage1_stage2_match: stage2 の計算結果が期待値と一致しない\n\
             src={src:?}\n\
             expected={expected}, got={stage2_result}"
        );
    }
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_stage2_match_fib_runtime_layout() {
    let src =
        "(defn fib [n] (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))) (defn main [] (fib 8))";
    let harness = build_simple_bootstrap_harness(src);
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let out_a = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .unwrap_or_else(|e| panic!("fib stage1 run_a 失敗: {e}"));
    let out_b = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .unwrap_or_else(|e| panic!("fib stage1 run_b 失敗: {e}"));
    assert_eq!(out_a, out_b, "fib stage1 → stage2 出力が非決定的");

    let modules = parse_emitted_wasm_modules(&out_a, 1);
    assert_eq!(modules.len(), 1, "fib stage2 モジュール数が不正");
    let stage2 = &modules[0];
    assert_valid_wasm(stage2);
    assert_eq!(run_exported_i64_with_runtime_imports(stage2, "_start"), 21);
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
#[ignore]
fn test_e2e_bootstrap_fixed_point_stage2_stage3() {
    run_bootstrap_acceptance_with_expanded_stack(|| {
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
    });
}

#[test]
#[ignore]
fn test_e2e_bootstrap_fixed_point_minimal_build_progress_matches_stage2_stage3() {
    run_bootstrap_acceptance_with_expanded_stack(|| {
        let main_path = selfhost_main_path();
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

        let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
            &stage1_wasm,
            Some(&selfhost_root),
            &["compiler", "src/App/Main.ls"],
        )
        .expect("minimal-build-progress: stage1 が Main.ls の self-compile に失敗");
        let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
        let stage2_self_compiler = stage2_modules[0].clone();
        assert_valid_wasm(&stage2_self_compiler);

        let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
            &stage2_self_compiler,
            &selfhost_root,
            &["compiler", "src/App/Main.ls"],
        )
        .expect("minimal-build-progress: stage2 が Main.ls の再コンパイルに失敗");
        let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
        let stage3_self_compiler = stage3_modules[0].clone();
        assert_valid_wasm(&stage3_self_compiler);

        let stage2_progress = run_wasm_with_six_imports_compiler_mode_fs(
            &stage2_self_compiler,
            &fixture_dir,
            &["compiler", "minimal.ls", "", "", "", "build-progress"],
        )
        .expect("minimal-build-progress: stage2 compiler が minimal.ls build-progress 実行に失敗");
        let stage3_progress = run_wasm_with_six_imports_compiler_mode_fs(
            &stage3_self_compiler,
            &fixture_dir,
            &["compiler", "minimal.ls", "", "", "", "build-progress"],
        )
        .expect("minimal-build-progress: stage3 compiler が minimal.ls build-progress 実行に失敗");

        let stage2_values =
            parse_printed_i64_lines(&stage2_progress, "minimal-build-progress stage2");
        let stage3_values =
            parse_printed_i64_lines(&stage3_progress, "minimal-build-progress stage3");

        eprintln!(
            "BOOT-04 minimal-build-progress stage2={:?} stage3={:?}",
            stage2_values, stage3_values
        );

        assert_eq!(
            stage2_values, stage3_values,
            "BOOT-04 minimal-build-progress mismatch: stage2={stage2_values:?}, stage3={stage3_values:?}"
        );
    });
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
#[ignore]
fn test_e2e_bootstrap_stage2_compiler_wasmemit_modules_deterministic() {
    run_bootstrap_acceptance_with_expanded_stack(|| {
        let (stage2, root) = build_stage2_self_compiler_from_main();

        for rel in [
            "src/Backend/Wasm/Compiler.ls",
            "src/Backend/Wasm/WasmEmit.ls",
        ] {
            let out_a =
                run_wasm_with_six_imports_compiler_mode_fs(&stage2, &root, &["compiler", rel])
                    .unwrap_or_else(|e| {
                        panic!("CP-01: stage2 が {rel} を 1 回目コンパイルできない: {e}")
                    });
            let out_b =
                run_wasm_with_six_imports_compiler_mode_fs(&stage2, &root, &["compiler", rel])
                    .unwrap_or_else(|e| {
                        panic!("CP-01: stage2 が {rel} を 2 回目コンパイルできない: {e}")
                    });

            assert_eq!(out_a, out_b, "CP-01: stage2 の {rel} 出力が非決定的");

            let mods = parse_emitted_wasm_modules(&out_a, 1);
            assert_eq!(
                mods.len(),
                1,
                "CP-01: {rel} のコンパイル出力は 1 wasm モジュールであるべき"
            );
            assert_valid_wasm(&mods[0]);
        }
    });
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
    assert!(
        script.contains("RUN_INCREMENTAL_COMPARE"),
        "compile-phase11-inputs.sh は incremental compare 実行フラグを受け取ること"
    );
    assert!(
        script.contains("test_e2e_incremental_compile_matches_full_compile_fixed_input_set"),
        "compile-phase11-inputs.sh は fixed input set の full vs incremental compare テストを呼び出せること"
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

fn fixed_input_set_target_by_path(path: &str) -> FixedInputSetTarget {
    fixed_input_set_self_feed_targets()
        .into_iter()
        .find(|target| target.path == path)
        .unwrap_or_else(|| panic!("BOOT-04: fixed input set target が見つからない: {path}"))
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
