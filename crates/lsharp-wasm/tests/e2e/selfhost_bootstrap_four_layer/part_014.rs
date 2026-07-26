
#[test]
#[ignore]
fn test_v2_12_self_hosted_stage2_compiles_vector_push_program() {
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
    .expect("V2-12 vector-push: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = r#"
(module App.Main)
(defn main []
  (print
    (let [v (vector-new 1)
          v2 (vector-push v 42)]
      (vector-get v2 0))))
"#;
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "inline-vector-push.ls"],
    )
    .expect("V2-12 vector-push: stage2_self_compiler が vector-push source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &[])
        .expect("V2-12 vector-push: stage3_wasm が runtime imports で実行できること");
    assert_eq!(run_output, "42\n");
}

#[test]
#[ignore]
fn test_v2_12_self_hosted_stage2_loads_wasm_emit_module() {
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
    .expect("V2-12 WasmEmit: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/Backend/Wasm/WasmEmit.ls"],
    )
    .expect("V2-12 WasmEmit: stage2_self_compiler が WasmEmit.ls をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    let violations = local_bound_violations(stage3_wasm);
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "V2-12 WasmEmit: wasmtime load failed: {e}; sections={:?}; violations={:?}; fingerprint={}",
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
    assert!(
        violations.is_empty(),
        "V2-12 WasmEmit: local bound violations: {:?}",
        violations
    );
}

#[test]
#[ignore]
fn test_v2_12_self_hosted_stage2_loads_compiler_mode_module() {
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
    .expect("V2-12 CompilerMode: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/App/CompilerMode.ls"],
    )
    .expect("V2-12 CompilerMode: stage2_self_compiler が CompilerMode.ls をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    let violations = local_bound_violations(stage3_wasm);
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "V2-12 CompilerMode: wasmtime load failed: {e}; sections={:?}; violations={:?}; fingerprint={}",
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
    assert!(
        violations.is_empty(),
        "V2-12 CompilerMode: local bound violations: {:?}",
        violations
    );
}

#[test]
#[ignore = "診断専用: regular invariant は test_v2_12_self_hosted_stage2_loads_compiler_mode_module が担う"]
fn test_v2_12_self_hosted_stage2_reports_compiler_mode_first_violation_body_diff() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let compiler_mode_path = selfhost_root.join("src/App/CompilerMode.ls");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage1_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/CompilerMode.ls"],
    )
    .expect("V2-12 CompilerMode diff: stage1_wasm が CompilerMode.ls をコンパイルできない");
    let stage1_modules = parse_emitted_wasm_modules(&stage1_output, 1);
    let stage1_compiler_mode = &stage1_modules[0];
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage1_compiler_mode)
        .expect("V2-12 CompilerMode diff: stage1 output は load できること");

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 CompilerMode diff: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/App/CompilerMode.ls"],
    )
    .expect(
        "V2-12 CompilerMode diff: stage2_self_compiler が CompilerMode.ls をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_compiler_mode = &stage3_modules[0];

    let bad_indices = local_bound_violation_indices(stage3_compiler_mode);
    let first_bad = *bad_indices
        .first()
        .expect("V2-12 CompilerMode diff: stage3 output に violation があること");
    let stage1_body = function_body_bytes(stage1_compiler_mode, first_bad)
        .expect("V2-12 CompilerMode diff: stage1 body が見つかること");
    let stage3_body = function_body_bytes(stage3_compiler_mode, first_bad)
        .expect("V2-12 CompilerMode diff: stage3 body が見つかること");
    let diff_at = first_byte_diff(stage1_body.as_slice(), stage3_body.as_slice())
        .expect("V2-12 CompilerMode diff: body 差分があること");
    let window_start = diff_at.saturating_sub(16);
    let window_end_stage1 = (diff_at + 24).min(stage1_body.len());
    let window_end_stage3 = (diff_at + 24).min(stage3_body.len());

    panic!(
        "V2-12 CompilerMode diff: path={}; first_bad={}; diff_at={}; stage1_size={}; stage3_size={}; stage1_prefix={:?}; stage3_prefix={:?}; stage1_window={:?}; stage3_window={:?}; stage1_ops={:?}; stage3_violations={:?}; stage1_fingerprint={}; stage3_fingerprint={}",
        compiler_mode_path.display(),
        first_bad,
        diff_at,
        stage1_body.len(),
        stage3_body.len(),
        stage1_body.iter().take(32).copied().collect::<Vec<_>>(),
        stage3_body.iter().take(32).copied().collect::<Vec<_>>(),
        stage1_body[window_start..window_end_stage1].to_vec(),
        stage3_body[window_start..window_end_stage3].to_vec(),
        function_operator_debug(stage1_compiler_mode, first_bad, 20),
        local_bound_violations(stage3_compiler_mode),
        hash_fingerprint(stage1_body.as_slice()),
        hash_fingerprint(stage3_body.as_slice())
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_high_function_index_calls() {
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
    .expect("BOOT-04 high-func-idx: stage1 self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let helper_count = 130usize;
    let helpers = (0..helper_count)
        .map(|idx| format!("(defn helper-{idx} [] {idx})"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("{helpers}\n(defn main [] (helper-129))\n");

    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        &source,
        &["compiler", "HighFunctionIndex.ls"],
    )
    .expect("BOOT-04 high-func-idx: stage2_self_compiler が synthetic source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    let violations = local_bound_violations(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 high-func-idx: validation failed: {e}; bytes={}; sections={:?}; violations={:?}; fingerprint={}",
            stage3_wasm.len(),
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
    assert!(
        violations.is_empty(),
        "BOOT-04 high-func-idx: local bound violations: {:?}",
        violations
    );
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 high-func-idx: wasmtime load failed: {e}; sections={:?}; violations={:?}; fingerprint={}",
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_high_function_index_step64_pattern() {
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
    .expect("BOOT-04 high-step64: stage1 self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let helper_count = 130usize;
    let helpers = (0..helper_count)
        .map(|idx| format!("(defn helper-{idx} [a b c d] a)"))
        .collect::<Vec<_>>()
        .join("\n");
    let let_count = 24usize;
    let mut body = String::from("step24");
    for idx in (1..=let_count).rev() {
        let helper = if idx % 2 == 0 { 129 } else { 128 };
        body = format!("(let [step{idx} (helper-{helper} a b c d)] {body})");
    }
    let source =
        format!("{helpers}\n(defn wrapper [a b c d] {body})\n(defn main [] (wrapper 1 2 3 4))\n");

    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        &source,
        &["compiler", "HighFunctionIndexStep64.ls"],
    )
    .expect("BOOT-04 high-step64: stage2_self_compiler が synthetic source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    let violations = local_bound_violations(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 high-step64: validation failed: {e}; bytes={}; sections={:?}; violations={:?}; fingerprint={}",
            stage3_wasm.len(),
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
    assert!(
        violations.is_empty(),
        "BOOT-04 high-step64: local bound violations: {:?}",
        violations
    );
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 high-step64: wasmtime load failed: {e}; sections={:?}; violations={:?}; fingerprint={}",
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_high_index_parser_like_step64() {
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
    .expect("BOOT-04 parser-like-step64: stage1 self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let helper_count = 130usize;
    let helpers = (0..helper_count)
        .map(|idx| format!("(defn helper-{idx} [state depth] state)"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "{helpers}\n\
         (defn make-state [done next]\n\
           (vector-push (vector-push (vector-new 2) done) next))\n\
         (defn step [state depth]\n\
           (if (<= depth 0)\n\
             (make-state 1 depth)\n\
             (let [kind (vector-get state 0)]\n\
               (if (= kind 4)\n\
                 (make-state 0 (+ depth 1))\n\
                 (if (= kind 5)\n\
                   (make-state 0 (- depth 1))\n\
                   (make-state 0 depth))))))\n\
         (defn cont [state]\n\
           (if (= (vector-get state 0) 1)\n\
             state\n\
             (step state (vector-get state 1))))\n\
         (defn step8 [state depth]\n\
           (let [step1 (step state depth)]\n\
             (let [step2 (cont step1)]\n\
               (let [step3 (cont step2)]\n\
                 (let [step4 (cont step3)]\n\
                   (let [step5 (cont step4)]\n\
                     (let [step6 (cont step5)]\n\
                       (let [step7 (cont step6)]\n\
                         (let [step8 (cont step7)]\n\
                           step8))))))))\n\
         (defn cont8 [state]\n\
           (if (= (vector-get state 0) 1)\n\
             state\n\
             (step8 state (vector-get state 1))))\n\
         (defn step64 [state depth]\n\
           (let [step1 (step8 state depth)]\n\
             (let [step2 (cont8 step1)]\n\
               (let [step3 (cont8 step2)]\n\
                 (let [step4 (cont8 step3)]\n\
                   (let [step5 (cont8 step4)]\n\
                     (let [step6 (cont8 step5)]\n\
                       (let [step7 (cont8 step6)]\n\
                         (let [step8 (cont8 step7)]\n\
                           step8))))))))\n\
         (defn main [] (step64 (make-state 0 3) 3))\n"
    );

    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        &source,
        &["compiler", "HighIndexParserLikeStep64.ls"],
    )
    .expect(
        "BOOT-04 parser-like-step64: stage2_self_compiler が synthetic source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    let violations = local_bound_violations(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 parser-like-step64: validation failed: {e}; bytes={}; sections={:?}; violations={:?}; fingerprint={}",
            stage3_wasm.len(),
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
    assert!(
        violations.is_empty(),
        "BOOT-04 parser-like-step64: local bound violations: {:?}",
        violations
    );
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 parser-like-step64: wasmtime load failed: {e}; sections={:?}; violations={:?}; fingerprint={}",
            extract_sections(stage3_wasm),
            violations,
            hash_fingerprint(stage3_wasm)
        )
    });
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reports_stage3_minimal_progress() {
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
    let minimal_src = std::fs::read_to_string(fixture_dir.join("minimal.ls"))
        .expect("BOOT-04 stage3-minimal-progress: minimal fixture を読めない");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 stage3-minimal-progress: stage1 self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 stage3-minimal-progress: stage2 self-compile に失敗した");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_self_compiler = &stage3_modules[0];
    assert_valid_wasm(stage3_self_compiler);

    let progress_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage3_self_compiler,
        &fixture_dir,
        &["compiler", "minimal.ls", "debug", "progress", "minimal"],
    )
    .expect("BOOT-04 stage3-minimal-progress: stage3 compiler の progress debug 実行に失敗した");
    let values = progress_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|err| {
                panic!("BOOT-04 stage3-minimal-progress: 数値でない debug 出力: {line:?} / {err}")
            })
        })
        .collect::<Vec<_>>();
    assert!(
        values.len() >= 26,
        "BOOT-04 stage3-minimal-progress: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        &values[..6],
        &[1, 1, 2, 0, 3, 1],
        "BOOT-04 stage3-minimal-progress: top-level progress prefix が崩れている: {:?}",
        values
    );
    assert!(
        values
            .windows(4)
            .any(|window| window == [29, 0, minimal_src.len() as i64, 1]),
        "BOOT-04 stage3-minimal-progress: pair progress marker 29 が崩れている: {:?}",
        values
    );
    assert!(
        values.windows(3).any(|window| window == [40, 0, 20]),
        "BOOT-04 stage3-minimal-progress: defn marker 40 が崩れている: {:?}",
        values
    );
    assert!(
        values.windows(2).any(|window| window == [41, 0]),
        "BOOT-04 stage3-minimal-progress: compiled-fn start marker 41 が崩れている: {:?}",
        values
    );
    assert!(
        values.windows(3).any(|window| window == [42, 0, 1]),
        "BOOT-04 stage3-minimal-progress: compiled-fn count marker 42 が崩れている: {:?}",
        values
    );
    assert!(
        values.windows(2).any(|window| window == [43, 0]),
        "BOOT-04 stage3-minimal-progress: decl completion marker 43 が崩れている: {:?}",
        values
    );
    assert!(
        values
            .windows(4)
            .any(|window| window == [30, 0, minimal_src.len() as i64, 1]),
        "BOOT-04 stage3-minimal-progress: pair completion marker 30 が崩れている: {:?}",
        values
    );
    assert_eq!(
        &values[values.len() - 2..],
        &[4, 1],
        "BOOT-04 stage3-minimal-progress: compiled function count は 1 であるべき: {:?}",
        values
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_classifies_chunked_lexer_failure_band() {
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
    .expect("BOOT-04 chunk diag: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let build_helper_source = |count: usize| {
        let helpers = (0..count)
            .map(|idx| format!("(defn helper-{idx} [] 0)"))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{helpers}\n(defn main [] 42)\n")
    };
    let parse_stage3_wasm = |label: &str, output: &str| -> Result<usize, String> {
        let modules = std::panic::catch_unwind(|| parse_emitted_wasm_modules(output, 1))
            .map_err(|_| format!("{label}: 出力が wasm モジュール形式でない"))?;
        let wasm = &modules[0];
        assert_valid_wasm(wasm);
        Ok(wasm.len())
    };
    let try_compile_inline = |label: &str, source: &str| -> Result<usize, String> {
        let output = run_wasm_with_six_imports_compiler_mode(
            stage2_self_compiler,
            source,
            &["compiler", label],
        )
        .map_err(|err| format!("{label}: {err}"))?;
        parse_stage3_wasm(label, &output)
    };
    let try_compile_file = |path: &str| -> Result<usize, String> {
        let output = run_wasm_with_six_imports_compiler_mode_fs(
            stage2_self_compiler,
            &selfhost_root,
            &["compiler", path],
        )
        .map_err(|err| format!("{path}: {err}"))?;
        parse_stage3_wasm(path, &output)
    };
    let summarize = |result: &Result<usize, String>| match result {
        Ok(bytes) => format!("ok({bytes} bytes)"),
        Err(err) => {
            let head = err.lines().next().unwrap_or(err);
            format!("err({head})")
        }
    };
    let summarize_optional = |result: &Option<Result<usize, String>>| match result {
        Some(inner) => summarize(inner),
        None => "skipped".to_string(),
    };

    // helper 1 個あたり約 7 トークンなので、36 個は 256 トークン未満、37 個で最初の chunk 境界を跨ぐ。
    let below_boundary = try_compile_inline("diag-below-boundary.ls", &build_helper_source(36));
    let cross_boundary = try_compile_inline("diag-cross-boundary.ls", &build_helper_source(37));
    let multi_chunk = try_compile_inline("diag-multi-chunk.ls", &build_helper_source(80));
    let need_real_world = below_boundary.is_ok() && cross_boundary.is_ok() && multi_chunk.is_ok();
    let large_single_file = need_real_world
        .then(|| try_compile_inline("diag-large-single-file.ls", &build_helper_source(800)));
    let main_again = need_real_world.then(|| try_compile_file("src/App/Main.ls"));

    let classification = if below_boundary.is_err() {
        "local-before-boundary"
    } else if cross_boundary.is_err() {
        "first-boundary-crossing"
    } else if multi_chunk.is_err() {
        "post-first-chunk"
    } else if large_single_file
        .as_ref()
        .is_some_and(|result| result.is_err())
        || main_again.as_ref().is_some_and(|result| result.is_err())
    {
        "real-world-only"
    } else {
        "no-probe-failure"
    };

    eprintln!(
        "BOOT-04 chunk-band diag: below={} cross={} multi={} large={} main={} => {}",
        summarize(&below_boundary),
        summarize(&cross_boundary),
        summarize(&multi_chunk),
        summarize_optional(&large_single_file),
        summarize_optional(&main_again),
        classification
    );

    assert!(matches!(
        classification,
        "local-before-boundary"
            | "first-boundary-crossing"
            | "post-first-chunk"
            | "real-world-only"
            | "no-probe-failure"
    ));
}
