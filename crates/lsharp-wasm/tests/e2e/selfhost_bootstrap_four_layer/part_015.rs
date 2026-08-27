
#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiles_step512_progress_harness() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let diagnostic_rel_path = "src/Tools/Test/Stage2LexerStep512Progress.ls";
    let diagnostic_abs_path = selfhost_root.join(diagnostic_rel_path);

    assert!(
        diagnostic_abs_path.exists(),
        "診断ハーネス {} が存在しない",
        diagnostic_abs_path.display()
    );

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 step512-compile: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", diagnostic_rel_path],
    )
    .expect("BOOT-04 step512-compile: stage2 が診断ハーネスをコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_compiler_runtime_resolves_param_and_user_call() {
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
    .expect("BOOT-04 runtime-lookup: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_source = r#"
(defn helper [x] x)
(defn main []
  (print (helper 7)))
"#;
    let stage3_output = run_wasm_with_eleven_imports_compiler_mode(
        stage2_self_compiler,
        stage3_source,
        &["compiler", "inline-runtime-lookup.ls"],
    )
    .expect("BOOT-04 runtime-lookup: stage2 が inline source をコンパイルできない");
    let stage3_modules = parse_emitted_wasm_modules(&stage3_output, 1);
    let stage3_wasm = &stage3_modules[0];
    assert_valid_wasm(stage3_wasm);

    let printed = run_wasm_with_eleven_imports_compiler_mode(stage3_wasm, "", &[])
        .expect("BOOT-04 runtime-lookup: stage3 inline wasm の実行に失敗");
    assert_eq!(
        printed, "7\n",
        "stage2 compiler runtime は param/local lookup と user call lookup を保持すること"
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_self_hosted_stage2_reports_step512_progress() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let diagnostic_rel_path = "src/Tools/Test/Stage2LexerStep512Progress.ls";
    let diagnostic_abs_path = selfhost_root.join(diagnostic_rel_path);

    assert!(
        diagnostic_abs_path.exists(),
        "診断ハーネス {} が存在しない",
        diagnostic_abs_path.display()
    );

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("BOOT-04 step512 diag: stage1 が Main.ls の self-compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let stage3_result = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &["compiler", diagnostic_rel_path],
    );

    match &stage3_result {
        Ok(stage3_output) => {
            // SUCCESS: stage2 が診断ハーネスをコンパイルできた
            let stage3_modules = parse_emitted_wasm_modules(stage3_output, 1);
            let stage3_wasm = &stage3_modules[0];
            assert_valid_wasm(stage3_wasm);

            match validate_wasm_detailed(stage3_wasm) {
                Err(validate_err) => {
                    eprintln!(
                        "BOOT-04 step512 diag ADVANCED: stage3 diagnostic wasm validation failed: {}",
                        validate_err
                    );
                    assert!(
                        validate_err.contains("values remaining on stack at end of block"),
                        "step512 stage3 validation 失敗モードが変わった可能性: {}",
                        validate_err
                    );
                }
                Ok(()) => match run_wasm_with_eleven_imports_compiler_mode(stage3_wasm, "", &[]) {
                    Ok(run_output) => {
                        let values = run_output
                            .lines()
                            .filter(|line| !line.trim().is_empty())
                            .map(|line| {
                                line.trim().parse::<i64>().unwrap_or_else(|err| {
                                    panic!("step512 診断出力が整数でない: {line:?} / {err}")
                                })
                            })
                            .collect::<Vec<_>>();

                        eprintln!("BOOT-04 step512 diag SUCCESS: {:?}", values);

                        assert!(
                            values.len() == 4 || values.len() == 7,
                            "step512 診断出力は 4 行または 7 行であるべき: {:?}",
                            values
                        );

                        let source_len = values[0];
                        let done1 = values[1];
                        let next1 = values[2];
                        let count1 = values[3];

                        assert!(source_len > 0, "step512 診断入力長が 0");
                        assert!(
                            next1 > 0 && next1 <= source_len,
                            "step1 next が範囲外: {:?}",
                            values
                        );
                        assert!(count1 > 0, "step1 token count が 0: {:?}", values);

                        if done1 == 0 {
                            assert_eq!(
                                values.len(),
                                7,
                                "step1 未完了なら step2 出力が必要: {:?}",
                                values
                            );
                            let next2 = values[5];
                            let count2 = values[6];
                            assert!(
                                next2 > next1 && next2 <= source_len,
                                "step2 next が前進していない: {:?}",
                                values
                            );
                            assert!(
                                count2 > count1,
                                "step2 token count が増えていない: {:?}",
                                values
                            );
                        } else {
                            assert_eq!(
                                values.len(),
                                4,
                                "step1 完了なら step2 出力は不要: {:?}",
                                values
                            );
                        }
                    }
                    Err(run_err) => {
                        // ADVANCED: stage2 compile は通ったので、次の narrow blocker は stage3 wasm の
                        // block stack-balance 崩れであることを診断として固定する。
                        let violations = local_bound_violations(stage3_wasm);
                        let _ = std::fs::write("/tmp/step512_progress_stage3.wasm", stage3_wasm);
                        eprintln!(
                            "BOOT-04 step512 diag ADVANCED: stage3 diagnostic wasm runtime/load failed: {}; full_error={}; sections={:?}; violations={:?}; fingerprint={}",
                            run_err.lines().next().unwrap_or(""),
                            run_err,
                            extract_sections(stage3_wasm),
                            violations,
                            hash_fingerprint(stage3_wasm)
                        );
                        assert!(
                            run_err.contains("values remaining on stack at end of block"),
                            "step512 stage3 実行失敗モードが変わった可能性: {}",
                            run_err
                        );
                    }
                },
            }
        }
        Err(compile_err) => {
            // BLOCKED: stage2 が Syntax.Lexer を含む診断ハーネスをコンパイルできない
            // wasm コールスタックの再帰限界を計測して文書化する
            let frame_count = compile_err
                .lines()
                .filter(|l| l.contains("wasm function"))
                .count();
            eprintln!(
                "BOOT-04 step512 diag BLOCKED: stage2 compile failed with {} wasm frames at overflow",
                frame_count
            );
            eprintln!(
                "BOOT-04 step512 diag BLOCKED: first error line: {}",
                compile_err.lines().next().unwrap_or("")
            );
            // stage2 の再帰1レベルあたり約 65 フレームを消費する。
            // Syntax.Lexer の classify-symbol は 12 段の nested-if を持ち、
            // 12 * 65 = 780 フレームが必要 >> wasmtime のデフォルト ~280 フレーム限界
            eprintln!(
                "BOOT-04 step512 diag THRESHOLD: stage2 wasm stack ~{} frames; \
                 ~{} recursion levels (each ~65 frames); \
                 Syntax.Lexer classify-symbol requires ~12 nested-if levels (~780 frames needed); \
                 fix = reduce stage2 expression recursion depth",
                frame_count,
                frame_count / 65
            );

            // 既知の失敗モード: wasm バックトレースを含む深い再帰スタックオーバーフロー
            assert!(
                compile_err.contains("wasm backtrace") || compile_err.contains("unreachable"),
                "step512 stage2 compile 失敗は wasm backtrace を含むべき (got: {})",
                compile_err.lines().next().unwrap_or("")
            );
            // フレーム数 ≥ 200 → 深い再帰であることを確認
            assert!(
                frame_count >= 200,
                "step512 stage2 overflow frame count が 200 未満 (got {}): 失敗モードが変わった可能性がある",
                frame_count
            );
        }
    }
}

/// BOOT-04: compiler-mode が Syntax.LexerCompat を含む selfhost probe を解決できること
#[test]
#[ignore]
fn test_e2e_boot04_compiler_mode_lexer_compat_import_resolution() {
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let probe_rel_path = "src/Tools/Test/LexerCompatImportProbe.ls";
    let probe_abs_path = selfhost_root.join(probe_rel_path);

    assert!(
        probe_abs_path.exists(),
        "compat probe {} が存在しない",
        probe_abs_path.display()
    );

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", probe_rel_path],
    )
    .expect("BOOT-04 lexer-compat-import: compiler-mode が compat probe をコンパイルできなかった");

    let modules = parse_emitted_wasm_modules(&output, 1);
    let result_wasm = &modules[0];
    assert_valid_wasm(result_wasm);

    let run_output = run_wasm_with_eleven_imports_compiler_mode(result_wasm, "", &[])
        .expect("BOOT-04 lexer-compat-import: 生成 wasm の実行に失敗した");
    let values = run_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim().parse::<i64>().unwrap_or_else(|err| {
                panic!("lexer-compat probe 出力が整数でない: {line:?} / {err}")
            })
        })
        .collect::<Vec<_>>();

    assert_eq!(
        values.len(),
        7,
        "lexer-compat probe 出力行数が不正: {:?}",
        values
    );
    assert!(
        values[0] >= 3,
        "legacy tokenize は少なくとも 3 要素以上を返すべき: {:?}",
        values
    );
    assert_eq!(&values[1..], &[5, 0, 1, 2, 42, 1]);
}

/// BOOT-04: compiler-mode が import 宣言を解決できること
///
/// stage1 (Rust bootstrap wasm) を compiler-mode で実行したとき、
/// (import ...) 宣言を持つファイルを正しく処理できることを検証する。
///
/// simple_main.ls: (import SimpleHelper) + (defn main [] (helper-value))
/// simple_helper.ls: (defn helper-value [] 42)
///
/// import 解決後:
/// - helper-value, main の両関数が ftable に登録される
/// - 生成 wasm は valid wasm
/// - _start → main → helper-value → 42 が正常実行される
#[test]
#[ignore]
fn test_e2e_boot04_compiler_mode_import_resolution() {
    let main_path = selfhost_main_path();
    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");

    assert!(
        fixture_dir.join("SimpleMain.ls").exists(),
        "fixture ファイル tests/fixtures/SimpleMain.ls が存在しない"
    );
    assert!(
        fixture_dir.join("SimpleHelper.ls").exists(),
        "fixture ファイル tests/fixtures/SimpleHelper.ls が存在しない"
    );

    // stage1 (Rust bootstrap) wasm
    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    // compiler-mode で SimpleMain.ls をコンパイル (import SimpleHelper を解決する必要あり)
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&fixture_dir),
        &["compiler", "SimpleMain.ls"],
    )
    .expect("BOOT-04 import-resolution: compiler-mode が SimpleMain.ls をコンパイルできなかった");

    // 出力が length-prefixed wasm バイト列であること
    let modules = parse_emitted_wasm_modules(&output, 1);
    let result_wasm = &modules[0];
    assert_valid_wasm(result_wasm);

    // 生成 wasm が正常実行できること (helper-value を呼び出す main が動く)
    // 11-import モデル: env.string-concat, env.substring も import される
    let run_result = run_wasm_with_eleven_imports_compiler_mode(result_wasm, "", &[]);
    assert!(
        run_result.is_ok(),
        "BOOT-04 import-resolution: 生成 wasm の WASI 実行に失敗: {:?}",
        run_result.err()
    );

    eprintln!(
        "BOOT-04 import-resolution GREEN: SimpleMain.ls + SimpleHelper → {} bytes の wasm を生成・実行 OK",
        result_wasm.len()
    );
}

/// BOOT-04: compiler-mode が manifest なし source root 配下の dotted import を解決できること
#[test]
#[ignore]
fn test_e2e_boot04_compiler_mode_dotted_import_resolution_from_src_root() {
    let main_path = selfhost_main_path();
    let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/hier-selfhost");

    assert!(
        fixture_dir.join("src/App/Main.ls").exists(),
        "fixture ファイル tests/fixtures/hier-selfhost/src/App/Main.ls が存在しない"
    );
    assert!(
        fixture_dir.join("src/Syntax/SimpleHelper.ls").exists(),
        "fixture ファイル tests/fixtures/hier-selfhost/src/Syntax/SimpleHelper.ls が存在しない"
    );

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&fixture_dir),
        &["compiler", "src/App/Main.ls"],
    )
    .expect(
        "BOOT-04 dotted-import-resolution: compiler-mode が src/App/Main.ls をコンパイルできなかった",
    );

    let modules = parse_emitted_wasm_modules(&output, 1);
    let result_wasm = &modules[0];
    assert_valid_wasm(result_wasm);

    let run_result = run_wasm_with_eleven_imports_compiler_mode(result_wasm, "", &[]);
    assert!(
        run_result.is_ok(),
        "BOOT-04 dotted-import-resolution: 生成 wasm の WASI 実行に失敗: {:?}",
        run_result.err()
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_compiler_mode_package_index_resolution() {
    let main_path = selfhost_main_path();
    let fixture_dir = std::env::temp_dir().join(format!(
        "lsharp_selfhost_package_index_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&fixture_dir);
    std::fs::create_dir_all(fixture_dir.join("src")).unwrap();
    std::fs::create_dir_all(fixture_dir.join(".lsharp/packages/demo-123/src")).unwrap();
    std::fs::create_dir_all(fixture_dir.join(".lsharp/module-index")).unwrap();

    std::fs::write(
        fixture_dir.join("src/Main.ls"),
        "(module Main)\n(import Geometry)\n(defn main [] (distance))",
    )
    .unwrap();
    std::fs::write(
        fixture_dir.join(".lsharp/packages/demo-123/src/Geometry.ls"),
        "(module Geometry)\n(defn distance [] 42)",
    )
    .unwrap();
    std::fs::write(
        fixture_dir.join(".lsharp/module-index/Geometry.path"),
        ".lsharp/packages/demo-123/src/Geometry.ls\n",
    )
    .unwrap();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&fixture_dir),
        &["compiler", "src/Main.ls"],
    )
    .expect(
        "BOOT-04 package-index-resolution: compiler-mode が src/Main.ls をコンパイルできなかった",
    );

    let modules = parse_emitted_wasm_modules(&output, 1);
    let result_wasm = &modules[0];
    assert_valid_wasm(result_wasm);

    let run_result = run_wasm_with_eleven_imports_compiler_mode(result_wasm, "", &[]);
    let _ = std::fs::remove_dir_all(&fixture_dir);
    assert!(
        run_result.is_ok(),
        "BOOT-04 package-index-resolution: 生成 wasm の WASI 実行に失敗: {:?}",
        run_result.err()
    );
}

#[test]
#[ignore]
fn test_e2e_boot04_compiler_mode_supports_twelve_arg_calls() {
    let main_path = selfhost_main_path();
    let fixture_dir =
        std::env::temp_dir().join(format!("lsharp_selfhost_many_args_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&fixture_dir);
    std::fs::create_dir_all(fixture_dir.join("src")).unwrap();
    std::fs::write(
        fixture_dir.join("src/Main.ls"),
        "(module Main)\n(defn pick-last [a b c d e f g h i j k l] (do (print l) l))\n(defn main [] (pick-last 1 2 3 4 5 6 7 8 9 10 11 12))",
    )
    .unwrap();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&fixture_dir),
        &["compiler", "src/Main.ls"],
    )
    .expect("BOOT-04 many-args: compiler-mode が src/Main.ls をコンパイルできなかった");

    let modules = parse_emitted_wasm_modules(&output, 1);
    let result_wasm = &modules[0];
    assert_valid_wasm(result_wasm);

    let run_result = run_wasm_with_eleven_imports_compiler_mode(result_wasm, "", &[]);
    let _ = std::fs::remove_dir_all(&fixture_dir);
    let run_output = run_result.expect("BOOT-04 many-args: 生成 wasm の WASI 実行に失敗した");
    assert_eq!(run_output, "12\n");
}

#[test]
#[ignore]
fn test_e2e_boot04_compiler_mode_ignores_dotted_flat_file() {
    let main_path = selfhost_main_path();
    let fixture_dir = std::env::temp_dir().join(format!(
        "lsharp_selfhost_dotted_flat_fallback_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&fixture_dir);
    std::fs::create_dir_all(fixture_dir.join("src/App")).unwrap();
    std::fs::create_dir_all(fixture_dir.join("src")).unwrap();

    std::fs::write(
        fixture_dir.join("src/App/Main.ls"),
        "(module App.Main)\n(import Syntax.Token)\n(defn main [] (print (token-tag)))",
    )
    .unwrap();
    std::fs::write(
        fixture_dir.join("src/Syntax.Token.ls"),
        "(module Syntax.Token)\n(defn token-tag [] 7)",
    )
    .unwrap();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let result = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&fixture_dir),
        &["compiler", "src/App/Main.ls"],
    );

    let _ = std::fs::remove_dir_all(&fixture_dir);
    // I-79: 実行失敗を黙って skip しない。skip すると assert_ne! ごと消え、
    // compiler-mode が何を module source に採ったかを一度も見ないまま緑になる。
    let output = result.unwrap_or_else(|e| {
        panic!("BOOT-04 dotted-flat-file: stage1 の compiler-mode 実行に失敗した: {e}")
    });
    let modules = parse_emitted_wasm_modules(&output, 1);
    let result_wasm = &modules[0];
    assert_valid_wasm(result_wasm);

    let run_output = run_wasm_with_eleven_imports_compiler_mode(result_wasm, "", &[])
        .unwrap_or_else(|e| {
            panic!("BOOT-04 dotted-flat-file: 生成 wasm の実行に失敗した: {e}")
        });
    assert_ne!(
        run_output, "7\n",
        "BOOT-04 dotted-flat-file: compiler-mode が src/Syntax.Token.ls を module source に採用している"
    );
}

#[test]
fn test_i64_if_condition_validity() {
    // if 条件は i32 でなければならないので、i64 を条件に使う TEST_I64_IF_WASM は不正である。
    // 同じバイト列に対する 3 つの検証手段の強度差をここで固定する:
    //
    //   validate_wasm_detailed        -- ValidPayload::Func を捨てるので見逃す (Ok)
    //   validate_wasm_function_bodies -- 関数本体を個別に検証するので捕捉する (Err)
    //   wasmtime::Module::new         -- 翻訳時に捕捉する (Err)
    //
    // この強度差が docs/adr/decisions-probe-subject-unchecked.md 裁定 5 の根拠である。
    // 裁定 5 はソース読解で導いたが、本 test はそれを実行可能な形で保持する。
    // 「緑になることと検査していることは別である」を検査する側の test なので、
    // 1 行目の Ok は「弱い helper が弱いままであること」の確認であって、望ましさの表明ではない。
    assert!(
        validate_wasm_detailed(TEST_I64_IF_WASM).is_ok(),
        "validate_wasm_detailed が関数本体の型不一致を捕捉するようになった。\
         裁定 5 の前提 (弱い helper を検査の引き取り先にしてはならない) を見直すこと: {:?}",
        validate_wasm_detailed(TEST_I64_IF_WASM)
    );

    let bodies_error = validate_wasm_function_bodies(TEST_I64_IF_WASM)
        .expect_err("validate_wasm_function_bodies は if 条件の i64 を捕捉するはず");
    assert!(
        bodies_error.contains("func[0]") && bodies_error.contains("expected i32, found i64"),
        "本体検証のエラーが関数番号と型不一致の両方を示していない: {bodies_error}"
    );

    let engine = wasmtime::Engine::default();
    let module_error = wasmtime::Module::new(&engine, TEST_I64_IF_WASM)
        .expect_err("wasmtime は i64 を if 条件に使う wasm を拒否するはず");
    let module_error = format!("{module_error:?}");
    assert!(
        module_error.contains("expected i32, found i64"),
        "wasmtime のエラーが型不一致を示していない: {module_error}"
    );
}

#[test]
#[ignore]
fn test_debug_stage2_save() {
    // stage2 を生成してファイルに保存する (デバッグ用)
    let main_path = selfhost_main_path();
    let stage1_wasm = compile_file_only(&main_path);
    let selfhost_dir = selfhost_package_root();
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_dir),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 failed");
    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage2 = &modules[0];
    std::fs::write("stage2_debug.wasm", stage2).expect("write failed");
    eprintln!("stage2_debug.wasm written ({} bytes)", stage2.len());
}

#[test]
fn test_parse_compiler_ls() {
    // selfhost のコード生成本体が Rust reference parser で構文エラー無く読めること。
    // decl 数は下限だけを固定する (実測 312 / 2026-08-27)。上限を固定すると
    // 関数を 1 つ足すたびに落ちる test になり、主題 (構文が壊れていないこと) から外れる。
    let source = std::fs::read_to_string(selfhost_source_path("Compiler.ls")).expect("read file");
    let program = lsharp_syntax::parse(&source)
        .unwrap_or_else(|error| panic!("Compiler.ls のパースに失敗した: {error:?}"));
    assert!(
        program.decls.len() >= 300,
        "Compiler.ls の decl 数が下限を割った: {} (実測 312 / 2026-08-27)",
        program.decls.len()
    );
}

#[test]
fn test_parse_caws_standalone() {
    // compile-apply-with-source を単独ファイルとしてパースできること。
    // fixture は深くネストした実コード (約 2.9KB を 1 行) なので、
    // ネストの深さや長さで parser が壊れていないことの回帰ガードになる。
    //
    // 2026-08-27 の実測でこの fixture は offset 1795 の
    // `(if (> arg-count 0) (do ...))` が else 節を欠いており、L# の if (3 引数) として
    // 不正だった。Rust parser が拒否したのは正しい挙動なので、fixture 側に `0` を補って
    // 修復した。selfhost parser は修復前の fixture を `diagnostics:0` で受理していた --
    // この乖離は ISSUES.md の I-86。
    let source = std::fs::read_to_string(
        selfhost_project_root().join("tests/fixtures/selfhost-debug/test_caws.ls"),
    )
    .expect("read file");
    let program = lsharp_syntax::parse(&source)
        .unwrap_or_else(|error| panic!("test_caws.ls のパースに失敗した: {error:?}"));
    assert_eq!(
        program.decls.len(),
        2,
        "fixture は (module TestCAWS) と defn の 2 decl から成る"
    );
}

#[test]
#[ignore]
fn test_debug_stage2_output_minimal() {
    // stage2 が minimal.ls をコンパイルした出力を保存・検証する
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let stage1_wasm = compile_file_only(&main_path);

    // stage1 で src/App/Main.ls をコンパイル → stage2
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 failed");
    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage2 = &modules[0];
    std::fs::write("stage2_debug2.wasm", stage2).expect("write failed");
    eprintln!("stage2 written ({} bytes)", stage2.len());

    // stage2 で minimal.ls をコンパイル
    let fixture_dir =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let stage3_result = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        stage2,
        Some(&fixture_dir),
        &["compiler", "minimal.ls"],
    );
    match stage3_result {
        Err(e) => eprintln!("stage2->minimal failed: {}", e),
        Ok(out) => {
            let modules3 = parse_emitted_wasm_modules(&out, 1);
            let stage3 = &modules3[0];
            std::fs::write("stage3_minimal.wasm", stage3).expect("write failed");
            eprintln!("stage3 written ({} bytes)", stage3.len());
        }
    }
}

#[test]
#[ignore]
fn test_validate_stage2_wasm() {
    // stage2 を詳細バリデーション
    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let stage1_wasm = compile_file_only(&main_path);
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 failed");
    let modules = parse_emitted_wasm_modules(&output, 1);
    let stage2 = &modules[0];
    match validate_wasm_detailed(stage2) {
        Ok(_) => eprintln!("stage2 詳細バリデーション PASSED ({} bytes)", stage2.len()),
        Err(e) => eprintln!("stage2 詳細バリデーション FAILED: {}", e),
    }
}
