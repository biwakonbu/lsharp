
#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_cache_probe_reads_main_again_entry() {
    let main_path = selfhost_main_path();
    let main_src =
        std::fs::read_to_string(&main_path).expect("BOOT-04 main-cache-probe: Main.ls を読めない");
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
    .expect("BOOT-04 main-cache-probe: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let debug_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
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
    .expect("BOOT-04 main-cache-probe: stage2_self_compiler の cache probe 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 main-cache-probe: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 main-cache-probe values = {:?}", values);

    assert!(
        values.len() >= 4,
        "BOOT-04 main-cache-probe: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        &values[..4],
        &[80, 1, main_src.len() as i64, 4],
        "BOOT-04 main-cache-probe: entry source parse 集計が期待と異なる"
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_cache_pairs_probe_handles_bare_module() {
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
    .expect("BOOT-04 cache-pairs-bare: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-cache-pairs-bare-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("cache-pairs-bare temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(defn main [] 0)\n",
    )
    .expect("cache-pairs-bare Main.ls を書けない");

    let debug_output = run_wasm_with_eleven_imports_compiler_mode_fs(
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
            "",
            "pairs",
        ],
    )
    .expect("BOOT-04 cache-pairs-bare: stage2_self_compiler の pairs probe 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 cache-pairs-bare: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 cache-pairs-bare values = {:?}", values);

    assert!(
        values.len() >= 4,
        "BOOT-04 cache-pairs-bare: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        &values[..4],
        &[81, 1, 1, 2],
        "BOOT-04 cache-pairs-bare: bare module の pair 集計が期待と異なる"
    );

    std::fs::remove_dir_all(&temp_root).expect("cache-pairs-bare temp dir を削除できない");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_cache_pairs_probe_handles_one_import() {
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
    .expect("BOOT-04 cache-pairs-one-import: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-cache-pairs-one-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("cache-pairs-one-import temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.Helper)\n(defn main [] 0)\n",
    )
    .expect("cache-pairs-one-import Main.ls を書けない");
    std::fs::write(
        app_dir.join("Helper.ls"),
        "(module App.Helper)\n(defn helper [] 1)\n",
    )
    .expect("cache-pairs-one-import Helper.ls を書けない");

    let debug_output = run_wasm_with_eleven_imports_compiler_mode_fs(
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
            "",
            "pairs",
        ],
    )
    .expect("BOOT-04 cache-pairs-one-import: stage2_self_compiler の pairs probe 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 cache-pairs-one-import: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 cache-pairs-one-import values = {:?}", values);

    assert!(
        values.len() >= 4,
        "BOOT-04 cache-pairs-one-import: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        &values[..4],
        &[81, 2, 2, 3],
        "BOOT-04 cache-pairs-one-import: single import graph の pair 集計が期待と異なる"
    );

    std::fs::remove_dir_all(&temp_root).expect("cache-pairs-one-import temp dir を削除できない");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_cache_pairs_probe_reads_main_again_graph() {
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
    .expect("BOOT-04 main-cache-pairs: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let debug_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "pairs",
        ],
    )
    .expect("BOOT-04 main-cache-pairs: stage2_self_compiler の pairs probe 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 main-cache-pairs: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 main-cache-pairs values = {:?}", values);

    assert!(
        values.len() >= 4,
        "BOOT-04 main-cache-pairs: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(values[0], 81, "BOOT-04 main-cache-pairs: marker mismatch");
    assert!(
        values[1] > 10,
        "BOOT-04 main-cache-pairs: parse count が小さすぎる: {:?}",
        values
    );
    assert!(
        values[2] > 10,
        "BOOT-04 main-cache-pairs: pair count が小さすぎる: {:?}",
        values
    );
    assert_eq!(
        values[3], 4,
        "BOOT-04 main-cache-pairs: entry decl count が期待と異なる"
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reports_main_again_cache_pairs_progress() {
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
    .expect("BOOT-04 main-cache-pairs-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let progress_output = run_wasm_with_eleven_imports_compiler_mode_fs_printed_first(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "pairs-progress",
        ],
    );
    eprintln!(
        "BOOT-04 main-cache-pairs-progress output = {:?}",
        progress_output
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reports_one_import_path_resolution() {
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
    .expect("BOOT-04 one-import-path-debug: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-boot04-one-import-path-debug-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("one-import-path-debug temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.CompilerMode)\n(defn main [] 0)\n",
    )
    .expect("one-import-path-debug Main.ls を書けない");

    let debug_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &temp_root,
        &["compiler", "src/App/Main.ls", "debug", "paths"],
    )
    .expect("BOOT-04 one-import-path-debug: path debug 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 one-import-path-debug: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 one-import path values = {:?}", values);

    std::fs::remove_dir_all(&temp_root).expect("one-import-path-debug temp dir を削除できない");
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reports_main_again_progress() {
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
    .expect("BOOT-04 main-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let progress_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "debug",
            "progress",
            "main-again",
        ],
    );
    eprintln!("BOOT-04 main-progress output = {:?}", progress_output);
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reports_main_again_build_progress() {
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
    .expect("BOOT-04 main-build-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let progress_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "debug",
            "progress",
            "build",
            "main-again",
        ],
    )
    .expect("BOOT-04 main-build-progress: stage2_self_compiler の build progress 実行に失敗した");
    let values: Vec<i64> = progress_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 main-build-progress: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 main-build-progress values = {:?}", values);

    assert!(
        values.len() >= 36,
        "BOOT-04 main-build-progress: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[0], 50,
        "BOOT-04 main-build-progress: 最初の marker は 50 であるべき"
    );
    assert_eq!(
        values[3], 51,
        "BOOT-04 main-build-progress: header marker 51 が続くべき"
    );
    assert!(
        values[1] > 1000,
        "BOOT-04 main-build-progress: function count が小さすぎる: {:?}",
        values
    );
    assert!(
        values[2] > 0,
        "BOOT-04 main-build-progress: data length が正であるべき: {:?}",
        values
    );
    assert!(
        values[4] > 0,
        "BOOT-04 main-build-progress: header length が正であるべき: {:?}",
        values
    );
    ordered_marker_positions(
        &values,
        &(50..=67).collect::<Vec<_>>(),
        "BOOT-04 main-build-progress: marker sequence が崩れている",
    );
    let last_marker_index = values
        .iter()
        .rposition(|value| *value == 67)
        .expect("BOOT-04 main-build-progress: final marker 67 が見つからない");
    assert_eq!(
        last_marker_index + 2,
        values.len(),
        "BOOT-04 main-build-progress: final marker の後には wasm size だけが続くべき"
    );
    assert_eq!(
        values[last_marker_index + 1],
        values[last_marker_index - 1],
        "BOOT-04 main-build-progress: final wasm size は data append 後と一致するべき"
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reaches_main_again_build_phase_markers() {
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
    .expect("BOOT-04 main-build-phase: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let phase_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/Main.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-phase",
        ],
    )
    .expect("BOOT-04 main-build-phase: stage2_self_compiler の build phase 実行に失敗した");
    let values: Vec<i64> = phase_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 main-build-phase: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();

    assert!(
        values.len() >= 24,
        "BOOT-04 main-build-phase: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[0], 101,
        "BOOT-04 main-build-phase: 最初の marker は 101 であるべき"
    );
    assert_eq!(
        values[1], 102,
        "BOOT-04 main-build-phase: compile 完了 marker 102 が続くべき"
    );
    assert_eq!(
        values[3], 104,
        "BOOT-04 main-build-phase: parse-count marker 104 が続くべき"
    );
    assert!(
        values[2] > 1000,
        "BOOT-04 main-build-phase: function count が小さすぎる: {:?}",
        values
    );
    assert!(
        values[4] > 10,
        "BOOT-04 main-build-phase: parse count が小さすぎる: {:?}",
        values
    );
    ordered_marker_positions(
        &values[5..],
        &(50..=66).collect::<Vec<_>>(),
        "BOOT-04 main-build-phase: build marker sequence が崩れている",
    );
    let last_marker_index = values
        .iter()
        .rposition(|value| *value == 103)
        .expect("BOOT-04 main-build-phase: final marker 103 が見つからない");
    assert_eq!(
        last_marker_index + 2,
        values.len(),
        "BOOT-04 main-build-phase: final marker の後には wasm size だけが続くべき"
    );
    assert!(
        values[last_marker_index + 1] > 0,
        "BOOT-04 main-build-phase: final wasm size が正であるべき: {:?}",
        values
    );
}
