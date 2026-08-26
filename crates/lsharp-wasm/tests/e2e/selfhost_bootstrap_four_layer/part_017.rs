// =============================================================================
// I-71: 空の do ブロックが値を残さず stage2 Wasm を型不正にする回帰テスト
//
// selfhost コンパイラは `(do)` (expr-count = 0) を「何も emit しない」で扱う。
// Rust host 側の参照実装 (crates/lsharp-ir/src/lower/expr/do_expr.rs) は
// `I64Const(0)` を unit として emit するので、両者は食い違う。
// `(if cond (do ...) (do))` は blockty = i64 の if を作るため、
// else 腕が空だと "expected i64 but nothing on stack" で validation に落ちる。
//
// do の compile 経路は 2 つあり、**別々に pin する**:
//   1. 非 source 経路 -- Compiler.ls の tag 9 dispatch → compile-do-exprs
//   2. source 付き経路 -- compile-do-with-source (compiler モードが通る方)
//
// 検証には `validate_wasm_function_bodies` を使う。`assert_valid_wasm` は
// マジックバイトしか見ず、`validate_wasm_detailed` は ValidPayload::Func を
// 捨てるため関数本体を一つも検証しない。どちらもこの不正を素通りさせる。
// =============================================================================

/// I-71: 非 source 経路 (tag 9 dispatch → compile-do-exprs) の空 do
#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_emits_valid_stage2_wasm_for_empty_do_block() {
    let stage2_src = r#"(defn pick [c] (if (> c 0) (do 42) (do))) (defn main [] (pick 1))"#;
    let harness = {
        let mut s = RUNTIME_STAGE2_HARNESS_PRELUDE.to_string();
        s.push_str("\n(defn main []\n  (let [stage2 (bootstrap-build-stage2 \"");
        s.push_str(stage2_src);
        s.push_str("\")]\n    (do\n      (bootstrap-print-module stage2)\n      0)))");
        s
    };
    let stage1_source = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let stage1_wasm = compile_only(&stage1_source);
    assert_valid_wasm(&stage1_wasm);

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm)
        .expect("空 do を含む stage1 wasm の実行に失敗");
    let modules = parse_emitted_wasm_modules(&output, 1);
    assert_eq!(modules.len(), 1, "stage2 モジュール数が不正");
    assert_valid_wasm(&modules[0]);
    validate_wasm_function_bodies(&modules[0]).unwrap_or_else(|error| {
        panic!("I-71: 空 do を含む stage2 の関数本体が型不正: {error}")
    });
    assert_eq!(
        run_exported_i64_with_runtime_imports(&modules[0], "_start"),
        42,
        "空 do の else 腕は unit を残し、then 腕の 42 が返ること"
    );
}

/// I-71: source 付き経路 (compile-do-with-source) の空 do
///
/// compiler モードの self-compile が通る経路。sweep の 72 件はこちら側で落ちている。
#[test]
#[ignore]
fn test_e2e_bootstrap_compiler_mode_emits_valid_stage2_wasm_for_empty_do_block() {
    let main_path = selfhost_main_path();
    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-i71-empty-do-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("時刻が巻き戻った")
            .as_nanos()
    ));
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("I-71 temp dir を作れない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(defn pick [c] (if (> c 0) (do 42) (do)))\n(defn main [] (print (pick 1)))\n",
    )
    .expect("I-71 Main.ls を書けない");

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&temp_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("I-71: stage1 が空 do を含む package の compile に失敗した");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    assert_eq!(stage2_modules.len(), 1, "stage2 モジュール数が不正");
    let stage2 = &stage2_modules[0];
    assert_valid_wasm(stage2);
    validate_wasm_function_bodies(stage2).unwrap_or_else(|error| {
        panic!("I-71: compiler モードの stage2 の関数本体が型不正: {error}")
    });

    let printed = run_wasm_with_eleven_imports_compiler_mode(stage2, "", &[])
        .expect("I-71: stage2 の実行に失敗");
    assert!(
        printed.lines().any(|line| line.trim() == "42"),
        "空 do の else 腕を持つ stage2 は 42 を print すること: {printed:?}"
    );

    std::fs::remove_dir_all(&temp_root).expect("I-71 temp dir を削除できない");
}
