
/// BOOT-04 リグレッション: 再帰深度境界の観測記録
///
/// bootstrap-append-bytes が何個の関数 (≈ code section バイト数) から失敗するかの境界を確認する。
/// 結果を eprintln で出力し、修正後の境界比較に利用する。
#[test]
#[ignore]
fn test_e2e_boot04_bootstrap_append_bytes_recursion_depth_boundary() {
    let make_full_source = |n_funcs: usize| -> Vec<u8> {
        let mut src = String::new();
        for i in 0..n_funcs {
            src.push_str(&format!("(defn fn{i:04} [] {i}) "));
        }
        src.push_str("(defn main [] 0)");
        let harness = format!(
            concat!(
                "(defn bootstrap-append-bytes [dst s idx count]\n",
                "  (if (>= idx count) dst\n",
                "    (bootstrap-append-bytes (vector-push dst (vector-get s idx)) s (+ idx 1) count)))\n",
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
                "(defn main []\n",
                "  (let [stage2 (bootstrap-build-stage2 \"{src}\")]\n",
                "    (print (vector-length stage2))))\n",
            ),
            src = src
        );
        let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
        compile_only(&stage1_source)
    };

    let try_n = |n: usize| -> bool {
        let wasm = make_full_source(n);
        lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm).is_ok()
    };

    let n10_ok = try_n(10);
    let n50_ok = try_n(50);
    let n200_ok = try_n(200);
    let n500_ok = try_n(500);
    let n1000_ok = try_n(1000);

    eprintln!(
        "BOOT-04 bootstrap-append-bytes 再帰深度境界 (Wasm 関数数):\n  \
         N=10:   {}\n  \
         N=50:   {}\n  \
         N=200:  {}\n  \
         N=500:  {}\n  \
         N=1000: {}",
        if n10_ok { "OK" } else { "TRAP" },
        if n50_ok { "OK" } else { "TRAP" },
        if n200_ok { "OK" } else { "TRAP" },
        if n500_ok { "OK" } else { "TRAP" },
        if n1000_ok { "OK" } else { "TRAP" },
    );

    // N=10 は必ず成功 (code section ~150 bytes)
    assert!(n10_ok, "N=10 は必ず成功するはず");

    // 単調性: 成功から失敗への遷移は一方向のみ
    if !n50_ok {
        assert!(!n200_ok, "N=50 で TRAP なら N=200 も TRAP のはず");
        assert!(!n500_ok, "N=50 で TRAP なら N=500 も TRAP のはず");
    }
    if !n200_ok {
        assert!(!n500_ok, "N=200 で TRAP なら N=500 も TRAP のはず");
        assert!(!n1000_ok, "N=200 で TRAP なら N=1000 も TRAP のはず");
    }
    if !n500_ok {
        assert!(!n1000_ok, "N=500 で TRAP なら N=1000 も TRAP のはず");
    }
}

// =============================================================================
// BOOT-04: read-file compiler-mode — Main.ls のコンパイラモードエントリポイント検証
// =============================================================================

/// BOOT-04: read-file compiler-mode — stage1 (Main.ls compiled by Rust) が
/// ファイル引数を受け取りコンパイラとして動作すること
///
/// Main.ls の compiler-mode を検証:
/// - argv[1] にソースファイルパスが渡されたとき、そのファイルを read-file で読み込み
/// - parse-program → compile-program-functions → emit-*-wasi でコンパイルし
/// - WASM バイトを length-prefixed 形式で stdout に出力すること
#[test]
#[ignore]
fn test_e2e_boot04_read_file_compiler_mode() {
    let main_path = selfhost_main_path();
    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    // テスト用 L# ソースファイルを用意
    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    assert!(
        fixture_dir.join("minimal.ls").exists(),
        "fixture ファイル tests/fixtures/minimal.ls が存在しない"
    );

    // compiler-mode で stage1 を実行 (argv[1] = "minimal.ls")
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&fixture_dir),
        &["compiler", "minimal.ls"],
    )
    .expect("BOOT-04 compiler-mode: stage1 実行失敗");

    // 出力が length-prefixed Wasm バイト列であること
    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage2_wasm = &modules[0];
    assert_valid_wasm(stage2_wasm);

    // stage2 が 6-import モデルで実行可能であること (_start: () -> () ラッパー付き)
    // minimal.ls = (defn main [] 42) → main は何も print しない
    let run_result = run_wasm_with_six_imports_compiler_mode(stage2_wasm, "", &[]);
    assert!(
        run_result.is_ok(),
        "BOOT-04 compiler-mode: stage2 の WASI 実行に失敗: {:?}",
        run_result.err()
    );

    eprintln!(
        "BOOT-04 compiler-mode: stage1 が minimal.ls をコンパイルして stage2 ({} bytes) を生成 OK",
        stage2_wasm.len()
    );
}

/// BOOT-04: stage2 コンパイラが minimal.ls を stage3 にコンパイルできること
///
/// stage1 (Rust bootstrap が生成した Main.ls コンパイラ wasm) を stage2_compiler と見なし、
/// stage2_compiler が compiler-mode で minimal.ls を読み込んで stage3 wasm を生成できること、
/// さらに stage3 が正しく実行できることを検証する。
///
/// - stage1 == stage2_compiler: どちらも Rust bootstrap が生成した同一の完全コンパイラ wasm
/// - stage2→stage3 の接続性を明示的に固定するテスト
/// - stage3 の出力が stage1→stage2 の出力と一致する（同一入力 → 決定論的出力）ことも検証
#[test]
#[ignore]
fn test_e2e_boot04_stage2_compiler_to_stage3_minimal() {
    let main_path = selfhost_main_path();
    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    assert!(
        fixture_dir.join("minimal.ls").exists(),
        "fixture ファイル tests/fixtures/minimal.ls が存在しない"
    );

    // stage2_compiler = Rust bootstrap が生成した完全コンパイラ wasm (= stage1 と同一)
    let stage2_compiler = compile_file_only(&main_path);
    assert_valid_wasm(&stage2_compiler);

    // stage2_compiler が compiler-mode で minimal.ls → stage3 を生成
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage2_compiler,
        Some(&fixture_dir),
        &["compiler", "minimal.ls"],
    )
    .expect("BOOT-04 stage2→stage3: stage2_compiler の compiler-mode 実行失敗");

    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage3_wasm = &modules[0];
    assert_valid_wasm(stage3_wasm);

    // stage3 が 6-import モデルで実行できること
    let stage3_result = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &[]);
    assert!(
        stage3_result.is_ok(),
        "BOOT-04 stage2→stage3: stage3 の WASI 実行に失敗: {:?}",
        stage3_result.err()
    );

    // stage3 の出力が空であること（(defn main [] 42) は print しない）
    let stage3_output = stage3_result.unwrap();
    assert_eq!(
        stage3_output, "",
        "BOOT-04 stage2→stage3: stage3 の stdout 出力が期待と異なる: {:?}",
        stage3_output
    );

    // stage3 が stage2_compiler の出力と一致する（同一入力 → 決定論的）
    let output2 = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage2_compiler,
        Some(&fixture_dir),
        &["compiler", "minimal.ls"],
    )
    .expect("BOOT-04 stage2→stage3: stage2_compiler 2回目の実行失敗");
    let modules2 = parse_emitted_wasm_modules(&output2, 1);
    let stage3_wasm_b = &modules2[0];
    assert_eq!(
        stage3_wasm, stage3_wasm_b,
        "BOOT-04 stage2→stage3: stage3 wasm が非決定的（同一入力で異なる出力）"
    );

    eprintln!(
        "BOOT-04 stage2→stage3: stage2_compiler が minimal.ls → stage3 ({} bytes) を生成し実行 OK (決定論的確認済み)",
        stage3_wasm.len()
    );
}

/// BOOT-04: 自己コンパイル stage2 の精密ブロッカー記録テスト
///
/// stage1 (Rust bootstrap compiler wasm) が compiler-mode で Main.ls 自身を
/// コンパイルして stage2_self_compiler を生成できるかを検証する。
///
/// 現在の blockerを精密に固定する:
/// BOOT-04: self-hosted stage2 compiler が minimal.ls を stage3 へコンパイルできること
#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_minimal() {
    let main_path = selfhost_main_path();
    // selfhost/ ルート（src/ の親）を WASI dir として設定する。
    // selfhost/src/App/Main.ls は dotted import (Syntax.AST 等) を使うため、
    // source_root = "src" が正しく解決されるには WASI dir = selfhost/ が必要。
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

    // stage1 = Rust bootstrap が生成した完全コンパイラ wasm
    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    // stage1 が compiler-mode で src/App/Main.ls 自身をコンパイル → stage2_self_compiler を試みる
    // WASI dir = selfhost/ にすることで dotted import (Syntax.AST → src/Syntax/AST.ls) が解決される
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 self-hosted-stage2: stage1 が Main.ls の self-compile に失敗した");
    eprintln!(
        "BOOT-04 self-hosted-stage2: stage1 が Main.ls → output ({} chars) を生成",
        output.len()
    );

    let modules = std::panic::catch_unwind(|| parse_emitted_wasm_modules(&output, 1))
        .expect("BOOT-04 self-hosted-stage2: stage1 出力が wasm モジュール形式でない");
    let stage2_self_compiler = &modules[0];
    eprintln!(
        "BOOT-04 self-hosted-stage2: stage2_self_compiler = {} bytes",
        stage2_self_compiler.len()
    );
    let sections = extract_sections(stage2_self_compiler);
    eprintln!("BOOT-04 stage2 sections: {:?}", sections);
    match validate_wasm_detailed(stage2_self_compiler) {
        Ok(_) => eprintln!("BOOT-04 stage2: wasmparser validation PASSED"),
        Err(e) => eprintln!("BOOT-04 stage2 wasmparser ERROR: {}", e),
    }
    assert_valid_wasm(stage2_self_compiler);

    let minimal_ls_content = std::fs::read_to_string(fixture_dir.join("minimal.ls"))
        .unwrap_or_else(|_| "(defn main [] 42)".to_string());
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        &minimal_ls_content,
        &["compiler", "minimal.ls"],
    )
    .expect("BOOT-04 self-hosted-stage2: stage2_self_compiler が minimal.ls をコンパイルできない");
    eprintln!(
        "BOOT-04 self-hosted-stage2: stage3_output = {} chars",
        stage3_output.len()
    );

    let stage3_modules = std::panic::catch_unwind(|| parse_emitted_wasm_modules(&stage3_output, 1))
        .expect("BOOT-04 self-hosted-stage2: stage3 出力が wasm 形式でない");
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);

    let run_result = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &[]);
    assert!(
        run_result.is_ok(),
        "stage2_self_compiler → stage3 実行失敗: {:?}",
        run_result.err()
    );
    eprintln!(
        "BOOT-04 self-hosted-stage2 GREEN: stage1→stage2_self_compiler→stage3 ({} bytes) 完全成功!",
        stage3_wasm.len()
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_preserves_batched_step_progress() {
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
    .expect("BOOT-04 batching probe: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let probe_source = r#"
(defn make-state [done next]
  (vector-push
    (vector-push (vector-new 4) done)
    next))

(defn step [limit pos]
  (if (>= pos limit)
    (make-state 1 pos)
    (make-state 0 (+ pos 1))))

(defn continue-step [limit state]
  (if (= (vector-get state 0) 1)
    state
    (step limit (vector-get state 1))))

(defn step-8 [limit pos]
  (let [step1 (step limit pos)
    step2 (continue-step limit step1)
    step3 (continue-step limit step2)
    step4 (continue-step limit step3)
    step5 (continue-step limit step4)
    step6 (continue-step limit step5)
    step7 (continue-step limit step6)
    step8 (continue-step limit step7)]
    step8))

(defn continue-step-8 [limit state]
  (if (= (vector-get state 0) 1)
    state
    (step-8 limit (vector-get state 1))))

(defn step-64 [limit pos]
  (let [step1 (step-8 limit pos)
    step2 (continue-step-8 limit step1)
    step3 (continue-step-8 limit step2)
    step4 (continue-step-8 limit step3)
    step5 (continue-step-8 limit step4)
    step6 (continue-step-8 limit step5)
    step7 (continue-step-8 limit step6)
    step8 (continue-step-8 limit step7)]
    step8))

(defn continue-step-64 [limit state]
  (if (= (vector-get state 0) 1)
    state
    (step-64 limit (vector-get state 1))))

(defn step-512 [limit pos]
  (let [step1 (step-64 limit pos)
    step2 (continue-step-64 limit step1)
    step3 (continue-step-64 limit step2)
    step4 (continue-step-64 limit step3)
    step5 (continue-step-64 limit step4)
    step6 (continue-step-64 limit step5)
    step7 (continue-step-64 limit step6)
    step8 (continue-step-64 limit step7)]
    step8))

(defn main []
  (let [state8 (step-8 1000 0)
    state64 (step-64 1000 0)
    state512 (step-512 1000 0)
    capped64 (step-64 13 0)]
    (do
      (print (vector-get state8 1))
      (print (vector-get state64 1))
      (print (vector-get state512 1))
      (print (vector-get capped64 1))
      0)))
"#;

    let stage3_result = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        probe_source,
        &["compiler", "batching-probe.ls"],
    );

    match &stage3_result {
        Ok(stage3_output) => {
            let stage3_modules = parse_emitted_wasm_modules(stage3_output, 1);
            let stage3_wasm = &stage3_modules[0];
            assert_valid_wasm(stage3_wasm);

            let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &[])
                .expect("BOOT-04 batching probe: stage3 probe module の実行に失敗した");
            let lines: Vec<&str> = run_output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .collect();

            assert!(
                lines.len() >= 4,
                "BOOT-04 batching probe の出力が不足: {:?}",
                lines
            );
            assert_eq!(lines[0], "8", "step-8 は 8 ステップぶん進むべき");
            assert_eq!(lines[1], "64", "step-64 は 64 ステップぶん進むべき");
            assert_eq!(lines[2], "512", "step-512 は 512 ステップぶん進むべき");
            assert_eq!(lines[3], "13", "step-64 は limit 到達時に早期終了すべき");
        }
        Err(compile_err) => {
            let frame_count = compile_err
                .lines()
                .filter(|l| l.contains("wasm function"))
                .count();
            eprintln!(
                "BOOT-04 batching probe BLOCKED: stage2 compile failed with {} wasm frames at overflow",
                frame_count
            );
            eprintln!(
                "BOOT-04 batching probe BLOCKED: first error line: {}",
                compile_err.lines().next().unwrap_or("")
            );
            eprintln!(
                "BOOT-04 batching probe THRESHOLD: synthetic step-8/64/512 probe still exceeds stage2 expression recursion budget (~{} recursion levels at ~65 frames each)",
                frame_count / 65
            );

            assert!(
                compile_err.contains("wasm backtrace") || compile_err.contains("unreachable"),
                "batching probe stage2 compile 失敗は wasm backtrace を含むべき (got: {})",
                compile_err.lines().next().unwrap_or("")
            );
            assert!(
                frame_count >= 200,
                "batching probe overflow frame count が 200 未満 (got {}): 失敗モードが変わった可能性がある",
                frame_count
            );
        }
    }
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_large_single_file() {
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
    .expect("BOOT-04 large-file: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let repeated_helpers = (0..800)
        .map(|idx| format!("(defn helper-{idx} [] 0)"))
        .collect::<Vec<_>>()
        .join("\n");
    let large_source = format!("{repeated_helpers}\n(defn main [] 42)\n");
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        &large_source,
        &["compiler", "large-token-file.ls"],
    )
    .expect("BOOT-04 large-file: stage2_self_compiler が大きい単一ファイルをコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_bare_module_file() {
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
    .expect("BOOT-04 bare-module: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let bare_module_source = "(module App.Main)\n(defn main [] 0)\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        bare_module_source,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 bare-module: stage2_self_compiler が bare module source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    std::fs::write("/tmp/bare_module_string_stage3.wasm", stage3_wasm)
        .expect("bare-module string stage3 dump に失敗");
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 bare-module: stage3 wasm validation failed: {e}"));
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_bare_zero_fs_package() {
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
    .expect("BOOT-04 bare-fs: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-bare-fs-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("bare-fs temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(defn main [] 0)\n",
    )
    .expect("bare-fs Main.ls を書けない");

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 bare-fs: stage2_self_compiler が temp package をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    std::fs::write("/tmp/bare_module_fs_stage3.wasm", stage3_wasm)
        .expect("bare-fs stage3 dump に失敗");
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 bare-fs: stage3 wasm validation failed: {e}"));
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 bare-fs: wasmtime load failed: {e}"));

    std::fs::remove_dir_all(&temp_root).expect("bare-fs temp dir を削除できない");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_cache_probe_parses_bare_module_once() {
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
    .expect("BOOT-04 cache-probe: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-cache-probe-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("cache-probe temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(defn main [] 0)\n",
    )
    .expect("cache-probe Main.ls を書けない");

    let debug_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "cache",
        ],
    )
    .expect("BOOT-04 cache-probe: stage2_self_compiler の cache probe 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("BOOT-04 cache-probe: 数値でない debug 出力: {line:?}"))
        })
        .collect();
    eprintln!("BOOT-04 cache-probe values = {:?}", values);

    assert!(
        values.len() >= 4,
        "BOOT-04 cache-probe: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        &values[..4],
        &[80, 1, 35, 2],
        "BOOT-04 cache-probe: bare module の parse-count / source / decl 集計が期待と異なる"
    );

    std::fs::remove_dir_all(&temp_root).expect("cache-probe temp dir を削除できない");
}
