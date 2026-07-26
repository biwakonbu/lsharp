
#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reaches_main_again_build_compile_progress_markers() {
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
    .expect("BOOT-04 main-build-compile-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let progress_output = run_wasm_with_six_imports_compiler_mode_fs(
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
            "",
            "build-compile-progress",
        ],
    )
    .expect(
        "BOOT-04 main-build-compile-progress: stage2_self_compiler の build compile progress 実行に失敗した",
    );
    let values: Vec<i64> = progress_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 main-build-compile-progress: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();

    assert!(
        values.len() >= 8,
        "BOOT-04 main-build-compile-progress: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[0], 111,
        "BOOT-04 main-build-compile-progress: 最初の marker は 111 であるべき"
    );
    assert_eq!(
        values[1], 112,
        "BOOT-04 main-build-compile-progress: register 後 marker 112 が続くべき"
    );
    assert!(
        values[2] > 0,
        "BOOT-04 main-build-compile-progress: register pair 数が正であるべき: {:?}",
        values
    );
    assert!(
        values.contains(&29),
        "BOOT-04 main-build-compile-progress: pair progress marker 29 が必要: {:?}",
        values
    );
    assert!(
        values.contains(&40),
        "BOOT-04 main-build-compile-progress: defn progress marker 40 が必要: {:?}",
        values
    );
    let last_marker_index = values
        .iter()
        .rposition(|value| *value == 113)
        .expect("BOOT-04 main-build-compile-progress: final marker 113 が見つからない");
    assert_eq!(
        last_marker_index + 2,
        values.len(),
        "BOOT-04 main-build-compile-progress: final marker の後には function count だけが続くべき"
    );
    assert!(
        values[last_marker_index + 1] > 1000,
        "BOOT-04 main-build-compile-progress: function count が小さすぎる: {:?}",
        values
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_warm_target_defn_parity_reaches_ast_make_type_constrained() {
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
    .expect("BOOT-04 warm-target-defn: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let parity_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/Syntax/AST.ls",
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
            "",
            "",
            "",
            "warm-target-defn",
        ],
    )
    .expect("BOOT-04 warm-target-defn: stage2_self_compiler の parity probe 実行に失敗した");
    let values: Vec<i64> = parity_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 warm-target-defn: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();

    assert!(
        values.len() >= 10,
        "BOOT-04 warm-target-defn: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[0], 141,
        "BOOT-04 warm-target-defn: warm-up 完了 marker 141 から始まるべき"
    );
    assert_eq!(
        values[2], 142,
        "BOOT-04 warm-target-defn: data length marker 142 が続くべき"
    );
    assert_eq!(
        values[4], 124,
        "BOOT-04 warm-target-defn: target decl tag marker 124 が必要"
    );
    assert_eq!(
        values[5], 20,
        "BOOT-04 warm-target-defn: target decl は defn であるべき"
    );
    assert_eq!(
        values[6], 123,
        "BOOT-04 warm-target-defn: ftable IR marker 123 が必要"
    );
    assert!(
        values[7] > 0,
        "BOOT-04 warm-target-defn: ftable IR は空であってはいけない: {:?}",
        values
    );
    assert_eq!(
        values[8], 144,
        "BOOT-04 warm-target-defn: source-aware function-meta marker 144 が必要"
    );
    assert!(
        values[9] > 0,
        "BOOT-04 warm-target-defn: source-aware IR は空であってはいけない: {:?}",
        values
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_target_defn_parity_reaches_ast_make_type_constrained() {
    fn marker_value(values: &[i64], marker: i64) -> i64 {
        assert_eq!(
            values.len() % 2,
            0,
            "BOOT-04 target-defn: marker/value ペア数が崩れている: {:?}",
            values
        );
        values
            .chunks_exact(2)
            .find_map(|chunk| (chunk[0] == marker).then_some(chunk[1]))
            .unwrap_or_else(|| {
                panic!("BOOT-04 target-defn: marker {marker} が見つからない: {values:?}")
            })
    }

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
    .expect("BOOT-04 target-defn: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let parity_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/Syntax/AST.ls",
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
            "",
            "",
            "target-defn",
        ],
    )
    .expect("BOOT-04 target-defn: stage2_self_compiler の parity probe 実行に失敗した");
    let values: Vec<i64> = parity_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("BOOT-04 target-defn: 数値でない debug 出力: {line:?}"))
        })
        .collect();
    eprintln!("BOOT-04 target-defn values = {:?}", values);

    assert!(
        values.len() >= 8,
        "BOOT-04 target-defn: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(marker_value(&values, 121), 59);
    assert_eq!(marker_value(&values, 124), 20);
    assert!(
        marker_value(&values, 125) > 0,
        "BOOT-04 target-defn: param-count は正であるべき: {:?}",
        values
    );
    assert!(
        marker_value(&values, 126) > 0,
        "BOOT-04 target-defn: body tag は正であるべき: {:?}",
        values
    );
    assert_eq!(marker_value(&values, 127), 5);
    assert_eq!(marker_value(&values, 128), 4);
    assert_eq!(
        marker_value(&values, 129),
        marker_value(&values, 131),
        "BOOT-04 target-defn: use-site と def-site の hash は一致するべき: {:?}",
        values
    );
    assert!(
        marker_value(&values, 130) > 0,
        "BOOT-04 target-defn: use-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 132) > 0,
        "BOOT-04 target-defn: def-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 133) > 0,
        "BOOT-04 target-defn: local use-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 134) > 0,
        "BOOT-04 target-defn: local def-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 123) > 0,
        "BOOT-04 target-defn: ftable IR は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 122) > 0,
        "BOOT-04 target-defn: source-aware IR は空であってはいけない: {:?}",
        values
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_stage1_target_defn_parity_reports_ast_make_type_constrained_lengths() {
    fn marker_value(values: &[i64], marker: i64) -> i64 {
        assert_eq!(
            values.len() % 2,
            0,
            "BOOT-04 stage1 target-defn: marker/value ペア数が崩れている: {:?}",
            values
        );
        values
            .chunks_exact(2)
            .find_map(|chunk| (chunk[0] == marker).then_some(chunk[1]))
            .unwrap_or_else(|| {
                panic!("BOOT-04 stage1 target-defn: marker {marker} が見つからない: {values:?}")
            })
    }

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

    let parity_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            "src/Syntax/AST.ls",
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
            "",
            "",
            "target-defn",
        ],
    )
    .expect("BOOT-04 stage1 target-defn: stage1 parity probe 実行に失敗した");
    let values: Vec<i64> = parity_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 stage1 target-defn: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 stage1 target-defn values = {:?}", values);

    assert_eq!(marker_value(&values, 121), 59);
    assert_eq!(marker_value(&values, 124), 20);
    assert_eq!(marker_value(&values, 125), 1);
    assert_eq!(marker_value(&values, 126), 7);
    assert_eq!(marker_value(&values, 127), 5);
    assert_eq!(marker_value(&values, 128), 4);
    assert_eq!(marker_value(&values, 129), marker_value(&values, 131));
    assert!(
        marker_value(&values, 130) > 0,
        "stage1 use-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 132) > 0,
        "stage1 def-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 133) > 0,
        "stage1 local use-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 134) > 0,
        "stage1 local def-site lookup は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 123) > 0,
        "stage1 ftable IR は空であってはいけない: {:?}",
        values
    );
    assert!(
        marker_value(&values, 122) > 0,
        "stage1 source-aware IR は空であってはいけない: {:?}",
        values
    );
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_first_defn_probe_on_minimal_make_type_constrained_shape() {
    let temp_root =
        std::env::temp_dir().join(format!("lsharp_target_defn_minimal_{}", std::process::id()));
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_ast_shape.ls");
    std::fs::write(
        &source_path,
        "(defn make-type-constrained [name-hash] (let [v (vector-new 2)] (vector-push (vector-push v (ast-typeconstrained)) name-hash)))\n(defn ast-typeconstrained [] 24)\n(defn main [] 0)\n",
    )
    .expect("mini source should be written");

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
    let source_path_str = source_path.to_str().expect("utf-8 path");
    let mut source_step_args = vec!["compiler", source_path_str];
    while source_step_args.len() < 21 {
        source_step_args.push("");
    }
    source_step_args.push("first-defn-source-step");

    let stage1_probe_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &source_step_args,
    )
    .expect("stage1 first-defn probe on minimal source should run");
    eprintln!(
        "BOOT-04 minimal first-defn stage1 = {:?}",
        stage1_probe_output
    );

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let probe_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &source_step_args,
    )
    .expect("stage2 first-defn probe on minimal source should run");
    eprintln!("BOOT-04 minimal first-defn values = {:?}", probe_output);
    assert!(!probe_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_first_defn_ir_parity_on_minimal_demo_main_shape() {
    let temp_root =
        std::env::temp_dir().join(format!("lsharp_demo_main_minimal_{}", std::process::id()));
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_demo_main_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.Token)\n(defn demo-main [] (do (print (tok-lparen)) (print (tok-rparen)) (print (tok-eof)) 0))\n(defn tok-lparen [] 40)\n(defn tok-rparen [] 41)\n(defn tok-eof [] 99)\n",
    )
    .expect("mini source should be written");

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
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source_path_str = source_path.to_str().expect("utf-8 path");
    let probe_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            source_path_str,
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
            "",
            "",
            "",
            "",
            "first-defn-ir-parity",
        ],
    )
    .expect("stage2 first-defn-ir-parity probe on minimal demo-main source should run");
    eprintln!(
        "BOOT-04 minimal demo-main first-defn-ir-parity = {:?}",
        probe_output
    );
    assert!(!probe_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_first_defn_source_probe_on_minimal_text_eq_loop_shape() {
    let temp_root = std::env::temp_dir().join(format!(
        "lsharp_text_eq_loop_minimal_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_text_eq_loop_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.ModuleResolver)\n(defn text-eq-loop [left right idx len] (if (>= idx len) true (if (= (string-char-at left idx) (string-char-at right idx)) (text-eq-loop left right (+ idx 1) len) false)))\n(defn main [] 0)\n",
    )
    .expect("mini source should be written");

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
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source_path_str = source_path.to_str().expect("utf-8 path");
    let mut probe_args = vec!["compiler", source_path_str];
    while probe_args.len() < 22 {
        probe_args.push("");
    }
    probe_args.push("first-defn-source");

    let probe_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &probe_args,
    )
    .expect("stage2 first-defn source probe on minimal text-eq-loop source should run");
    eprintln!(
        "BOOT-04 minimal text-eq-loop source probe = {:?}",
        probe_output
    );
    assert!(!probe_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_first_defn_source_step_probe_on_minimal_path_parent_shape() {
    let temp_root = selfhost_project_root()
        .join("target/test-artifacts")
        .join(format!(
            "lsharp_path_parent_minimal_step_probe_{}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_path_parent_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.ModuleResolver)\n(defn path-parent [path] (let [len (string-length path)] (if (= len 0) \"\" (if (has-path-sep path 0 len) (let [last (find-last-path-sep path 0 len -1)] (if (< last 0) \"\" (if (= last 0) \"/\" (substring path 0 last)))) \".\"))))\n(defn path-char [path idx] (string-char-at path idx))\n(defn is-path-sep [path idx] (let [ch (path-char path idx)] (if (= ch 47) true (if (= ch 92) true false))))\n(defn has-path-sep [path idx len] (if (>= idx len) false (if (is-path-sep path idx) true (has-path-sep path (+ idx 1) len))))\n(defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))\n(defn main [] 0)\n",
    )
    .expect("mini source should be written");

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
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source_path_str = source_path.to_str().expect("utf-8 path");
    let mut probe_args = vec!["compiler", source_path_str];
    while probe_args.len() < 21 {
        probe_args.push("");
    }
    probe_args.push("first-defn-source-step");

    let probe_output = run_wasm_with_six_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &probe_args,
    )
    .expect("stage2 first-defn source step probe on minimal path-parent source should run");
    eprintln!(
        "BOOT-04 minimal path-parent source step probe = {:?}",
        probe_output
    );
    assert!(!probe_output.trim().is_empty());
}
