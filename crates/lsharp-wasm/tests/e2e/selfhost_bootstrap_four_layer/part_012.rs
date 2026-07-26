
#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_main_shape_source() {
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
    .expect("BOOT-04 main-shape: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-main-shape-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("main-shape temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.CompilerMode)\n(import App.PipelineSmoke)\n(defn main [] (if (> (string-length (command-line-arg 1)) 0) (compile-file-mode) (run-main-smoke)))\n",
    )
    .expect("main-shape Main.ls を書けない");
    std::fs::write(
        app_dir.join("CompilerMode.ls"),
        "(module App.CompilerMode)\n(defn compile-file-mode [] 1)\n",
    )
    .expect("main-shape CompilerMode.ls を書けない");
    std::fs::write(
        app_dir.join("PipelineSmoke.ls"),
        "(module App.PipelineSmoke)\n(defn run-main-smoke [] 2)\n",
    )
    .expect("main-shape PipelineSmoke.ls を書けない");

    let stage3_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &["compiler", "src/App/Main.ls"],
    )
    .expect(
        "BOOT-04 main-shape: stage2_self_compiler が Main.ls shape package をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 main-shape: stage3 wasm validation failed: {e}"));
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 main-shape: wasmtime load failed: {e}"));

    std::fs::remove_dir_all(&temp_root).expect("main-shape temp dir を削除できない");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_text_eq_repro_source() {
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
    .expect("BOOT-04 text-eq-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-eq-loop [left right idx len] (if (>= idx len) true (if (= (string-char-at left idx) (string-char-at right idx)) (text-eq-loop left right (+ idx 1) len) false)))\n(defn text-eq [left right] (let [len (string-length left)] (if (= len (string-length right)) (text-eq-loop left right 0 len) false)))\n(defn main [] (print (if (text-eq (command-line-arg 0) (command-line-arg 1)) 1 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 text-eq-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 text-eq-repro: stage3 wasm validation failed: {e}"));
    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["same", "same"])
        .unwrap_or_else(|e| panic!("BOOT-04 text-eq-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "1");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_string_length_repro_source() {
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
    .expect("BOOT-04 string-length-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-len [text] (string-length text))\n(defn main [] (text-len (command-line-arg 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect(
        "BOOT-04 string-length-repro: stage2_self_compiler が repro source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 string-length-repro: stage3 wasm validation failed: {e}")
    });
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 string-length-repro: wasmtime load failed: {} / {:?}",
            e, e
        )
    });
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_string_length_if_repro_source() {
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
    .expect("BOOT-04 string-length-if-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-len-eq [left right] (let [len (string-length left)] (if (= len (string-length right)) 1 0)))\n(defn main [] (text-len-eq (command-line-arg 0) (command-line-arg 1)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect(
        "BOOT-04 string-length-if-repro: stage2_self_compiler が repro source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 string-length-if-repro: stage3 wasm validation failed: {e}")
    });
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 string-length-if-repro: wasmtime load failed: {} / {:?}",
            e, e
        )
    });
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_let_string_length_repro_source() {
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
    .expect("BOOT-04 let-string-length-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-len [left] (let [len (string-length left)] len))\n(defn main [] (text-len (command-line-arg 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 let-string-length-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 let-string-length-repro: stage3 wasm validation failed: {e}")
    });
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 let-string-length-repro: wasmtime load failed: {} / {:?}",
            e, e
        )
    });
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_eq_string_length_repro_source() {
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
    .expect("BOOT-04 eq-string-length-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-len-eq [left right] (= (string-length left) (string-length right)))\n(defn main [] (text-len-eq (command-line-arg 0) (command-line-arg 1)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect(
        "BOOT-04 eq-string-length-repro: stage2_self_compiler が repro source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 eq-string-length-repro: stage3 wasm validation failed: {e}")
    });
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 eq-string-length-repro: wasmtime load failed: {} / {:?}",
            e, e
        )
    });
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_let_eq_string_length_repro_source() {
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
    .expect("BOOT-04 let-eq-string-length-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-len-eq [left right] (let [len (string-length left)] (= len (string-length right))))\n(defn main [] (text-len-eq (command-line-arg 0) (command-line-arg 1)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 let-eq-string-length-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 let-eq-string-length-repro: stage3 wasm validation failed: {e}")
    });
    let engine = wasmtime::Engine::default();
    wasmtime::Module::new(&engine, stage3_wasm).unwrap_or_else(|e| {
        panic!(
            "BOOT-04 let-eq-string-length-repro: wasmtime load failed: {} / {:?}",
            e, e
        )
    });
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_runs_path_parent_repro_source() {
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
    .expect("BOOT-04 path-parent-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn path-parent [path] (let [len (string-length path)] (if (= len 0) \"\" (if (has-path-sep path 0 len) (let [last (find-last-path-sep path 0 len -1)] (if (< last 0) \"\" (if (= last 0) \"/\" (substring path 0 last)))) \".\"))))\n(defn path-char [path idx] (string-char-at path idx))\n(defn is-path-sep [path idx] (let [ch (path-char path idx)] (if (= ch 47) true (if (= ch 92) true false))))\n(defn has-path-sep [path idx len] (if (>= idx len) false (if (is-path-sep path idx) true (has-path-sep path (+ idx 1) len))))\n(defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))\n(defn main [] (print (string-length (path-parent (command-line-arg 1)))))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 path-parent-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 path-parent-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "a/b"])
        .unwrap_or_else(|e| panic!("BOOT-04 path-parent-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "1");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_runs_path_join_repro_source() {
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
    .expect("BOOT-04 path-join-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn path-join [base child] (if (= (string-length base) 0) child (let [len (string-length base)] (if (= (string-char-at base (- len 1)) 47) (string-concat base child) (if (= (string-char-at base (- len 1)) 92) (string-concat base child) (string-concat (string-concat base \"/\") child))))))\n(defn main [] (print (string-length (path-join (command-line-arg 1) (command-line-arg 2)))))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 path-join-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 path-join-repro: stage3 wasm validation failed: {e}"));

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "a", "b"])
        .unwrap_or_else(|e| panic!("BOOT-04 path-join-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "3");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_runs_string_concat_repro_source() {
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
    .expect("BOOT-04 string-concat-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn main [] (let [value (string-concat (command-line-arg 1) (command-line-arg 2))] (do (print (string-length value)) (print (string-char-at value 0)) (print (string-char-at value 1)) 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect(
        "BOOT-04 string-concat-repro: stage2_self_compiler が repro source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 string-concat-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "a", "b"])
        .unwrap_or_else(|e| panic!("BOOT-04 string-concat-repro: 実行失敗: {e}"));
    let values: Vec<i64> = run_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("BOOT-04 string-concat-repro: 数値でない出力: {line:?}"))
        })
        .collect();
    assert_eq!(values, vec![2, 97, 98]);
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_runs_recursive_string_accumulator_repro_source() {
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
    .expect(
        "BOOT-04 recursive-string-accumulator-repro: stage1 が Main.ls の self-compile に失敗した",
    );
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn grow-loop [seed idx len out] (if (>= idx len) out (grow-loop seed (+ idx 1) len (string-concat out seed))))\n(defn main [] (let [value (grow-loop (command-line-arg 1) 0 2 \"\")] (do (print (string-length value)) (print (string-char-at value 0)) (print (string-char-at value 1)) 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 recursive-string-accumulator-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 recursive-string-accumulator-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "a"])
        .unwrap_or_else(|e| panic!("BOOT-04 recursive-string-accumulator-repro: 実行失敗: {e}"));
    let values: Vec<i64> = run_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 recursive-string-accumulator-repro: 数値でない出力: {line:?}")
            })
        })
        .collect();
    assert_eq!(values, vec![2, 97, 97]);
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_runs_substring_repro_source() {
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
    .expect("BOOT-04 substring-repro: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn main [] (let [value (substring (command-line-arg 1) 1 3)] (do (print (string-length value)) (print (string-char-at value 0)) (print (string-char-at value 1)) 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 substring-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 substring-repro: stage3 wasm validation failed: {e}"));

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "abcd"])
        .unwrap_or_else(|e| panic!("BOOT-04 substring-repro: 実行失敗: {e}"));
    let values: Vec<i64> = run_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("BOOT-04 substring-repro: 数値でない出力: {line:?}"))
        })
        .collect();
    assert_eq!(values, vec![2, 98, 99]);
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_runs_recursive_substring_accumulator_repro_source() {
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
    .expect(
        "BOOT-04 recursive-substring-accumulator-repro: stage1 が Main.ls の self-compile に失敗した",
    );
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-eq-loop [left right idx len] (if (>= idx len) true (if (= (string-char-at left idx) (string-char-at right idx)) (text-eq-loop left right (+ idx 1) len) false)))\n(defn text-eq [left right] (let [len (string-length left)] (if (= len (string-length right)) (text-eq-loop left right 0 len) false)))\n(defn copy-loop [src idx len out] (if (>= idx len) out (copy-loop src (+ idx 1) len (string-concat out (substring src idx (+ idx 1))))))\n(defn main [] (let [src (command-line-arg 1)] (print (if (text-eq (copy-loop src 0 (string-length src) \"\") src) 1 0))))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 recursive-substring-accumulator-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 recursive-substring-accumulator-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "abc"])
        .unwrap_or_else(|e| panic!("BOOT-04 recursive-substring-accumulator-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "1");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_runs_string_concat_literal_suffix_repro_source() {
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
    .expect(
        "BOOT-04 string-concat-literal-suffix-repro: stage1 が Main.ls の self-compile に失敗した",
    );
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-eq-loop [left right idx len] (if (>= idx len) true (if (= (string-char-at left idx) (string-char-at right idx)) (text-eq-loop left right (+ idx 1) len) false)))\n(defn text-eq [left right] (let [len (string-length left)] (if (= len (string-length right)) (text-eq-loop left right 0 len) false)))\n(defn main [] (print (if (text-eq (string-concat (command-line-arg 1) \".ls\") \"ab.ls\") 1 0)))\n";
    let stage3_output = run_wasm_with_six_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "src/App/ModuleResolver.ls"],
    )
    .expect("BOOT-04 string-concat-literal-suffix-repro: stage2_self_compiler が repro source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm).unwrap_or_else(|e| {
        panic!("BOOT-04 string-concat-literal-suffix-repro: stage3 wasm validation failed: {e}")
    });

    let run_output = run_wasm_with_six_imports_compiler_mode(stage3_wasm, "", &["prog", "ab"])
        .unwrap_or_else(|e| panic!("BOOT-04 string-concat-literal-suffix-repro: 実行失敗: {e}"));
    assert_eq!(run_output.trim(), "1");
}
