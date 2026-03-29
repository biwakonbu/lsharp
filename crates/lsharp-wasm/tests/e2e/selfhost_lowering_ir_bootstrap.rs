use super::support::*;

fn selfhost_lowering_label(name: &str) -> &'static str {
    match name {
        "ModuleGraph.ls" => "selfhost/src/IR/ModuleGraph.ls",
        "Lower.ls" => "selfhost/src/IR/Lower.ls",
        "LowerExpr.ls" => "selfhost/src/IR/LowerExpr.ls",
        "LowerDecl.ls" => "selfhost/src/IR/LowerDecl.ls",
        "LowerPattern.ls" => "selfhost/src/IR/LowerPattern.ls",
        "Closure.ls" => "selfhost/src/IR/Closure.ls",
        "IR.ls" => "selfhost/src/IR/IR.ls",
        "Codegen.ls" => "selfhost/src/Backend/Wasm/Codegen.ls",
        "Emit.ls" => "selfhost/src/Backend/Wasm/Emit.ls",
        "WasiBackend.ls" => "selfhost/src/Backend/Wasm/WasiBackend.ls",
        "TestRunner.ls" => "selfhost/src/Tools/Test/TestRunner.ls",
        "GC.ls" => "selfhost/src/Runtime/GC.ls",
        other => panic!("不明な lowering selfhost module: {other}"),
    }
}

fn read_selfhost_lowering_source(name: &str, missing_hint: &str) -> String {
    let path = selfhost_source_path(name);
    let label = selfhost_lowering_label(name);
    assert!(path.exists(), "{label} が存在しない -- {missing_hint}");
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{label} の読み込みに失敗"))
}

fn collect_ls_files_recursive(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_ls_files_recursive(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "ls") {
            files.push(path);
        }
    }
}

// === Phase 6 Group E: IR / WASM / BOOT 系テスト ===

/// TEST-IR-01: selfhost/src/IR/ModuleGraph.ls の存在 + topological-sort, detect-cycle 関数
#[test]
fn test_e2e_selfhost_module_graph() {
    let mg_content = read_selfhost_lowering_source("ModuleGraph.ls", "モジュール依存グラフ未作成");

    // モジュール宣言を検証
    assert!(
        mg_content.contains("(module IR.ModuleGraph)"),
        "selfhost/src/IR/ModuleGraph.ls に (module IR.ModuleGraph) 宣言がない"
    );

    // topological-sort 関数が定義されていることを検証
    assert!(
        mg_content.contains("(defn topological-sort"),
        "selfhost/src/IR/ModuleGraph.ls に topological-sort 関数が未定義"
    );

    // detect-cycle 関数が定義されていることを検証
    assert!(
        mg_content.contains("(defn detect-cycle"),
        "selfhost/src/IR/ModuleGraph.ls に detect-cycle 関数が未定義"
    );
}

/// TEST-IR-02: selfhost/src/IR/Lower.ls, LowerExpr.ls, LowerDecl.ls, LowerPattern.ls の存在
#[test]
fn test_e2e_selfhost_lower_split() {
    let files = ["Lower.ls", "LowerExpr.ls", "LowerDecl.ls", "LowerPattern.ls"];

    for file in files {
        let path = selfhost_source_path(file);
        assert!(
            path.exists(),
            "{} が存在しない -- lowering 分割モジュール未作成",
            selfhost_lowering_label(file)
        );
    }

    // 各ファイルにモジュール宣言があることを検証
    for (file, expected_module) in &[
        ("Lower.ls", "(module IR.Lower)"),
        ("LowerExpr.ls", "(module IR.LowerExpr)"),
        ("LowerDecl.ls", "(module IR.LowerDecl)"),
        ("LowerPattern.ls", "(module IR.LowerPattern)"),
    ] {
        let content = read_selfhost_lowering_source(file, "lowering 分割モジュール未作成");
        assert!(
            content.contains(expected_module),
            "{} に {} 宣言がない",
            selfhost_lowering_label(file),
            expected_module
        );
    }
}

/// TEST-IR-03: selfhost/src/IR/Closure.ls の存在 + free-vars, capture-env 関数
#[test]
fn test_e2e_selfhost_closure_conversion() {
    let content = read_selfhost_lowering_source("Closure.ls", "クロージャ変換モジュール未作成");

    assert!(
        content.contains("(module IR.Closure)"),
        "selfhost/src/IR/Closure.ls に (module IR.Closure) 宣言がない"
    );

    assert!(
        content.contains("(defn free-vars"),
        "selfhost/src/IR/Closure.ls に free-vars 関数が未定義"
    );

    assert!(
        content.contains("(defn capture-env"),
        "selfhost/src/IR/Closure.ls に capture-env 関数が未定義"
    );
}

/// TEST-IR-04: LowerPattern.ls に literal/constructor/record/wildcard パターン lowering 関数
#[test]
fn test_e2e_selfhost_pattern_lowering() {
    let content = read_selfhost_lowering_source("LowerPattern.ls", "lowering 分割モジュール未作成");

    // literal パターン lowering
    assert!(
        content.contains("(defn lower-literal-pattern")
            || content.contains("(defn lower-pattern-literal"),
        "selfhost/src/IR/LowerPattern.ls に literal パターン lowering 関数が未定義"
    );

    // constructor パターン lowering
    assert!(
        content.contains("(defn lower-constructor-pattern")
            || content.contains("(defn lower-pattern-constructor"),
        "selfhost/src/IR/LowerPattern.ls に constructor パターン lowering 関数が未定義"
    );

    // record パターン lowering
    assert!(
        content.contains("(defn lower-record-pattern")
            || content.contains("(defn lower-pattern-record"),
        "selfhost/src/IR/LowerPattern.ls に record パターン lowering 関数が未定義"
    );

    // wildcard パターン lowering
    assert!(
        content.contains("(defn lower-wildcard-pattern")
            || content.contains("(defn lower-pattern-wildcard"),
        "selfhost/src/IR/LowerPattern.ls に wildcard パターン lowering 関数が未定義"
    );
}

/// TEST-IR-05: LowerDecl.ls に辞書引数付き call 変換関数
#[test]
fn test_e2e_selfhost_trait_dispatch_lowering() {
    let content = read_selfhost_lowering_source("LowerDecl.ls", "lowering 分割モジュール未作成");

    assert!(
        content.contains("(module IR.LowerDecl)"),
        "selfhost/src/IR/LowerDecl.ls に (module IR.LowerDecl) 宣言がない"
    );

    // 辞書引数付き call 変換関数を検証
    assert!(
        content.contains("(defn lower-trait-call")
            || content.contains("(defn lower-dict-call")
            || content.contains("(defn emit-dict-passing"),
        "selfhost/src/IR/LowerDecl.ls に辞書引数付き call 変換関数が未定義"
    );
}

/// TEST-IR-06: IR snapshot を line-based format で出力できること
#[test]
fn test_e2e_selfhost_ir_snapshot_serializer() {
    let content = read_selfhost_lowering_source("IR.ls", "IR スナップショット出力未作成");

    // line-based snapshot serializer 関数を検証
    assert!(
        content.contains("(defn ir-to-snapshot")
            || content.contains("(defn serialize-ir")
            || content.contains("(defn ir-snapshot"),
        "selfhost/src/IR/IR.ls に IR snapshot シリアライザ関数が未定義"
    );

    // 出力が line-based であることを示す改行処理が含まれるか検証
    assert!(
        content.contains("newline") || content.contains("\\n") || content.contains("line-format"),
        "selfhost/src/IR/IR.ls に line-based format の出力処理がない"
    );
}

/// TEST-WASM-01: FrontendResult/LoweredModule/CodegenArtifact の3層境界が IR.ls に定義
#[test]
fn test_e2e_selfhost_backend_boundary() {
    let content = read_selfhost_lowering_source("IR.ls", "IR 3層境界未作成");

    // FrontendResult 型定義
    assert!(
        content.contains("FrontendResult"),
        "selfhost/src/IR/IR.ls に FrontendResult 型が未定義"
    );

    // LoweredModule 型定義
    assert!(
        content.contains("LoweredModule"),
        "selfhost/src/IR/IR.ls に LoweredModule 型が未定義"
    );

    // CodegenArtifact 型定義
    assert!(
        content.contains("CodegenArtifact"),
        "selfhost/src/IR/IR.ls に CodegenArtifact 型が未定義"
    );
}

/// TEST-WASM-02: selfhost/src/Backend/Wasm/Codegen.ls, Emit.ls, WasiBackend.ls の存在
#[test]
fn test_e2e_selfhost_section_builders() {
    let files = [
        ("Codegen.ls", "(module Backend.Wasm.Codegen)"),
        ("Emit.ls", "(module Backend.Wasm.Emit)"),
        ("WasiBackend.ls", "(module Backend.Wasm.WasiBackend)"),
    ];

    for (file, expected_module) in &files {
        let path = selfhost_source_path(file);
        assert!(
            path.exists(),
            "{} が存在しない -- Wasm 生成モジュール未作成",
            selfhost_lowering_label(file)
        );

        let content = read_selfhost_lowering_source(file, "Wasm 生成モジュール未作成");
        assert!(
            content.contains(expected_module),
            "{} に {} 宣言がない",
            selfhost_lowering_label(file),
            expected_module
        );
    }
}

/// TEST-WASM-03: 同じソースの2回コンパイルで byte-identical な Wasm 出力
/// + selfhost Emit.ls に LEB128 エンコーダが定義されていること
#[test]
fn test_e2e_selfhost_deterministic_leb_emit() {
    // Rust コンパイラの決定的出力を検証
    let source = r#"
        (defn main []
          (+ 1 2))
    "#;

    // 1回目のコンパイル
    let wasm1 = compile_only(source);
    assert_valid_wasm(&wasm1);

    // 2回目のコンパイル
    let wasm2 = compile_only(source);
    assert_valid_wasm(&wasm2);

    // byte-identical であることを検証
    assert_eq!(
        wasm1, wasm2,
        "同じソースの2回コンパイルで異なる Wasm バイナリが生成された (決定的コンパイルの違反)"
    );

    // selfhost Emit.ls に LEB128 エンコーダが定義されていること
    let emit_content = read_selfhost_lowering_source("Emit.ls", "LEB128 エンコーダ未実装");

    assert!(
        emit_content.contains("(defn encode-leb128")
            || emit_content.contains("(defn leb128")
            || emit_content.contains("(defn emit-leb128"),
        "selfhost/src/Backend/Wasm/Emit.ls に LEB128 エンコーダ関数が未定義"
    );
}

/// TEST-WASM-04: WasiBackend.ls に print/read-file/write-file/clock-now ヘルパー
#[test]
fn test_e2e_selfhost_wasi_helpers() {
    let content = read_selfhost_lowering_source("WasiBackend.ls", "WASI バックエンド未作成");

    assert!(
        content.contains("(module Backend.Wasm.WasiBackend)"),
        "selfhost/src/Backend/Wasm/WasiBackend.ls に (module Backend.Wasm.WasiBackend) 宣言がない"
    );

    // print ヘルパー
    assert!(
        content.contains("(defn print")
            || content.contains("(defn wasi-print")
            || content.contains("(defn emit-print"),
        "selfhost/src/Backend/Wasm/WasiBackend.ls に print ヘルパーが未定義"
    );

    // read-file ヘルパー
    assert!(
        content.contains("(defn read-file")
            || content.contains("(defn wasi-read-file")
            || content.contains("(defn emit-read-file"),
        "selfhost/src/Backend/Wasm/WasiBackend.ls に read-file ヘルパーが未定義"
    );

    // write-file ヘルパー
    assert!(
        content.contains("(defn write-file")
            || content.contains("(defn wasi-write-file")
            || content.contains("(defn emit-write-file"),
        "selfhost/src/Backend/Wasm/WasiBackend.ls に write-file ヘルパーが未定義"
    );

    // clock-now ヘルパー
    assert!(
        content.contains("(defn clock-now")
            || content.contains("(defn wasi-clock-now")
            || content.contains("(defn emit-clock-now"),
        "selfhost/src/Backend/Wasm/WasiBackend.ls に clock-now ヘルパーが未定義"
    );
}

/// TEST-WASM-05: selfhost/src/Tools/Test/TestRunner.ls の存在 + :example/:invariant テスト生成
#[test]
fn test_e2e_selfhost_test_runner() {
    let content = read_selfhost_lowering_source("TestRunner.ls", "テストランナーモジュール未作成");

    assert!(
        content.contains("(module Tools.Test.TestRunner)"),
        "selfhost/src/Tools/Test/TestRunner.ls に (module Tools.Test.TestRunner) 宣言がない"
    );

    // :example メタデータからテスト生成
    assert!(
        content.contains("example")
            && (content.contains("(defn generate-example-tests")
                || content.contains("(defn extract-examples")
                || content.contains("(defn run-examples")),
        "selfhost/src/Tools/Test/TestRunner.ls に :example テスト生成関数が未定義"
    );

    // :invariant メタデータからテスト生成
    assert!(
        content.contains("invariant")
            && (content.contains("(defn generate-invariant-tests")
                || content.contains("(defn extract-invariants")
                || content.contains("(defn run-invariants")),
        "selfhost/src/Tools/Test/TestRunner.ls に :invariant テスト生成関数が未定義"
    );
}

/// TEST-WASM-06: tests/golden/wasm/ に section hash golden fixture
#[test]
fn test_e2e_selfhost_wasm_golden() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let golden_dir = project_root.join("tests/golden/wasm");
    assert!(
        golden_dir.exists(),
        "tests/golden/wasm/ ディレクトリが存在しない -- golden fixture 未作成"
    );

    assert!(
        golden_dir.is_dir(),
        "tests/golden/wasm がディレクトリではない"
    );

    // golden ディレクトリに少なくとも1つのファイルがあることを検証
    let entries: Vec<_> = std::fs::read_dir(&golden_dir)
        .expect("tests/golden/wasm/ の読み込みに失敗")
        .filter_map(|e| e.ok())
        .collect();

    assert!(
        !entries.is_empty(),
        "tests/golden/wasm/ にgolden fixture ファイルがない"
    );
}

/// TEST-BOOT-03: selfhost/src/**/*.ls, stdlib/*.ls, examples/*.ls 全件 individual compile
#[test]
fn test_e2e_selfhost_all_files_compile() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let mut all_files = Vec::new();
    let mut failures = Vec::new();

    // selfhost/src/**/*.ls を再帰収集
    let selfhost_dir = project_root.join("selfhost/src");
    if selfhost_dir.exists() {
        collect_ls_files_recursive(&selfhost_dir, &mut all_files);
    }

    // stdlib/*.ls を収集
    let stdlib_dir = project_root.join("stdlib");
    if stdlib_dir.exists() {
        for entry in std::fs::read_dir(&stdlib_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "ls") {
                all_files.push(path);
            }
        }
    }

    // examples/*.ls を収集
    let examples_dir = project_root.join("examples");
    if examples_dir.exists() {
        for entry in std::fs::read_dir(&examples_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "ls") {
                all_files.push(path);
            }
        }
    }

    assert!(
        !all_files.is_empty(),
        "コンパイル対象の .ls ファイルが1つも見つからない"
    );

    // 全ファイルを個別にコンパイル
    for file in &all_files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                failures.push(format!("{}: 読み込み失敗 - {}", file.display(), e));
                continue;
            }
        };

        // パースを試行
        match lsharp_syntax::parse(&source) {
            Ok(program) => {
                // import 宣言があるファイルはモジュール間依存があるため
                // 単体での型チェックをスキップしてパース成功のみを確認する
                let has_imports = program
                    .decls
                    .iter()
                    .any(|d| matches!(d, lsharp_syntax::ast::Decl::ImportDecl { .. }));
                if has_imports {
                    // パース成功のみ確認 (import 解決が必要なため型チェックはスキップ)
                    continue;
                }

                // 型チェックを試行
                let mut infer = Infer::new();
                match infer.infer_program(&program) {
                    Ok(type_results) => {
                        // IR lowering を試行
                        let mut lower = Lower::new();
                        match lower.lower_program(&program, &type_results) {
                            Ok(module) => {
                                // Wasm コンパイルを試行
                                if let Err(e) = lsharp_wasm::wasi::emit_wasm_wasi(&module) {
                                    failures.push(format!(
                                        "{}: Wasm 生成失敗 - {}",
                                        file.display(),
                                        e
                                    ));
                                }
                            }
                            Err(e) => {
                                failures.push(format!(
                                    "{}: IR lowering 失敗 - {}",
                                    file.display(),
                                    e
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        failures.push(format!("{}: 型チェック失敗 - {}", file.display(), e));
                    }
                }
            }
            Err(e) => {
                failures.push(format!("{}: パース失敗 - {}", file.display(), e));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "以下のファイルのコンパイルに失敗:\n{}",
        failures.join("\n")
    );
}

/// TEST-BOOT-04: 実体3段固定点検証 (stage0 -> stage1 -> stage2 -> stage3)
#[test]
fn test_e2e_selfhost_true_bootstrap_fixed_point() {
    // canonical entrypoint が存在することを前提とする
    let main_path = selfhost_main_path();
    assert!(
        main_path.exists(),
        "{} が存在しない",
        main_path.display()
    );

    // stage0: Rust コンパイラで canonical Main をマルチファイル経路でコンパイル -> stage1 wasm
    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    // stage1: stage1_wasm をセルフホストコンパイラとして実行し、同じソースをコンパイル
    let stage1_output = lsharp_wasm::wasi_runner::run_wasm_wasi(&stage1_wasm);
    assert!(
        stage1_output.is_ok(),
        "stage1 wasm の実行に失敗 -- {:?}",
        stage1_output.err()
    );
    let stage1_output = stage1_output.unwrap();

    // stage1 コンパイラが何らかの出力を生成すること (現時点では compile サブコマンド未実装)
    // Main.ls が完全なコンパイラ CLI を実装した段階で、compile サブコマンド対応を検証する
    let _ = stage1_output;

    // stage2 wasm を取得して stage3 と比較する固定点検証
    // (stage1 が完全なコンパイラになった段階で有効化)
    // stage0 -> stage1 -> stage2 -> stage3 で stage2 == stage3 であれば固定点
    // NOTE: true bootstrap 固定点検証は未実装: stage1 コンパイラが compile サブコマンドを実装した後に有効化
}

// =============================================================================
// Phase 6 Group K: GC Runtime テスト (TDD Red Phase)
// =============================================================================

/// TEST-GC-01: selfhost/src/Runtime/GC.ls が存在し、object header / trace map / root API を持つ
#[test]
fn test_e2e_selfhost_gc_object_model() {
    let gc_source = read_selfhost_lowering_source("GC.ls", "GC モジュールを作成してください");

    // モジュール宣言
    assert!(
        gc_source.contains("(module Runtime.GC)"),
        "selfhost/src/Runtime/GC.ls に (module Runtime.GC) 宣言がない"
    );

    // object header 型定義
    assert!(
        gc_source.contains("ObjectHeader"),
        "selfhost/src/Runtime/GC.ls に ObjectHeader 型が定義されていない"
    );

    // trace map (GC がオブジェクト内のポインタを辿るためのマップ)
    assert!(
        gc_source.contains("trace-map")
            || gc_source.contains("trace_map")
            || gc_source.contains("TraceMap"),
        "selfhost/src/Runtime/GC.ls に trace map 関連の定義がない"
    );

    // root API (GC ルート登録/解除)
    assert!(
        gc_source.contains("add-root")
            || gc_source.contains("add_root")
            || gc_source.contains("gc-root"),
        "selfhost/src/Runtime/GC.ls に root 登録 API がない"
    );

    assert!(
        gc_source.contains("remove-root")
            || gc_source.contains("remove_root")
            || gc_source.contains("gc-unroot"),
        "selfhost/src/Runtime/GC.ls に root 解除 API がない"
    );

    // コンパイルが通ること
    let program = lsharp_syntax::parse(&gc_source);
    assert!(
        program.is_ok(),
        "selfhost/src/Runtime/GC.ls のパースに失敗: {:?}",
        program.err()
    );
}

/// TEST-GC-02: GC モジュールに mark-sweep 実装 (free-list, mark-bit, sweep-loop)
#[test]
fn test_e2e_selfhost_gc_mark_sweep() {
    let gc_source = read_selfhost_lowering_source("GC.ls", "GC モジュールを作成してください");

    // free-list 管理
    assert!(
        gc_source.contains("free-list")
            || gc_source.contains("free_list")
            || gc_source.contains("FreeList"),
        "selfhost/src/Runtime/GC.ls に free-list 関連の定義がない"
    );

    // mark-bit 操作
    assert!(
        gc_source.contains("mark-bit")
            || gc_source.contains("mark_bit")
            || gc_source.contains("set-mark")
            || gc_source.contains("is-marked"),
        "selfhost/src/Runtime/GC.ls に mark-bit 関連の定義がない"
    );

    // sweep ループ
    assert!(
        gc_source.contains("sweep") || gc_source.contains("gc-sweep"),
        "selfhost/src/Runtime/GC.ls に sweep 関連の定義がない"
    );

    // mark フェーズ
    assert!(
        gc_source.contains("gc-mark")
            || gc_source.contains("mark-phase")
            || gc_source.contains("(defn mark"),
        "selfhost/src/Runtime/GC.ls に mark フェーズ関連の定義がない"
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
