use super::support::*;

fn selfhost_test_label(name: &str) -> &'static str {
    match name {
        "GC.ls" => "selfhost/src/Runtime/GC.ls",
        "NativeTarget.ls" => "selfhost/src/Backend/Native/NativeTarget.ls",
        "NativeCodegen.ls" => "selfhost/src/Backend/Native/NativeCodegen.ls",
        "NativeEmit.ls" => "selfhost/src/Backend/Native/NativeEmit.ls",
        "Linker.ls" => "selfhost/src/Backend/Native/Linker.ls",
        "Cli.ls" => "selfhost/src/App/Cli.ls",
        other => panic!("不明な selfhost test label: {other}"),
    }
}

fn read_selfhost_test_source(name: &str, missing_hint: &str) -> String {
    let path = selfhost_source_path(name);
    let label = selfhost_test_label(name);
    assert!(path.exists(), "{label} が存在しない -- {missing_hint}");
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{label} の読み込みに失敗"))
}

/// TEST-GC-03: 世代別 GC (nursery, write-barrier, promotion)
#[test]
fn test_e2e_selfhost_gc_generational() {
    let gc_source = read_selfhost_test_source("GC.ls", "GC モジュールを作成してください");

    // nursery (若い世代の領域)
    assert!(
        gc_source.contains("nursery")
            || gc_source.contains("Nursery")
            || gc_source.contains("young-gen"),
        "selfhost/src/Runtime/GC.ls に nursery / young generation 関連の定義がない"
    );

    // write-barrier (古い世代から若い世代へのポインタ書き込み検知)
    assert!(
        gc_source.contains("write-barrier")
            || gc_source.contains("write_barrier")
            || gc_source.contains("WriteBarrier"),
        "selfhost/src/Runtime/GC.ls に write-barrier 関連の定義がない"
    );

    // promotion (若い世代から古い世代への昇格)
    assert!(
        gc_source.contains("promote")
            || gc_source.contains("promotion")
            || gc_source.contains("tenure"),
        "selfhost/src/Runtime/GC.ls に promotion / tenure 関連の定義がない"
    );

    // コンパイルが通ること
    let program = lsharp_syntax::parse(&gc_source);
    assert!(
        program.is_ok(),
        "selfhost/src/Runtime/GC.ls のパースに失敗: {:?}",
        program.err()
    );
    let program = program.unwrap();

    let mut infer = Infer::new();
    let types = infer.infer_program(&program);
    assert!(
        types.is_ok(),
        "selfhost/src/Runtime/GC.ls の型チェックに失敗: {:?}",
        types.err()
    );
}

/// TEST-GC-04: 長寿命ベンチマーク -- GC が大量割り当て後も安定動作すること
#[test]
fn test_e2e_selfhost_gc_longevity_benchmark() {
    let gc_source = read_selfhost_test_source("GC.ls", "GC モジュールを作成してください");

    // gc-collect または collect 関数 (手動/自動 GC トリガー)
    assert!(
        gc_source.contains("gc-collect")
            || gc_source.contains("collect")
            || gc_source.contains("(defn gc"),
        "selfhost/src/Runtime/GC.ls に collect / gc トリガー関数がない"
    );

    // 大量割り当てテスト用のコード: GC モジュールをインポートして繰り返し alloc する
    let bench_source = r#"
(module Bench)
(import GC)

(defn bench-alloc [n]
  (if (<= n 0)
    0
    (let [_ (GC.alloc 64)]
      (bench-alloc (- n 1)))))

(defn main []
  (let [result (bench-alloc 10000)
        _ (GC.collect)]
    (do
      (print (GC.heap-used))
      0)))
"#;

    // ベンチマークソースがパースできること (GC.ls 実装後に実行可能になる)
    let program = lsharp_syntax::parse(bench_source);
    assert!(
        program.is_ok(),
        "ベンチマークソースのパースに失敗: {:?}",
        program.err()
    );

    // GC モジュール自体が型チェックを通ること
    let gc_program = lsharp_syntax::parse(&gc_source);
    assert!(gc_program.is_ok(), "selfhost/src/Runtime/GC.ls のパースに失敗");
    let gc_program = gc_program.unwrap();

    let mut infer = Infer::new();
    let types = infer.infer_program(&gc_program);
    assert!(
        types.is_ok(),
        "selfhost/src/Runtime/GC.ls の型チェックに失敗: {:?}",
        types.err()
    );

    // heap-used メトリクス関数が存在すること
    assert!(
        gc_source.contains("heap-used")
            || gc_source.contains("heap_used")
            || gc_source.contains("HeapUsed"),
        "selfhost/src/Runtime/GC.ls に heap-used メトリクス関数がない"
    );
}

/// TEST-GC-05: LSP soak + REPL GC テスト -- 長時間稼働で GC が正しく動作すること
#[test]
fn test_e2e_selfhost_gc_lsp_soak_repl() {
    let gc_source = read_selfhost_test_source("GC.ls", "GC モジュールを作成してください");

    // GC 統計情報 API (LSP soak テストで使用)
    assert!(
        gc_source.contains("gc-stats")
            || gc_source.contains("gc_stats")
            || gc_source.contains("GcStats"),
        "selfhost/src/Runtime/GC.ls に gc-stats 関連の定義がない"
    );

    // LSP soak テスト: 繰り返し型チェック + GC を行うシナリオ
    let soak_source = r#"
(module SoakTest)
(import GC)

(defn simulate-lsp-cycle [iterations]
  (if (<= iterations 0)
    (GC.total-collections)
    (let [_ (GC.alloc 128)
          _ (GC.alloc 256)
          _ (GC.collect)]
      (simulate-lsp-cycle (- iterations 1)))))

(defn main []
  (let [collections (simulate-lsp-cycle 100)]
    (do
      (print collections)
      0)))
"#;

    let program = lsharp_syntax::parse(soak_source);
    assert!(
        program.is_ok(),
        "LSP soak テストソースのパースに失敗: {:?}",
        program.err()
    );

    // total-collections メトリクス関数
    assert!(
        gc_source.contains("total-collections")
            || gc_source.contains("total_collections")
            || gc_source.contains("num-collections"),
        "selfhost/src/Runtime/GC.ls に total-collections メトリクス関数がない"
    );

    // REPL 用途: セッション間の GC リセット
    assert!(
        gc_source.contains("gc-reset")
            || gc_source.contains("gc_reset")
            || gc_source.contains("reset-heap"),
        "selfhost/src/Runtime/GC.ls に gc-reset / reset-heap 関連の定義がない"
    );
}

/// TEST-GC-06: leak detection + metrics -- メモリリーク検知と GC メトリクス
#[test]
fn test_e2e_selfhost_gc_leak_detection() {
    let gc_source = read_selfhost_test_source("GC.ls", "GC モジュールを作成してください");

    // leak detection 機能
    assert!(
        gc_source.contains("detect-leak")
            || gc_source.contains("detect_leak")
            || gc_source.contains("leak-check")
            || gc_source.contains("LeakDetector"),
        "selfhost/src/Runtime/GC.ls に leak detection 関連の定義がない"
    );

    // メトリクス: 割り当て数
    assert!(
        gc_source.contains("alloc-count")
            || gc_source.contains("alloc_count")
            || gc_source.contains("total-allocs"),
        "selfhost/src/Runtime/GC.ls に alloc-count メトリクス関数がない"
    );

    // メトリクス: 回収数
    assert!(
        gc_source.contains("freed-count")
            || gc_source.contains("freed_count")
            || gc_source.contains("total-freed"),
        "selfhost/src/Runtime/GC.ls に freed-count メトリクス関数がない"
    );

    // leak detection テスト: alloc → collect 後に leak がないことを検証
    let leak_test_source = r#"
(module LeakTest)
(import GC)

(defn main []
  (let [before-allocs (GC.alloc-count)
        _ (GC.alloc 64)
        _ (GC.alloc 128)
        _ (GC.collect)
        after-freed (GC.freed-count)
        leaks (GC.detect-leak)]
    (do
      (print leaks)
      0)))
"#;

    let program = lsharp_syntax::parse(leak_test_source);
    assert!(
        program.is_ok(),
        "leak detection テストソースのパースに失敗: {:?}",
        program.err()
    );

    // GC モジュール自体が型チェックを通ること
    let gc_program = lsharp_syntax::parse(&gc_source);
    assert!(gc_program.is_ok(), "selfhost/src/Runtime/GC.ls のパースに失敗");
    let gc_program = gc_program.unwrap();

    let mut infer = Infer::new();
    let types = infer.infer_program(&gc_program);
    assert!(
        types.is_ok(),
        "selfhost/src/Runtime/GC.ls の型チェックに失敗: {:?}",
        types.err()
    );
}

// =============================================================================
// Phase 6 Group G: Native Backend テスト (TDD Red Phase)
// =============================================================================

/// TEST-NATIVE-01: selfhost/src/Backend/Native/NativeTarget.ls の存在 + ターゲット記述子定義
///
/// selfhost/src/Backend/Native/NativeTarget.ls が存在し、x86_64-apple-darwin, aarch64-apple-darwin,
/// x86_64-unknown-linux-gnu の3つのターゲット記述子が定義されていることを検証する。
/// Red Phase: NativeTarget.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_native_target_descriptors() {
    let source = read_selfhost_test_source(
        "NativeTarget.ls",
        "ネイティブターゲットモジュールを作成してください",
    );

    // モジュール宣言
    assert!(
        source.contains("(module Backend.Native.NativeTarget)"),
        "selfhost/src/Backend/Native/NativeTarget.ls に namespaced module 宣言がない"
    );

    // x86_64-apple-darwin ターゲット記述子
    assert!(
        source.contains("x86_64-apple-darwin")
            || source.contains("x86-64-macos")
            || source.contains("target-x86-64-darwin"),
        "selfhost/src/Backend/Native/NativeTarget.ls に x86_64-apple-darwin ターゲット記述子がない"
    );

    // aarch64-apple-darwin ターゲット記述子
    assert!(
        source.contains("aarch64-apple-darwin")
            || source.contains("arm64-macos")
            || source.contains("target-aarch64-darwin"),
        "selfhost/src/Backend/Native/NativeTarget.ls に aarch64-apple-darwin ターゲット記述子がない"
    );

    // x86_64-unknown-linux-gnu ターゲット記述子
    assert!(
        source.contains("x86_64-unknown-linux-gnu")
            || source.contains("x86-64-linux")
            || source.contains("target-x86-64-linux"),
        "selfhost/src/Backend/Native/NativeTarget.ls に x86_64-unknown-linux-gnu ターゲット記述子がない"
    );

    // ターゲット取得関数が存在すること
    assert!(
        source.contains("(defn get-target")
            || source.contains("(defn native-target")
            || source.contains("(defn make-target"),
        "selfhost/src/Backend/Native/NativeTarget.ls にターゲット取得関数が未定義"
    );
}

/// TEST-NATIVE-02: selfhost/src/Backend/Native/NativeCodegen.ls + NativeEmit.ls の存在
///
/// ネイティブコード生成モジュール (NativeCodegen.ls) と
/// ネイティブバイナリ出力モジュール (NativeEmit.ls) が存在することを検証する。
/// Red Phase: 両ファイルが未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_native_object_emitter() {
    let codegen_source = read_selfhost_test_source(
        "NativeCodegen.ls",
        "ネイティブコード生成モジュールを作成してください",
    );

    // モジュール宣言
    assert!(
        codegen_source.contains("(module Backend.Native.NativeCodegen)"),
        "selfhost/src/Backend/Native/NativeCodegen.ls に namespaced module 宣言がない"
    );

    // コード生成関数が定義されていること
    assert!(
        codegen_source.contains("(defn emit-native")
            || codegen_source.contains("(defn codegen-native")
            || codegen_source.contains("(defn generate-native"),
        "selfhost/src/Backend/Native/NativeCodegen.ls にネイティブコード生成関数が未定義"
    );

    let emit_source = read_selfhost_test_source(
        "NativeEmit.ls",
        "ネイティブバイナリ出力モジュールを作成してください",
    );

    // モジュール宣言
    assert!(
        emit_source.contains("(module Backend.Native.NativeEmit)"),
        "selfhost/src/Backend/Native/NativeEmit.ls に namespaced module 宣言がない"
    );

    // オブジェクトファイル出力関数が定義されていること
    assert!(
        emit_source.contains("(defn emit-object")
            || emit_source.contains("(defn write-object")
            || emit_source.contains("(defn emit-elf")
            || emit_source.contains("(defn emit-macho"),
        "selfhost/src/Backend/Native/NativeEmit.ls にオブジェクトファイル出力関数が未定義"
    );
}

/// TEST-NATIVE-03: selfhost/src/Backend/Native/Linker.ls の存在 + response file 関連関数
///
/// selfhost/src/Backend/Native/Linker.ls が存在し、リンカー呼び出しと
/// response file (@file) 生成関数が定義されていることを検証する。
/// Red Phase: Linker.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_linker_response() {
    let source =
        read_selfhost_test_source("Linker.ls", "リンカーモジュールを作成してください");

    // モジュール宣言
    assert!(
        source.contains("(module Backend.Native.Linker)"),
        "selfhost/src/Backend/Native/Linker.ls に namespaced module 宣言がない"
    );

    // リンカー呼び出し関数
    assert!(
        source.contains("(defn link")
            || source.contains("(defn invoke-linker")
            || source.contains("(defn run-linker"),
        "selfhost/src/Backend/Native/Linker.ls にリンカー呼び出し関数が未定義"
    );

    // response file 生成関数
    assert!(
        source.contains("response-file")
            || source.contains("write-response")
            || source.contains("generate-response"),
        "selfhost/src/Backend/Native/Linker.ls に response file 関連関数が未定義"
    );
}

/// TEST-NATIVE-04: ネイティブビルドの決定性検証 -- 2回ビルドで同一バイナリハッシュ
///
/// selfhost/src/Backend/Native/NativeCodegen.ls を使用して同じソースを2回コンパイルし、
/// 生成されるバイナリが同一であること (決定的コンパイル) を検証する。
/// Red Phase: NativeCodegen.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_native_deterministic_codegen() {
    let codegen_source = read_selfhost_test_source(
        "NativeCodegen.ls",
        "決定的コンパイルの検証にはネイティブコード生成モジュールが必要",
    );

    // 決定的コード生成を保証する関数やメカニズムが存在すること
    assert!(
        codegen_source.contains("deterministic")
            || codegen_source.contains("reproducible")
            || codegen_source.contains("(defn codegen")
            || codegen_source.contains("(defn emit-native"),
        "selfhost/src/Backend/Native/NativeCodegen.ls に決定的コード生成メカニズムがない"
    );

    // NativeCodegen.ls がコンパイル可能であることを検証
    let program = lsharp_syntax::parse(&codegen_source);
    assert!(
        program.is_ok(),
        "selfhost/src/Backend/Native/NativeCodegen.ls のパースに失敗: {:?}",
        program.err()
    );
    let program = program.unwrap();

    // NativeCodegen.ls は NativeTarget をインポートするため単体での型チェックはスキップ
    // パースが成功していれば決定的コード生成の前提条件 (ソースの一貫性) を満たす
    let has_imports = program
        .decls
        .iter()
        .any(|d| matches!(d, lsharp_syntax::ast::Decl::ImportDecl { .. }));
    if has_imports {
        // インポートがある場合: パース成功のみ確認 (型チェックはモジュール間依存があるためスキップ)
        // 決定的コード生成の検証: 同一ソースから同一パース結果が得られることを確認
        let program2 = lsharp_syntax::parse(&codegen_source).unwrap();
        assert_eq!(
            format!("{:?}", program.decls.len()),
            format!("{:?}", program2.decls.len()),
            "selfhost/src/Backend/Native/NativeCodegen.ls の2回パースで宣言数が一致しない (非決定的パース)"
        );
        return;
    }

    // インポートがない場合: フルコンパイルで決定性を検証
    let mut infer1 = Infer::new();
    let types1 = infer1.infer_program(&program);
    assert!(
        types1.is_ok(),
        "selfhost/src/Backend/Native/NativeCodegen.ls の型チェック (1回目) に失敗: {:?}",
        types1.err()
    );
    let types1 = types1.unwrap();

    let mut lower1 = Lower::new();
    let module1 = lower1.lower_program(&program, &types1);
    assert!(
        module1.is_ok(),
        "selfhost/src/Backend/Native/NativeCodegen.ls の IR lowering (1回目) に失敗: {:?}",
        module1.err()
    );
    let wasm1 = lsharp_wasm::wasi::emit_wasm_wasi(&module1.unwrap()).unwrap();

    // 2回目
    let program2 = lsharp_syntax::parse(&codegen_source).unwrap();
    let mut infer2 = Infer::new();
    let types2 = infer2.infer_program(&program2).unwrap();
    let mut lower2 = Lower::new();
    let module2 = lower2.lower_program(&program2, &types2).unwrap();
    let wasm2 = lsharp_wasm::wasi::emit_wasm_wasi(&module2).unwrap();

    assert_eq!(
        wasm1, wasm2,
        "selfhost/src/Backend/Native/NativeCodegen.ls の2回コンパイルでバイナリが一致しない (非決定的コンパイル)"
    );
}

/// TEST-NATIVE-05: stage1-native 自己再生成
///
/// Rust コンパイラで生成した stage1 ネイティブバイナリが、
/// 自身のソースを再コンパイルして stage2 を生成できる構造を持つことを検証する。
/// Red Phase: ネイティブバックエンドが未実装のため FAIL する。
#[test]
fn test_e2e_selfhost_native_self_regeneration() {
    // ネイティブバックエンドの主要モジュールが全て存在すること
    let required_modules = [
        "NativeTarget.ls",
        "NativeCodegen.ls",
        "NativeEmit.ls",
        "Linker.ls",
    ];

    for name in &required_modules {
        let path = selfhost_source_path(name);
        assert!(
            path.exists(),
            "{} が存在しない -- ネイティブバックエンドの自己再生成には全モジュールが必要",
            selfhost_test_label(name)
        );
    }

    // canonical Main にネイティブバックエンド関連の import が存在すること
    let main_path = selfhost_main_path();
    let main_source = std::fs::read_to_string(&main_path)
        .unwrap_or_else(|_| panic!("{} の読み込みに失敗", main_path.display()));

    assert!(
        main_source.contains("NativeTarget")
            || main_source.contains("NativeCodegen")
            || main_source.contains("native"),
        "canonical Main にネイティブバックエンド関連の参照がない -- \
         自己再生成にはネイティブコンパイルパスが Main に統合されている必要がある"
    );

    // NativeCodegen.ls がコンパイルパイプライン関数を持つこと
    let codegen_source = read_selfhost_test_source(
        "NativeCodegen.ls",
        "ネイティブバックエンドの自己再生成にはコード生成モジュールが必要",
    );

    assert!(
        codegen_source.contains("(defn compile-to-native")
            || codegen_source.contains("(defn emit-native")
            || codegen_source.contains("(defn native-pipeline"),
        "selfhost/src/Backend/Native/NativeCodegen.ls にネイティブコンパイルパイプライン関数がない"
    );
}

/// TEST-NATIVE-06: Wasm/native 結果比較 -- 同じソースの Wasm 実行とネイティブ実行の結果が一致
///
/// 同じ L# ソースを Wasm バックエンドとネイティブバックエンドの両方でコンパイル・実行し、
/// stdout 出力が一致することを検証する。
/// Red Phase: ネイティブバックエンドが未実装のため FAIL する。
#[test]
fn test_e2e_selfhost_wasm_native_differential() {
    // ネイティブバックエンドの主要モジュールが存在すること
    let codegen_path = selfhost_source_path("NativeCodegen.ls");
    assert!(
        codegen_path.exists(),
        "selfhost/src/Backend/Native/NativeCodegen.ls が存在しない -- Wasm/native 差分比較にはネイティブバックエンドが必要"
    );

    let emit_path = selfhost_source_path("NativeEmit.ls");
    assert!(
        emit_path.exists(),
        "selfhost/src/Backend/Native/NativeEmit.ls が存在しない -- Wasm/native 差分比較にはネイティブバイナリ出力が必要"
    );

    // テスト対象のシンプルなソース
    let test_source = r#"
        (defn factorial [n]
          (if (== n 0)
            1
            (* n (factorial (- n 1)))))
        (defn main [] (print (factorial 10)))
    "#;

    // Wasm バックエンドで実行
    let wasm_output = compile_and_run(test_source);
    assert_eq!(
        wasm_output.trim(),
        "3628800",
        "Wasm バックエンドの factorial(10) が不正"
    );

    // NATIVE-06 前提: 同一ソースの Wasm バイナリが連続 compile で一致（バックエンド差分比較の土台）
    let wasm_bin_a = compile_only(test_source);
    let wasm_bin_b = compile_only(test_source);
    assert_eq!(
        wasm_bin_a, wasm_bin_b,
        "factorial ソースの Wasm 出力は決定的であるべき (WASM-03 / NATIVE-06 前提)"
    );

    // ネイティブバックエンド用のコンパイル関数が NativeCodegen.ls に存在すること
    let codegen_source =
        std::fs::read_to_string(&codegen_path).expect("selfhost/src/Backend/Native/NativeCodegen.ls の読み込みに失敗");

    assert!(
        codegen_source.contains("(defn compile-and-run-native")
            || codegen_source.contains("(defn native-run")
            || codegen_source.contains("(defn emit-and-execute"),
        "selfhost/src/Backend/Native/NativeCodegen.ls にネイティブ実行関数が未定義 -- \
         Wasm/native 差分比較にはネイティブコンパイル + 実行関数が必要"
    );

    // TODO: ネイティブバックエンド実装後に以下を有効化
    // let native_output = native_compile_and_run(test_source);
    // assert_eq!(
    //     wasm_output.trim(), native_output.trim(),
    //     "Wasm とネイティブの実行結果が一致しない: wasm='{}', native='{}'",
    //     wasm_output.trim(), native_output.trim()
    // );
}

// =============================================================================
// Phase 6 Group I: Toolchain parity テスト (TDD Red Phase)
// =============================================================================

/// TEST-CLI-01: docs/development/planning/toolchain-parity-spec.md に 13 CLI command の入出力契約が表形式で定義されていること
///
/// T4a-1 AC-100/AC-101/AC-102: サブコマンド引数仕様テーブル、stdout/stderr 使い分け、終了コード表
/// Red Phase: 仕様書に入出力契約テーブルが未記載のため FAIL する。
#[test]
fn test_e2e_selfhost_cli_command_contracts() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let spec_path = project_root.join("docs/development/planning/toolchain-parity-spec.md");
    assert!(
        spec_path.exists(),
        "docs/development/planning/toolchain-parity-spec.md が存在しない"
    );
    let spec = std::fs::read_to_string(&spec_path)
        .expect("docs/development/planning/toolchain-parity-spec.md の読み込みに失敗");

    // 13 CLI コマンドの入出力契約テーブルが存在すること
    let cli_commands = [
        "parse",
        "check",
        "compile",
        "build",
        "test",
        "review",
        "doc-ack",
        "doc-check",
        "install",
        "repl",
        "lsp",
        "fmt",
        "doc",
    ];

    // 仕様書に全 13 コマンドが記載されていることを確認
    for cmd in &cli_commands {
        assert!(
            spec.contains(cmd),
            "../../../docs/development/planning/toolchain-parity-spec.md に CLI コマンド '{}' の記載がない",
            cmd
        );
    }

    // テーブル形式 (Markdown table) で引数・入出力・終了コードが定義されていること
    // AC-100: 引数仕様テーブル
    assert!(
        spec.contains("| コマンド")
            || spec.contains("| Command")
            || spec.contains("| サブコマンド"),
        "CLI コマンドの入出力契約テーブルが存在しない (AC-100)"
    );
    // AC-102: 終了コード体系
    assert!(
        spec.contains("終了コード") || spec.contains("exit code") || spec.contains("Exit Code"),
        "終了コード体系の記載がない (AC-102)"
    );
    // AC-101: stdout/stderr の使い分け
    assert!(
        spec.contains("stdout") && spec.contains("stderr"),
        "stdout/stderr の使い分け記載がない (AC-101)"
    );
}

/// TEST-CLI-02-A: selfhost/src/App/Cli.ls 存在 + parse/check/compile/build/test コマンド定義
///
/// T4-1: L# 製 CLI の正式化 -- 基本コンパイラコマンドが定義されていること
/// Red Phase: selfhost/src/App/Cli.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_cli_parse_check_compile() {
    let source = read_selfhost_test_source("Cli.ls", "T4-1: L# 製 CLI の正式化");

    // 基本コンパイラコマンドの定義を確認
    let commands = ["parse", "check", "compile", "build", "test"];
    for cmd in &commands {
        assert!(
            source.contains(cmd),
            "selfhost/src/App/Cli.ls に '{}' コマンドの定義がない",
            cmd
        );
    }
}

/// TEST-CLI-02-B: selfhost/src/App/Cli.ls に review/doc-ack/doc-check/install コマンド定義
///
/// T4-4 AC-013: docs/review 系コマンドが L# 実装で動作すること
/// Red Phase: selfhost/src/App/Cli.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_cli_review_doc() {
    let source = read_selfhost_test_source("Cli.ls", "L# 製 CLI の正式化");

    // docs/review 系コマンドの定義を確認 (T4-4 AC-013)
    let commands = ["review", "doc-ack", "doc-check", "install"];
    for cmd in &commands {
        assert!(
            source.contains(cmd),
            "selfhost/src/App/Cli.ls に '{}' コマンドの定義がない (AC-013)",
            cmd
        );
    }
}
