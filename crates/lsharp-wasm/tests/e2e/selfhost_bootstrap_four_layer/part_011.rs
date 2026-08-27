
#[test]
#[ignore]
fn test_e2e_boot04_stage2_first_defn_source_probe_emits_expected_plus_ir_on_minimal_text_eq_loop_shape()
 {
    let temp_root = selfhost_project_root()
        .join("target/test-artifacts")
        .join(format!(
            "lsharp_text_eq_loop_minimal_stage_compare_{}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&temp_root);
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

    let source_path_str = source_path.to_str().expect("utf-8 path");
    let mut probe_args = vec!["compiler", source_path_str];
    while probe_args.len() < 22 {
        probe_args.push("");
    }
    probe_args.push("first-defn-source");

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage2_probe_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &probe_args,
    )
    .expect("stage2 first-defn source probe on minimal text-eq-loop source should run");
    let values: Vec<i64> = stage2_probe_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 minimal text-eq-loop source probe: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    assert_eq!(
        values,
        vec![
            301, 1, 302, 6, 303, 5, 304, 3, 206, 0, 209, 2, 207, 10, 208, 3, 206, 1, 209, 2, 207,
            1, 208, 1, 206, 2, 209, 2, 207, 20, 208, 0,
        ],
        "minimal text-eq-loop source probe は (+ idx 1) を local-get / i64-const / i64-add に lower すべき: {:?}",
        values
    );
    std::fs::remove_dir_all(&temp_root).expect("repo-local temp dir should be removed");
}

#[test]
#[ignore]
fn test_debug_boot04_stage2_ast_chunked_step_progress_on_ast_file() {
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

    let mut args = vec!["compiler", "src/Syntax/AST.ls"];
    while args.len() < 20 {
        args.push("");
    }
    args.push("ast-chunked-step");

    let output =
        run_wasm_with_eleven_imports_compiler_mode_fs(stage2_self_compiler, &selfhost_root, &args)
            .expect("stage2 ast-chunked-step probe should run");
    eprintln!("BOOT-04 ast-chunked-step values = {:?}", output);
    let values = parse_progress_values(&output, "BOOT-04 ast-chunked-step");
    assert!(
        values.len() > 100,
        "BOOT-04 ast-chunked-step: 出力が短すぎる: {values:?}"
    );

    // 実測 (2026-08-27) の構造:
    //   150 <parse 回数> 151 <decl 数>
    //   decl ごとに 170 <序数> ... (decl 0 は (module ...) なので内側 marker を出さない)
    //   153 <生成関数の総数>
    assert_eq!(values[0], 150, "BOOT-04 ast-chunked-step: 先頭 marker は 150");
    assert_eq!(values[2], 151, "BOOT-04 ast-chunked-step: 3 番目の marker は 151");
    let parse_count = values[1];
    let decl_count = values[3];
    assert_eq!(
        parse_count, 2,
        "BOOT-04 ast-chunked-step: AST.ls は自身 + import の 2 回パースされるはず"
    );
    // AST.ls は実ソースなので decl 数は下限だけ (実測 45 / 2026-08-27)。
    assert!(
        decl_count >= 40,
        "BOOT-04 ast-chunked-step: AST.ls の decl 数が下限を割った: {decl_count}"
    );

    // 170 は decl 1 個につき 1 回、直後に 0 から始まる序数が来る。
    let step_ordinals: Vec<i64> = values
        .windows(2)
        .filter(|pair| pair[0] == 170)
        .map(|pair| pair[1])
        .collect();
    assert_eq!(
        step_ordinals,
        (0..decl_count).collect::<Vec<_>>(),
        "BOOT-04 ast-chunked-step: 170 の序数が 0..{decl_count} の連番になっていない"
    );

    // decl 0 は (module ...) で関数にならないため、内側 marker は decl 数 - 1 回。
    for marker in [172, 173, 174, 175, 176, 177, 180, 181, 182, 183, 184, 185] {
        assert_eq!(
            values.iter().filter(|value| **value == marker).count() as i64,
            decl_count - 1,
            "BOOT-04 ast-chunked-step: marker {marker} の出現回数が decl 数 - 1 でない"
        );
    }

    assert_eq!(
        values[values.len() - 2],
        153,
        "BOOT-04 ast-chunked-step: 終端 marker 153 が末尾から 2 番目に無い: {:?}",
        &values[values.len().saturating_sub(6)..]
    );
    assert_eq!(
        values[values.len() - 1],
        decl_count - 1,
        "BOOT-04 ast-chunked-step: 生成関数の総数が decl 数 - 1 でない"
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_cache_compile_progress_counts_all_main_modules() {
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
    .expect("BOOT-04 cache-compile-progress: stage1 が Main.ls の self-compile に失敗した");
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
            "",
            "",
            "",
            "cache-compile-progress",
        ],
    )
    .expect(
        "BOOT-04 cache-compile-progress: stage2_self_compiler の cache compile progress 実行に失敗した",
    );
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 cache-compile-progress: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 cache-compile-progress values = {:?}", values);

    assert_eq!(
        values.len(),
        8,
        "BOOT-04 cache-compile-progress: debug 出力長が期待と異なる: {:?}",
        values
    );
    assert_eq!(
        values[0], 86,
        "BOOT-04 cache-compile-progress: marker 86 が必要"
    );
    assert_eq!(
        values[2], 87,
        "BOOT-04 cache-compile-progress: marker 87 が必要"
    );
    assert_eq!(
        values[4], 88,
        "BOOT-04 cache-compile-progress: marker 88 が必要"
    );
    assert_eq!(
        values[6], 89,
        "BOOT-04 cache-compile-progress: marker 89 が必要"
    );

    let parse_count = values[1];
    let pair_count = values[3];
    let reg_count = values[5];
    let function_count = values[7];

    assert_eq!(
        parse_count, pair_count,
        "BOOT-04 cache-compile-progress: cache compile は Main graph の全 pair を一度ずつ parse するべき: {:?}",
        values
    );
    assert!(
        pair_count >= 26,
        "BOOT-04 cache-compile-progress: Main graph pair count が小さすぎる: {:?}",
        values
    );
    assert_eq!(
        reg_count, function_count,
        "BOOT-04 cache-compile-progress: register/compile 後の function count は一致するべき: {:?}",
        values
    );
    assert!(
        function_count >= 1531,
        "BOOT-04 cache-compile-progress: compiled function count が小さすぎる: {:?}",
        values
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_module_resolver_first_defn_with_source_matches_ftable_ir() {
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
    .expect("BOOT-04 first-defn-ir-parity: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let debug_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/ModuleResolver.ls",
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
    .expect("BOOT-04 first-defn-ir-parity: stage2_self_compiler の parity probe 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("BOOT-04 first-defn-ir-parity: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();
    eprintln!("BOOT-04 first-defn-ir-parity values = {:?}", values);

    assert!(
        values.len() >= 17,
        "BOOT-04 first-defn-ir-parity: raw/source/ftable marker 群を期待した: {:?}",
        values
    );
    assert_eq!(
        values[0], 91,
        "BOOT-04 first-defn-ir-parity: raw-source marker が崩れている: {:?}",
        values
    );
    assert!(
        values[1] >= 0,
        "BOOT-04 first-defn-ir-parity: defn index を見つけられていない: {:?}",
        values
    );
    assert_eq!(
        values[2], 92,
        "BOOT-04 first-defn-ir-parity: raw-source length marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[4], 93,
        "BOOT-04 first-defn-ir-parity: with-source pre-marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[5], 94,
        "BOOT-04 first-defn-ir-parity: with-source length marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[7], 95,
        "BOOT-04 first-defn-ir-parity: defn index replay marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[9], 96,
        "BOOT-04 first-defn-ir-parity: source IR marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[11], 97,
        "BOOT-04 first-defn-ir-parity: ftable IR marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[13], 98,
        "BOOT-04 first-defn-ir-parity: data marker が崩れている: {:?}",
        values
    );
    assert_eq!(
        values[15], 99,
        "BOOT-04 first-defn-ir-parity: raw-ftable marker が崩れている: {:?}",
        values
    );
    assert!(
        values[3] > 3 && values[6] > 3 && values[10] > 3,
        "BOOT-04 first-defn-ir-parity: source IR が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[1], values[8],
        "BOOT-04 first-defn-ir-parity: defn index replay が一致するべき: {:?}",
        values
    );
    assert_eq!(
        values[3], values[16],
        "BOOT-04 first-defn-ir-parity: raw source / raw ftable で IR 長が一致するべき: {:?}",
        values
    );
    assert_eq!(
        values[6], values[10],
        "BOOT-04 first-defn-ir-parity: with-source marker 94/96 の IR 長が一致するべき: {:?}",
        values
    );
    assert_eq!(
        values[10], values[12],
        "BOOT-04 first-defn-ir-parity: with-source / with-ftable で IR 長が一致するべき: {:?}",
        values
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reports_module_resolver_progress() {
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
    .expect("BOOT-04 module-resolver-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let progress_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            "src/App/ModuleResolver.ls",
            "debug",
            "progress",
            "module-resolver",
        ],
    )
    .expect("BOOT-04 module-resolver-progress: stage2 の progress probe が失敗した");

    // I-82 裁定 3: 従来は Result を print するだけだった。
    // 単一 module を回す経路なので import 数は 0 になり、生成関数の総数は decl 数 - 1 に一致する。
    // 出力長も decl 数から一意に決まる (pair ごとに 10 値 + 前後の固定 21 値)。
    // 実測 2026-08-27: 401 値、decl 数 39、src 12043 bytes。
    let values = parse_progress_values(&progress_output, "module-resolver-progress");
    let (import_count, last_decls, total_functions) =
        assert_debug_progress_shape(&values, "module-resolver-progress");
    assert_eq!(
        import_count, 0,
        "単一 module の progress なのに import 数が 0 でない: {import_count}"
    );
    assert_eq!(
        total_functions,
        last_decls - 1,
        "import が無いので生成関数の総数は decl 数 - 1 であるべき: {total_functions} / {last_decls}"
    );
    assert!(
        last_decls >= 30,
        "decl 数が少なすぎる ({last_decls}): selfhost の module を読めていない疑い"
    );
    assert_eq!(
        values.len() as i64,
        21 + 10 * (last_decls - 1),
        "出力長が decl 数 {last_decls} から決まる値と食い違う: {}",
        values.len()
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reports_string_length_if_progress() {
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
    .expect("BOOT-04 string-length-if-progress: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.ModuleResolver)\n(defn text-len-eq [left right] (let [len (string-length left)] (if (= len (string-length right)) 1 0)))\n(defn main [] (text-len-eq (command-line-arg 0) (command-line-arg 1)))\n";
    let progress_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &[
            "compiler",
            "src/App/ModuleResolver.ls",
            "debug",
            "progress",
            "inline",
        ],
    )
    .expect("BOOT-04 string-length-if-progress: stage2 の inline progress probe が失敗した");

    // I-82 裁定 3: 従来は Result を print するだけだった。
    // 入力が上の `source` リテラルなので、出力は 1 値も動く余地が無い。
    // ここは下限ではなく完全一致で固定する — 動いたら inline 経路の挙動が変わったということ。
    // 実測 2026-08-27: 41 値 (decl 数 3、src 203 bytes)。
    let values = parse_progress_values(&progress_output, "string-length-if-progress");
    assert_debug_progress_shape(&values, "string-length-if-progress");
    assert_eq!(
        values,
        vec![
            1, 3, 2, 0, 3, 2, 29, 0, 203, 3,
            40, 0, 25, 43, 0, 40, 1, 20, 41, 1,
            42, 1, 1, 43, 1, 40, 2, 20, 41, 2,
            42, 2, 2, 43, 2, 30, 0, 203, 3, 4,
            2,
        ],
        "inline source は固定なのに progress 出力が変わった"
    );
}

#[test]
#[ignore]
fn test_v2_12_self_hosted_stage2_keeps_complex_defn_decl_tag() {
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
    .expect("V2-12 complex-defn-tag: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = "(module App.Main)\n(defn helper [x] (if (= x 0) 0 (+ x 1)))\n";
    let debug_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        source,
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
            "expr-tag",
        ],
    )
    .expect("V2-12 complex-defn-tag: stage2_self_compiler の debug 実行に失敗した");
    let values: Vec<i64> = debug_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|_| {
                panic!("V2-12 complex-defn-tag: 数値でない debug 出力: {line:?}")
            })
        })
        .collect();

    assert!(
        values.len() >= 10,
        "V2-12 complex-defn-tag: debug 出力が短すぎる: {:?}",
        values
    );
    assert_eq!(
        values[0], 73,
        "V2-12 complex-defn-tag: expr-tag debug marker が期待と異なる: {:?}",
        values
    );
    assert_eq!(
        values[7], 20,
        "V2-12 complex-defn-tag: complex body を持つ defn も decl tag 20 を維持するべき: {:?}",
        values
    );
    assert_eq!(
        values[8], 1,
        "V2-12 complex-defn-tag: helper の引数数が期待と異なる: {:?}",
        values
    );
    assert_eq!(
        values[9], 6,
        "V2-12 complex-defn-tag: helper body は if tag を維持するべき: {:?}",
        values
    );
}

#[test]
#[ignore]
fn test_v2_12_self_hosted_stage2_emits_data_section_for_string_literals() {
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
    .expect("V2-12 string-data: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source = r#"
(module App.Main)
(defn main []
  (if (= 1 1)
    "hello"
    "world"))
"#;
    let stage3_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        source,
        &["compiler", "inline-string-data.ls"],
    )
    .expect("V2-12 string-data: stage2_self_compiler が inline string source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);

    let data_section = extract_section_bytes(stage3_wasm, 11)
        .expect("V2-12 string-data: string literal を含む stage3 wasm は data section を持つべき");
    let hello = b"hello";
    let world = b"world";
    assert!(
        data_section
            .windows(hello.len())
            .any(|window| window == hello),
        "V2-12 string-data: data section に hello bytes が見つからない: {:?}",
        &data_section[..data_section.len().min(64)]
    );
    assert!(
        data_section
            .windows(world.len())
            .any(|window| window == world),
        "V2-12 string-data: data section に world bytes が見つからない: {:?}",
        &data_section[..data_section.len().min(64)]
    );
}

#[test]
#[ignore]
fn test_v2_12_self_hosted_stage2_keeps_if_and_string_expr_tags() {
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
    let source = "(module App.Main)\n(defn main [] (if (= 1 1) \"hello\" \"world\"))\n";
    let expr_tag_args = [
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
        "expr-tag",
    ];

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("V2-12 expr-tag: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let printed =
        run_wasm_with_eleven_imports_compiler_mode(stage2_self_compiler, source, &expr_tag_args)
            .expect("V2-12 expr-tag: stage2_self_compiler の expr-tag 実行に失敗した");
    let values: Vec<i64> = printed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("V2-12 expr-tag: 数値でない診断出力: {line:?}"))
        })
        .collect();

    assert_eq!(
        values,
        vec![73, 0, 32, 12, 12, 6, 6, 20, 0, 6, 3, 3],
        "V2-12 expr-tag: stage2 parser は defn/main-if/string tags を保つべき"
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_module_import_file() {
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
    .expect("BOOT-04 module-import: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let module_import_source = "(module App.Main)\n(import App.CompilerMode)\n(defn main [] 0)\n";
    let stage3_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        module_import_source,
        &["compiler", "src/App/Main.ls"],
    )
    .expect(
        "BOOT-04 module-import: stage2_self_compiler が module+import source をコンパイルできない",
    );
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
    validate_wasm_detailed(stage3_wasm)
        .unwrap_or_else(|e| panic!("BOOT-04 module-import: stage3 wasm validation failed: {e}"));
}
