
#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_runs_string_literal_repro_source() {
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
    .expect("BOOT-04 string-literal-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn main [] (do (print (string-length \".ls\")) (print (string-char-at \".ls\" 0)) (print (string-char-at \".ls\" 1)) (print (string-char-at \".ls\" 2)) 0))\n";
    let stage3_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect(
        "BOOT-04 string-literal-repro: stage2_self_compiler が repro source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 string-literal-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_eleven_imports_compiler_mode(stage3_wasm, "", &["prog"])
        .unwrap_or_else(|e| panic!("BOOT-04 string-literal-repro: 実行失敗: {e}"));
    let values: Vec<i64> = run_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 string-literal-repro: 数値でない出力: {line:?}")
            })
        })
        .collect();
    assert_eq!(values, vec![3, 46, 108, 115]);
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_runs_module_relative_join_repro_source() {
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
    .expect("BOOT-04 module-relative-join-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-eq-loop [left right idx len] (if (>= idx len) true (if (= (string-char-at left idx) (string-char-at right idx)) (text-eq-loop left right (+ idx 1) len) false)))\n(defn text-eq [left right] (let [len (string-length left)] (if (= len (string-length right)) (text-eq-loop left right 0 len) false)))\n(defn path-char [path idx] (string-char-at path idx))\n(defn path-join [base child] (if (= (string-length base) 0) child (let [len (string-length base)] (if (= (string-char-at base (- len 1)) 47) (string-concat base child) (if (= (string-char-at base (- len 1)) 92) (string-concat base child) (string-concat (string-concat base \"/\") child))))))\n(defn module-name-to-relative-loop [name idx len out] (if (>= idx len) (string-concat out \".ls\") (let [piece (if (= (path-char name idx) 46) \"/\" (substring name idx (+ idx 1)))] (module-name-to-relative-loop name (+ idx 1) len (string-concat out piece)))))\n(defn module-name-to-relative [name] (module-name-to-relative-loop name 0 (string-length name) \"\"))\n(defn main [] (print (if (text-eq (path-join \"src\" (module-name-to-relative (command-line-arg 1))) \"src/App/ModuleResolver.ls\") 1 0)))\n";
    let stage3_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 module-relative-join-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 module-relative-join-repro: stage3 wasm validation failed: {e}")
    });

    let run_output =
        run_wasm_with_eleven_imports_compiler_mode(stage3_wasm, "", &["prog", "App.ModuleResolver"])
            .unwrap_or_else(|e| panic!("BOOT-04 module-relative-join-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "1");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_user_call_four_args_repro_source() {
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
    .expect("BOOT-04 user-call-4-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn helper [left right idx len] 1)\n(defn text-eq [left right] (helper left right 0 0))\n(defn main [] (text-eq (command-line-arg 0) (command-line-arg 1)))\n";
    let stage3_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 user-call-4-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 user-call-4-repro: stage3 wasm validation failed: {e}")
    });
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 user-call-4-repro: wasmtime load failed: {} / {:?}",
            e, e
        )
    });
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_runs_command_line_arg_repro_source() {
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
    .expect("BOOT-04 command-line-arg-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.Main)\n(defn main [] (print (string-length (command-line-arg 1))))\n";
    let stage3_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/Main.ls"],
    )
    .expect(
        "BOOT-04 command-line-arg-repro: stage2_self_compiler が repro source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 command-line-arg-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_eleven_imports_compiler_mode(stage3_wasm, "", &["prog", "abc"])
        .unwrap_or_else(|e| panic!("BOOT-04 command-line-arg-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "3");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_runs_print_repro_source() {
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
    .expect("BOOT-04 print-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.Main)\n(defn main [] (print 7))\n";
    let stage3_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 print-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 print-repro: stage3 wasm validation failed: {e}"));
    let start_idx = exported_function_index(stage3_wasm, "_start")
        .expect("BOOT-04 print-repro: _start export が必要");
    eprintln!(
        "BOOT-04 print-repro: sections={:?} _start={} start_ops={:?} main_ops={:?}",
        extract_sections(stage3_wasm),
        start_idx,
        function_operator_debug(stage3_wasm, start_idx, 8),
        function_operator_debug(stage3_wasm, start_idx - 1, 16)
    );

    let run_output = run_wasm_with_eleven_imports_compiler_mode(stage3_wasm, "", &[])
        .unwrap_or_else(|e| panic!("BOOT-04 print-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "7");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reports_print_repro_ir() {
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
    .expect("BOOT-04 print-ir: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.Main)\n(defn main [] (print 7))\n";
    let ir_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/Main.ls", "", "", "", "", "ir"],
    )
    .expect("BOOT-04 print-ir: IR debug 実行に失敗");
    let lines: Vec<&str> = ir_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    eprintln!("BOOT-04 print-ir lines: {:?}", lines);
    assert!(
        lines.first() == Some(&"71"),
        "BOOT-04 print-ir: IR debug marker が不正: {:?}",
        lines
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reports_print_repro_tokens() {
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
    .expect("BOOT-04 print-token: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.Main)\n(defn main [] (print 7))\n";
    let token_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/Main.ls", "", "", "", "", "", "tokens"],
    )
    .expect("BOOT-04 print-token: token debug 実行に失敗");
    let lines: Vec<&str> = token_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    eprintln!("BOOT-04 print-token lines: {:?}", lines);
    assert!(
        lines.first() == Some(&"72"),
        "BOOT-04 print-token: token debug marker が不正: {:?}",
        lines
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_two_imports_zero_source() {
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
    .expect("BOOT-04 two-imports-zero: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.Main)\n(import App.CompilerMode)\n(import App.PipelineSmoke)\n(defn main [] 0)\n";
    let stage3_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 two-imports-zero: stage2_self_compiler が source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 two-imports-zero: stage3 wasm validation failed: {e}"));
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 two-imports-zero: wasmtime load failed: {e}"));
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_one_import_zero_fs_package() {
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
    .expect("BOOT-04 one-import-fs: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-one-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("one-import-fs temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.CompilerMode)\n(defn main [] 0)\n",
    )
    .expect("one-import-fs Main.ls を書けない");
    std::fs::write(
        app_dir.join("CompilerMode.ls"),
        "(module App.CompilerMode)\n(defn compile-file-mode [] 1)\n",
    )
    .expect("one-import-fs CompilerMode.ls を書けない");

    let stage3_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 one-import-fs: stage2_self_compiler が temp package をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 one-import-fs: stage3 wasm validation failed: {e}"));
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 one-import-fs: wasmtime load failed: {e}"));

    std::fs::remove_dir_all(&temp_root).expect("one-import-fs temp dir を削除できない");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_two_imports_zero_fs_package() {
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
    .expect("BOOT-04 two-imports-fs: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-two-imports-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("two-imports-fs temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.CompilerMode)\n(import App.PipelineSmoke)\n(defn main [] 0)\n",
    )
    .expect("two-imports-fs Main.ls を書けない");
    std::fs::write(
        app_dir.join("CompilerMode.ls"),
        "(module App.CompilerMode)\n(defn compile-file-mode [] 1)\n",
    )
    .expect("two-imports-fs CompilerMode.ls を書けない");
    std::fs::write(
        app_dir.join("PipelineSmoke.ls"),
        "(module App.PipelineSmoke)\n(defn run-main-smoke [] 2)\n",
    )
    .expect("two-imports-fs PipelineSmoke.ls を書けない");

    let stage3_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 two-imports-fs: stage2_self_compiler が temp package をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    std::fs::write("/tmp/two_imports_zero_fs_stage3.wasm", stage3_wasm)
        .expect("two-imports-fs stage3 dump に失敗");
    eprintln!(
        "BOOT-04 two-imports-fs stage3: bytes={}, sections={:?}",
        stage3_wasm.len(),
        extract_sections(stage3_wasm)
    );
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 two-imports-fs: stage3 wasm validation failed: {e}"));
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 two-imports-fs: wasmtime load failed: {e}"));

    std::fs::remove_dir_all(&temp_root).expect("two-imports-fs temp dir を削除できない");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_if_builtin_source() {
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
    .expect("BOOT-04 if-builtin: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source =
        "(module App.Main)\n(defn main [] (if (> (string-length (command-line-arg 1)) 0) 1 0))\n";
    let stage3_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 if-builtin: stage2_self_compiler が source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 if-builtin: stage3 wasm validation failed: {e}"));
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 if-builtin: wasmtime load failed: {e}"));
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_main_again() {
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
    .expect("BOOT-04 self-feed: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 self-feed: stage2_self_compiler が Main.ls を再コンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_self_compiler = &stage3_modules[0];
    assert_valid_wasm(stage3_self_compiler);
    std::fs::write(
        "/tmp/main_again_stage3_self_compiler.wasm",
        stage3_self_compiler,
    )
    .expect("stage3 self compiler dump に失敗");
    eprintln!(
        "BOOT-04 stage3 self compiler: bytes={}, sections={:?}",
        stage3_self_compiler.len(),
        extract_sections(stage3_self_compiler)
    );
    validate_wasm_detailed(stage3_self_compiler).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 self-feed: stage3 self compiler validation failed: {e}; sections={:?}; fingerprint={}",
            extract_sections(stage3_self_compiler),
            hash_fingerprint(stage3_self_compiler)
        )
    });

    let stage4_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage3_self_compiler,
        &fixture_dir,
        &["compiler", "minimal.ls"],
    )
    .expect("BOOT-04 self-feed: stage3_self_compiler が minimal.ls をコンパイルできない");
    let stage4_modules = parse_emitted_wasm_modules(&stage4_output, 1);
    let stage4_wasm = &stage4_modules[0];
    assert_valid_wasm(stage4_wasm);

    let run_result = run_wasm_with_eleven_imports_compiler_mode(stage4_wasm, "", &[]);
    assert!(
        run_result.is_ok(),
        "BOOT-04 self-feed: stage4 minimal 実行失敗: {:?}",
        run_result.err()
    );
}

#[test]
#[ignore]
fn test_v2_12_self_hosted_stage2_reports_main_again_stage3_local_bounds() {
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
    .expect("V2-12 main-again-locals: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 main-again-locals: stage2_self_compiler が Main.ls を再コンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_self_compiler = &stage3_modules[0];
    assert_valid_wasm(stage3_self_compiler);

    let violations = local_bound_violations(stage3_self_compiler);
    let first_violation_func = violations.first().and_then(|msg| {
        msg.strip_prefix("func ")
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|func| func.parse::<u32>().ok())
    });
    let first_violation_ops = first_violation_func
        .map(|func| function_operator_debug(stage3_self_compiler, func, 24))
        .unwrap_or_default();
    validate_wasm_detailed(stage3_self_compiler).unwrap_or_else(|e| {
        panic!(
            "V2-12 main-again-locals: stage3 self compiler validation failed: {e}; sections={:?}; violations={:?}; first_violation_func={:?}; first_violation_ops={:?}; fingerprint={}",
            extract_sections(stage3_self_compiler),
            violations,
            first_violation_func,
            first_violation_ops,
            hash_fingerprint(stage3_self_compiler)
        )
    });
    assert!(
        violations.is_empty(),
        "V2-12 main-again-locals: local bound violations: {:?}; first_violation_func={:?}; first_violation_ops={:?}",
        violations,
        first_violation_func,
        first_violation_ops
    );
}

#[test]
#[ignore]
fn test_v2_12_self_hosted_stage2_compiles_large_let_chain() {
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
    .expect("V2-12 let-chain: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let bindings = (0..160)
        .map(|idx| format!("v{idx} {idx}"))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(module App.Main)\n(defn main []\n  (let [{bindings}]\n    v159))\n");

    let stage3_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        &source,
        &["compiler", "inline-large-let-chain.ls"],
    )
    .expect("V2-12 let-chain: stage2_self_compiler が large let-chain source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
}
