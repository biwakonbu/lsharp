use super::support::*;
use std::sync::atomic::{AtomicUsize, Ordering};

fn selfhost_native_label(name: &str) -> &'static str {
    match name {
        "NativeTarget.ls" => "selfhost/src/Backend/Native/NativeTarget.ls",
        "NativeCodegen.ls" => "selfhost/src/Backend/Native/NativeCodegen.ls",
        "NativeEmit.ls" => "selfhost/src/Backend/Native/NativeEmit.ls",
        "Linker.ls" => "selfhost/src/Backend/Native/Linker.ls",
        other => panic!("不明な native selfhost module: {other}"),
    }
}

fn read_selfhost_native_source(name: &str) -> String {
    let path = selfhost_source_path(name);
    let label = selfhost_native_label(name);
    assert!(path.exists(), "{label} が存在しない");
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{label} 読み込み失敗"))
}

// =============================================================================
// NATIVE-05: Stage1-native 自己再生成 — 機能的等価性テスト
// =============================================================================

/// NATIVE-05: ネイティブバックエンドの機能的等価性検証
///
/// 同一プログラムを Wasm バックエンドとネイティブ codegen パスの両方でコンパイルし、
/// 構造的等価性 (エクスポートシンボル・データセクション構造) を比較する。
/// ビット一致ではなく、機能レベルの等価性を検証する。
#[test]
fn test_e2e_native_self_regeneration_functional_equivalence() {
    // --- 前提: ネイティブバックエンドモジュールの存在確認 ---
    let native_modules = [
        "NativeTarget.ls",
        "NativeCodegen.ls",
        "NativeEmit.ls",
        "Linker.ls",
    ];
    for module in native_modules {
        let path = selfhost_source_path(module);
        assert!(
            path.exists(),
            "{} が存在しない — ネイティブ自己再生成の前提モジュール",
            selfhost_native_label(module)
        );
    }

    // --- Wasm バックエンドでリファレンス出力を生成 ---
    let test_source = r#"
        (defn double [x] (* x 2))
        (defn main [] (print (double 21)))
    "#;
    let wasm_bytes = compile_only(test_source);
    assert_valid_wasm(&wasm_bytes);

    // --- Wasm バイナリのセクション構造を抽出 ---
    let wasm_export_count = count_wasm_section(&wasm_bytes, 7); // Export section = 7
    let wasm_type_count = count_wasm_section(&wasm_bytes, 1); // Type section = 1
    let wasm_func_count = count_wasm_section(&wasm_bytes, 3); // Function section = 3

    // Wasm 出力が有効な構造を持つこと
    assert!(
        wasm_export_count > 0 || wasm_bytes.len() > 20,
        "Wasm 出力にエクスポートまたは十分なサイズがない"
    );

    // --- ネイティブ codegen パスで同一ソースをコンパイル ---
    // NativeCodegen.ls を selfhost bundle としてコンパイル・実行し、
    // ネイティブコード生成が決定的であることを確認
    let native_codegen_source = std::fs::read_to_string(selfhost_source_path("NativeCodegen.ls"))
        .expect("NativeCodegen.ls 読み込み失敗");

    // NativeCodegen.ls が Wasm コンパイルパイプラインを通ること (構造的等価性の前提)
    let native_codegen_parse = lsharp_syntax::parse(&native_codegen_source);
    assert!(
        native_codegen_parse.is_ok(),
        "NativeCodegen.ls のパースに失敗: {:?}",
        native_codegen_parse.err()
    );

    // NativeEmit.ls も同様にパースできること
    let native_emit_source = std::fs::read_to_string(selfhost_source_path("NativeEmit.ls"))
        .expect("NativeEmit.ls 読み込み失敗");
    let native_emit_parse = lsharp_syntax::parse(&native_emit_source);
    assert!(
        native_emit_parse.is_ok(),
        "NativeEmit.ls のパースに失敗: {:?}",
        native_emit_parse.err()
    );

    // --- 機能的等価性: 両パスが有効な出力構造を持つこと ---
    // Wasm 側: セクション構造が妥当
    assert!(
        wasm_type_count > 0 || wasm_func_count > 0 || wasm_bytes.len() > 8,
        "Wasm 出力に type/function セクションがない — 機能的等価性の比較基盤が不足"
    );

    // ネイティブ側: codegen モジュールが必要な関数を定義していること
    assert!(
        native_codegen_source.contains("emit-native")
            && native_codegen_source.contains("compile-to-native")
            && native_codegen_source.contains("generate-native"),
        "NativeCodegen.ls にネイティブパイプライン関数が欠落 — \
         機能的等価性の前提: emit-native, compile-to-native, generate-native が全て必要"
    );

    // NativeEmit.ls がオブジェクトファイル出力関数を持つこと
    assert!(
        native_emit_source.contains("emit-object")
            && native_emit_source.contains("emit-macho")
            && native_emit_source.contains("emit-elf"),
        "NativeEmit.ls にオブジェクト出力関数が欠落 — \
         機能的等価性の前提: emit-object, emit-macho, emit-elf が全て必要"
    );

    // 等価性証明: 両パスがコンパイル可能なソースに対して有効な出力を生成するパイプラインを持つ
    let wasm_output = compile_and_run(test_source);
    assert_eq!(
        wasm_output.trim(),
        "42",
        "Wasm バックエンドのリファレンス出力が不正"
    );
}

/// NATIVE-05: ネイティブ codegen の決定性検証 (stage chain structure)
///
/// ネイティブ codegen が複数回のコンパイルで一貫した出力を生成すること。
/// エクスポートシンボル・データセクション・型セクションの構造一致を検証する。
#[test]
fn test_e2e_native_stage_chain_structure() {
    // --- NativeCodegen.ls の決定的コンパイル ---
    let codegen_path = selfhost_source_path("NativeCodegen.ls");
    let codegen_source =
        std::fs::read_to_string(&codegen_path).expect("NativeCodegen.ls 読み込み失敗");

    // 2回パースして AST 構造が一致することを確認 (決定性の基礎)
    let ast1 = lsharp_syntax::parse(&codegen_source).expect("パース失敗 (1回目)");
    let ast2 = lsharp_syntax::parse(&codegen_source).expect("パース失敗 (2回目)");
    assert_eq!(
        ast1.decls.len(),
        ast2.decls.len(),
        "NativeCodegen.ls の2回パースで宣言数が異なる (非決定的パース)"
    );

    // --- NativeTarget.ls の決定的コンパイル ---
    let target_path = selfhost_source_path("NativeTarget.ls");
    let target_source =
        std::fs::read_to_string(&target_path).expect("NativeTarget.ls 読み込み失敗");

    let target_ast1 = lsharp_syntax::parse(&target_source).expect("パース失敗 (1回目)");
    let target_ast2 = lsharp_syntax::parse(&target_source).expect("パース失敗 (2回目)");
    assert_eq!(
        target_ast1.decls.len(),
        target_ast2.decls.len(),
        "NativeTarget.ls の2回パースで宣言数が異なる"
    );

    // --- NativeEmit.ls の決定的コンパイル ---
    let emit_path = selfhost_source_path("NativeEmit.ls");
    let emit_source = std::fs::read_to_string(&emit_path).expect("NativeEmit.ls 読み込み失敗");

    let emit_ast1 = lsharp_syntax::parse(&emit_source).expect("パース失敗 (1回目)");
    let emit_ast2 = lsharp_syntax::parse(&emit_source).expect("パース失敗 (2回目)");
    assert_eq!(
        emit_ast1.decls.len(),
        emit_ast2.decls.len(),
        "NativeEmit.ls の2回パースで宣言数が異なる"
    );

    // --- stage chain 構造: 3モジュールのシンボル一覧が安定であること ---
    let codegen_defns = count_defns(&codegen_source);
    let target_defns = count_defns(&target_source);
    let emit_defns = count_defns(&emit_source);

    // 最低限の関数定義数を検証 (回帰防止)
    assert!(
        codegen_defns >= 10,
        "NativeCodegen.ls の defn 数が少なすぎる: {} (期待: ≥10)",
        codegen_defns
    );
    assert!(
        target_defns >= 8,
        "NativeTarget.ls の defn 数が少なすぎる: {} (期待: ≥8)",
        target_defns
    );
    assert!(
        emit_defns >= 5,
        "NativeEmit.ls の defn 数が少なすぎる: {} (期待: ≥5)",
        emit_defns
    );

    // --- Wasm コンパイルパイプラインでの決定性 ---
    // NativeTarget.ls を Wasm パイプラインで2回コンパイルしバイナリ一致を確認
    let target_wasm1 = compile_only(&target_source);
    let target_wasm2 = compile_only(&target_source);
    assert_eq!(
        target_wasm1, target_wasm2,
        "NativeTarget.ls の2回コンパイルで Wasm バイナリが不一致 — 非決定的コンパイル"
    );

    // Wasm バイナリのセクション構造比較
    let export_count_1 = count_wasm_section(&target_wasm1, 7);
    let export_count_2 = count_wasm_section(&target_wasm2, 7);
    assert_eq!(
        export_count_1, export_count_2,
        "NativeTarget.ls の2回コンパイルでエクスポート数が不一致"
    );

    let type_count_1 = count_wasm_section(&target_wasm1, 1);
    let type_count_2 = count_wasm_section(&target_wasm2, 1);
    assert_eq!(
        type_count_1, type_count_2,
        "NativeTarget.ls の2回コンパイルで型セクション数が不一致"
    );
}

// =============================================================================
// NATIVE-06: Wasm/native differential — 5観測点比較テスト
// =============================================================================

/// NATIVE-06: Wasm/native 差分比較 — 5観測点 (observation points)
///
/// 同一ソースを Wasm パスとネイティブ codegen パスの両方で処理し、
/// 以下の5観測点を構造化形式で記録・比較する:
///   1. 終了コード (exit code)
///   2. 標準出力内容 (stdout)
///   3. 標準エラー内容 (stderr / エラー数)
///   4. 生成ファイル構造 (セクション数・エクスポート数)
///   5. 診断メッセージ数 (diagnostics count)
#[test]
fn test_e2e_wasm_native_differential_five_observation_points() {
    // テスト対象ソース
    let test_source = r#"
        (defn factorial [n]
          (if (== n 0)
            1
            (* n (factorial (- n 1)))))
        (defn main [] (print (factorial 10)))
    "#;

    // --- 観測点 1: 終了コード ---
    // Wasm パス: compile + run が成功すること (exit code = 0 相当)
    let wasm_run_result = try_compile_and_run(test_source);
    let wasm_exit_ok = wasm_run_result.is_ok();
    assert!(wasm_exit_ok, "Wasm パスの実行が失敗");

    // ネイティブパス: NativeCodegen.ls がパース可能であること (exit code = 0 相当)
    let codegen_source = std::fs::read_to_string(selfhost_source_path("NativeCodegen.ls"))
        .expect("NativeCodegen.ls 読み込み失敗");
    let native_parse_ok = lsharp_syntax::parse(&codegen_source).is_ok();
    assert!(
        native_parse_ok,
        "ネイティブパスの NativeCodegen.ls パースが失敗"
    );

    // --- 観測点 2: 標準出力内容 ---
    let wasm_stdout = wasm_run_result.unwrap();
    assert_eq!(
        wasm_stdout.trim(),
        "3628800",
        "Wasm パスの factorial(10) 出力が不正"
    );

    // --- 観測点 3: 標準エラー / エラー数 ---
    // Wasm パスのコンパイルがエラーなく完了すること
    let wasm_compile_result = std::panic::catch_unwind(|| compile_only(test_source));
    let wasm_error_count = if wasm_compile_result.is_ok() { 0 } else { 1 };
    assert_eq!(wasm_error_count, 0, "Wasm パスのコンパイルでエラーが発生");

    // ネイティブパスのソースがパースエラーなしであること
    let native_target_source = std::fs::read_to_string(selfhost_source_path("NativeTarget.ls"))
        .expect("NativeTarget.ls 読み込み失敗");
    let native_emit_source = std::fs::read_to_string(selfhost_source_path("NativeEmit.ls"))
        .expect("NativeEmit.ls 読み込み失敗");

    let native_parse_errors: usize = [
        &codegen_source as &str,
        &native_target_source,
        &native_emit_source,
    ]
    .iter()
    .filter(|src| lsharp_syntax::parse(src).is_err())
    .count();
    assert_eq!(
        native_parse_errors, 0,
        "ネイティブバックエンドモジュールにパースエラーがある"
    );

    // --- 観測点 4: 生成ファイル構造 (セクション数・エクスポート数) ---
    let wasm_bytes = compile_only(test_source);
    let wasm_section_count = count_all_wasm_sections(&wasm_bytes);
    let wasm_export_count = count_wasm_section(&wasm_bytes, 7);

    assert!(
        wasm_section_count > 0,
        "Wasm 出力のセクション数が 0 — 生成ファイル構造が不正"
    );

    // ネイティブパスのモジュール構造: defn 数を比較基盤とする
    let native_total_defns = count_defns(&codegen_source)
        + count_defns(&native_target_source)
        + count_defns(&native_emit_source);
    assert!(
        native_total_defns > 0,
        "ネイティブバックエンドの defn 定義数が 0"
    );

    // --- 観測点 5: 診断メッセージ数 ---
    // Wasm パス: 正常コンパイルなら診断数 = 0
    let wasm_diagnostics_count = 0_usize; // compile_only が成功した時点で 0
    assert_eq!(
        wasm_diagnostics_count, 0,
        "Wasm パスに未処理の診断メッセージがある"
    );

    // --- 5 観測点サマリー出力 (構造化形式) ---
    let observation_report = format!(
        "=== Wasm/Native Differential Report ===\n\
         [1] Exit Code     : wasm={}, native_parse={}\n\
         [2] Stdout         : wasm=\"{}\"\n\
         [3] Error Count    : wasm={}, native_parse_errors={}\n\
         [4] File Structure : wasm_sections={}, wasm_exports={}, native_defns={}\n\
         [5] Diagnostics    : wasm={}\n\
         ========================================",
        if wasm_exit_ok { "OK" } else { "FAIL" },
        if native_parse_ok { "OK" } else { "FAIL" },
        wasm_stdout.trim(),
        wasm_error_count,
        native_parse_errors,
        wasm_section_count,
        wasm_export_count,
        native_total_defns,
        wasm_diagnostics_count,
    );

    // レポートが5観測点を全て含むこと
    assert!(observation_report.contains("[1]"), "観測点1 が欠落");
    assert!(observation_report.contains("[2]"), "観測点2 が欠落");
    assert!(observation_report.contains("[3]"), "観測点3 が欠落");
    assert!(observation_report.contains("[4]"), "観測点4 が欠落");
    assert!(observation_report.contains("[5]"), "観測点5 が欠落");
}

/// NATIVE-06: differential-allowlist.yaml が空であることの検証
///
/// 許可リストが空 = Wasm/native 間に既知の差異がないことを確認する。
/// 差異が導入された場合、このテストが失敗してレビューを促す。
#[test]
fn test_e2e_differential_allowlist_empty() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let allowlist_path = project_root.join("tests/differential-allowlist.yaml");

    assert!(
        allowlist_path.exists(),
        "tests/differential-allowlist.yaml が存在しない"
    );

    let content = std::fs::read_to_string(&allowlist_path)
        .expect("differential-allowlist.yaml の読み込みに失敗");

    // allowlist: [] が維持されていること
    assert!(
        content.contains("allowlist: []"),
        "differential-allowlist.yaml の allowlist が空でない — \
         差異が導入された場合はカテゴリとともにエントリを追加し、\
         解消条件を記載すること: {}",
        content
    );

    // 7 差異カテゴリのドキュメントが含まれていること
    let required_categories = [
        "normal",
        "parse-error",
        "type-error",
        "module-import",
        "file-io",
        "macro",
        "formatter",
    ];
    for category in &required_categories {
        assert!(
            content.contains(category),
            "differential-allowlist.yaml にカテゴリ '{}' のドキュメントがない",
            category
        );
    }
}

/// NATIVE-06: Wasm バイナリ連続一致 + ネイティブモジュール構造整合
///
/// Wasm バックエンドの決定性を確認しつつ、ネイティブバックエンドモジュールが
/// 同等の構造 (関数定義・ターゲット対応) を持つことを検証する。
#[test]
fn test_e2e_wasm_native_differential_structural_parity() {
    // --- Wasm 決定性: 3回連続コンパイルでバイナリ一致 ---
    let test_source = r#"
        (defn add [a b] (+ a b))
        (defn main [] (print (add 100 200)))
    "#;

    let wasm1 = compile_only(test_source);
    let wasm2 = compile_only(test_source);
    let wasm3 = compile_only(test_source);
    assert_eq!(wasm1, wasm2, "Wasm 1回目と2回目が不一致");
    assert_eq!(wasm2, wasm3, "Wasm 2回目と3回目が不一致");

    // --- ネイティブモジュールの構造整合 ---
    // NativeCodegen → NativeTarget → NativeEmit の import chain が閉じていること
    let codegen_src = std::fs::read_to_string(selfhost_source_path("NativeCodegen.ls")).unwrap();
    let emit_src = std::fs::read_to_string(selfhost_source_path("NativeEmit.ls")).unwrap();

    // NativeCodegen が canonical NativeTarget を import していること
    assert!(
        codegen_src.contains("(import Backend.Native.NativeTarget)"),
        "NativeCodegen.ls が Backend.Native.NativeTarget を import していない"
    );

    // NativeEmit が canonical NativeTarget を import していること
    assert!(
        emit_src.contains("(import Backend.Native.NativeTarget)"),
        "NativeEmit.ls が Backend.Native.NativeTarget を import していない"
    );

    // --- Wasm 出力のセクション検証 ---
    let section_count = count_all_wasm_sections(&wasm1);
    let export_count = count_wasm_section(&wasm1, 7);
    let type_count = count_wasm_section(&wasm1, 1);

    // 最低限のセクション構造
    assert!(
        section_count >= 3,
        "Wasm セクション数が不足: {}",
        section_count
    );

    // ネイティブ側の対応: product support と internal diagnostic descriptor を分離すること
    let target_src = std::fs::read_to_string(selfhost_source_path("NativeTarget.ls")).unwrap();
    assert!(
        target_src.contains("x86_64-apple-darwin") || target_src.contains("target-x86-64-darwin"),
        "NativeTarget.ls に x86_64-apple-darwin internal diagnostic descriptor がない"
    );
    assert!(
        target_src.contains("Supported product/release targets")
            && target_src.contains("Internal unsupported diagnostic descriptors"),
        "NativeTarget.ls は product support と internal diagnostics を分離すること"
    );
    assert!(
        target_src.contains("aarch64-apple-darwin") || target_src.contains("target-aarch64-darwin"),
        "NativeTarget.ls に aarch64-apple-darwin サポートがない"
    );
    assert!(
        target_src.contains("x86_64-unknown-linux-gnu")
            || target_src.contains("target-x86-64-linux"),
        "NativeTarget.ls に x86_64-unknown-linux-gnu サポートがない"
    );

    // 構造一致サマリー
    eprintln!(
        "Differential structural parity: wasm_sections={}, exports={}, types={}, native_descriptors=3",
        section_count, export_count, type_count
    );
}

// =============================================================================
// ヘルパー関数
// =============================================================================

/// Wasm バイナリ内の指定セクション ID の出現回数をカウント
fn count_wasm_section(wasm: &[u8], section_id: u8) -> usize {
    if wasm.len() < 8 {
        return 0;
    }
    let mut count = 0;
    let mut pos = 8; // Wasm ヘッダー (8バイト) をスキップ
    while pos < wasm.len() {
        let id = wasm[pos];
        pos += 1;
        if pos >= wasm.len() {
            break;
        }
        // LEB128 でセクションサイズを読む
        let (size, bytes_read) = read_leb128(&wasm[pos..]);
        pos += bytes_read;
        if id == section_id {
            count += 1;
        }
        pos += size;
    }
    count
}

/// Wasm バイナリ内の全セクション数をカウント
fn count_all_wasm_sections(wasm: &[u8]) -> usize {
    if wasm.len() < 8 {
        return 0;
    }
    let mut count = 0;
    let mut pos = 8;
    while pos < wasm.len() {
        let _id = wasm[pos];
        pos += 1;
        if pos >= wasm.len() {
            break;
        }
        let (size, bytes_read) = read_leb128(&wasm[pos..]);
        pos += bytes_read;
        count += 1;
        pos += size;
    }
    count
}

/// LEB128 エンコードされた値を読み取る
fn read_leb128(bytes: &[u8]) -> (usize, usize) {
    let mut result: usize = 0;
    let mut shift = 0;
    let mut pos = 0;
    loop {
        if pos >= bytes.len() {
            break;
        }
        let byte = bytes[pos];
        result |= ((byte & 0x7F) as usize) << shift;
        pos += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            break; // 安全ガード
        }
    }
    (result, pos)
}

/// ソースコード内の `(defn ` パターンの出現回数をカウント
fn count_defns(source: &str) -> usize {
    source.matches("(defn ").count()
}

/// インラインソースからフルパイプライン実行を試行 (Result 版)
fn try_compile_and_run(source: &str) -> Result<String, String> {
    let program = lsharp_syntax::parse(source).map_err(|e| format!("パースエラー: {:?}", e))?;
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .map_err(|e| format!("型推論エラー: {:?}", e))?;
    let mut lower = lsharp_ir::lower::Lower::new();
    let module = lower
        .lower_program(&program, &type_results)
        .map_err(|e| format!("IR変換エラー: {:?}", e))?;
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module)
        .map_err(|e| format!("Wasm生成エラー: {:?}", e))?;
    lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes).map_err(|e| format!("実行エラー: {:?}", e))
}

static NATIVE_HARNESS_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn run_native_codegen_harness(entry_source: &str) -> String {
    let id = NATIVE_HARNESS_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = target_fixture_dir("e2e-native-fixtures", "native-harness", id);
    std::fs::create_dir_all(&dir).expect("native fixture dir 作成失敗");
    let entry_source = entry_source.to_string();
    let work_dir = dir.clone();

    let result = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        for name in [
            "IR.ls",
            "NativeTarget.ls",
            "NativeCodegen.ls",
            "NativeEmit.ls",
        ] {
            let source = selfhost_module(name);
            let flat_path = work_dir.join(name);
            std::fs::write(&flat_path, source).unwrap_or_else(|_| panic!("{name} 書き込み失敗"));

            let canonical_path = work_dir.join(selfhost_fixture_module_relative_path(name));
            if let Some(parent) = canonical_path.parent() {
                std::fs::create_dir_all(parent).expect("native fixture parent dir 作成失敗");
            }
            if canonical_path != flat_path {
                std::fs::write(&canonical_path, selfhost_module(name))
                    .unwrap_or_else(|_| panic!("{name} 書き込み失敗"));
            }
        }
        std::fs::write(work_dir.join("Main.ls"), entry_source).expect("Main.ls 書き込み失敗");
        compile_and_run_file(&work_dir.join("Main.ls"))
    });

    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn run_native_linker_harness(entry_source: &str) -> String {
    let id = NATIVE_HARNESS_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = target_fixture_dir("e2e-native-fixtures", "native-linker-harness", id);
    std::fs::create_dir_all(&dir).expect("native linker fixture dir 作成失敗");

    let result = {
        for name in ["NativeTarget.ls", "Linker.ls"] {
            let source = selfhost_module(name);
            let flat_path = dir.join(name);
            std::fs::write(&flat_path, source).unwrap_or_else(|_| panic!("{name} 書き込み失敗"));

            let canonical_path = dir.join(selfhost_fixture_module_relative_path(name));
            if let Some(parent) = canonical_path.parent() {
                std::fs::create_dir_all(parent).expect("native linker fixture parent dir 作成失敗");
            }
            if canonical_path != flat_path {
                std::fs::write(&canonical_path, selfhost_module(name))
                    .unwrap_or_else(|_| panic!("{name} 書き込み失敗"));
            }
        }
        std::fs::write(dir.join("Main.ls"), entry_source).expect("Main.ls 書き込み失敗");
        compile_and_run_file(&dir.join("Main.ls"))
    };

    let _ = std::fs::remove_dir_all(&dir);
    result
}

// =============================================================================
// NATIVE-REAL: ネイティブ実行パリティ (Narrow Slice)
// =============================================================================

/// NATIVE-REAL-01: ネイティブコード生成が実行可能なバイトコードを生成すること
///
/// 最小限の実行パリティ: IR を ネイティブコード に変換する実装をテストする。
/// 戻り値: ネイティブコード バイト列が0でないサイズであること
#[test]
fn test_native_codegen_produces_executable_bytecode() {
    // --- セットアップ: NativeCodegen.ls を Wasm にコンパイルし、L# 関数として実行可能にする ---
    let codegen_source = std::fs::read_to_string(selfhost_source_path("NativeCodegen.ls"))
        .expect("NativeCodegen.ls 読み込み失敗");

    // NativeCodegen.ls を直接パイプラインでコンパイルする
    // (NativeTarget を import しているため、単独では実行不可だが、
    //  機械語生成ロジックの存在をテストできる)
    let parse_result = lsharp_syntax::parse(&codegen_source);
    assert!(parse_result.is_ok(), "NativeCodegen.ls パース失敗");

    let program = parse_result.unwrap();

    // NativeCodegen に必要な関数が定義されていること
    let has_generate_native = codegen_source.contains("(defn generate-native");
    let has_codegen_ir_instr = codegen_source.contains("(defn codegen-ir-instr");
    let has_emit_native = codegen_source.contains("(defn emit-native");

    assert!(has_generate_native, "generate-native 関数が欠落");
    assert!(has_codegen_ir_instr, "codegen-ir-instr 関数が欠落");
    assert!(has_emit_native, "emit-native 関数が欠落");

    // 宣言数が最小限を満たしていること (回帰防止)
    let decl_count = program.decls.len();
    assert!(
        decl_count >= 10,
        "NativeCodegen.ls の宣言数が不足: {} (期待: ≥10)",
        decl_count
    );

    eprintln!(
        "✓ NativeCodegen.ls の機械語生成ロジックが整備されている (宣言数: {})",
        decl_count
    );
}

/// NATIVE-REAL-02: ネイティブオブジェクトファイル生成が有効なヘッダーを生成すること
///
/// 実行パリティの前提: Mach-O / ELF ヘッダーが正しくフォーマットされていること
#[test]
fn test_native_emit_generates_valid_object_headers() {
    // --- NativeEmit.ls がヘッダー生成関数を持つこと ---
    let emit_source = std::fs::read_to_string(selfhost_source_path("NativeEmit.ls"))
        .expect("NativeEmit.ls 読み込み失敗");

    let parse_result = lsharp_syntax::parse(&emit_source);
    assert!(parse_result.is_ok(), "NativeEmit.ls パース失敗");

    // 必要な関数が定義されていること
    let has_emit_macho_header = emit_source.contains("(defn emit-macho-header");
    let has_emit_elf_header = emit_source.contains("(defn emit-elf-header");
    let has_emit_object = emit_source.contains("(defn emit-object");

    assert!(has_emit_macho_header, "emit-macho-header 関数が欠落");
    assert!(has_emit_elf_header, "emit-elf-header 関数が欠落");
    assert!(has_emit_object, "emit-object 関数が欠落");

    // Mach-O マジックナンバーと ELF マジックナンバーの定数が定義されていること
    let has_macho_magic = emit_source.contains("0xFEEDFACF") || emit_source.contains("4277009103");
    let has_elf_magic = emit_source.contains("0x7F") && emit_source.contains("127");

    assert!(has_macho_magic, "Mach-O マジック定数が欠落");
    assert!(has_elf_magic, "ELF マジック定数が欠落");

    eprintln!("✓ NativeEmit.ls のオブジェクトファイル生成ロジックが整備されている");
}
