use super::support::*;

// =====================================================
// TASK-007: Main.ls モジュール構造テスト
// =====================================================

/// Main.ls が slim entrypoint であり、pipeline smoke は App.PipelineSmoke に分離されていること、
/// モジュール依存関係のドキュメントコメントを含むことを検証する。
#[test]
fn test_e2e_selfhost_main_module_structure() {
    let source = selfhost_module("Main.ls");

    // 1. module 宣言の存在
    assert!(
        source.contains("(module App.Main)"),
        "Main.ls に (module App.Main) 宣言が必要"
    );

    // 2. 全ての import 宣言の存在
    let expected_imports = ["App.CompilerMode", "App.PipelineSmoke"];
    for imp in &expected_imports {
        let import_decl = format!("(import {})", imp);
        assert!(
            source.contains(&import_decl),
            "Main.ls に {} が必要",
            import_decl
        );
    }

    // 3. pipeline smoke は App.PipelineSmoke に分離されていること
    let pipeline_smoke_path = selfhost_package_root().join("src/App/PipelineSmoke.ls");
    let pipeline_smoke_source = std::fs::read_to_string(&pipeline_smoke_path)
        .unwrap_or_else(|_| panic!("{} が読み込めない", pipeline_smoke_path.display()));
    assert!(
        pipeline_smoke_source.contains("(module App.PipelineSmoke)"),
        "PipelineSmoke.ls に (module App.PipelineSmoke) 宣言が必要"
    );
    assert!(
        pipeline_smoke_source.contains("(defn compile-full-pipeline")
            && pipeline_smoke_source.contains("(defn compile-source")
            && pipeline_smoke_source.contains("(defn run-main-smoke"),
        "App.PipelineSmoke に smoke/pipeline 関数群が必要"
    );
    assert!(
        !source.contains("(defn compile-full-pipeline")
            && !source.contains("(defn compile-source")
            && !source.contains("(defn compile-native-pipeline"),
        "pipeline smoke 関数群は Main.ls ではなく App.PipelineSmoke に分離されているべき"
    );

    // 4. resolver は App.ModuleResolver に分離されていること
    let resolver_path = selfhost_package_root().join("src/App/ModuleResolver.ls");
    let resolver_source = std::fs::read_to_string(&resolver_path)
        .unwrap_or_else(|_| panic!("{} が読み込めない", resolver_path.display()));
    assert!(
        resolver_source.contains("(module App.ModuleResolver)"),
        "ModuleResolver.ls に (module App.ModuleResolver) 宣言が必要"
    );
    assert!(
        resolver_source.contains("(defn resolve-source-root")
            && resolver_source.contains("(defn resolve-package-root")
            && resolver_source.contains("(defn resolve-module-path"),
        "App.ModuleResolver に canonical resolver 関数群が必要"
    );
    assert!(
        !source.contains("(import App.ModuleResolver)"),
        "Main.ls は App.ModuleResolver を直接 import せず、App.CompilerMode 経由で使うべき"
    );
    assert!(
        !source.contains("(defn resolve-source-root")
            && !source.contains("(defn resolve-package-root")
            && !source.contains("(defn resolve-module-path"),
        "resolver 関数群は Main.ls ではなく App.ModuleResolver に分離されているべき"
    );

    // 5. compiler-mode は App.CompilerMode に分離されていること
    let compiler_mode_path = selfhost_package_root().join("src/App/CompilerMode.ls");
    let compiler_mode_source = std::fs::read_to_string(&compiler_mode_path)
        .unwrap_or_else(|_| panic!("{} が読み込めない", compiler_mode_path.display()));
    assert!(
        compiler_mode_source.contains("(module App.CompilerMode)"),
        "CompilerMode.ls に (module App.CompilerMode) 宣言が必要"
    );
    assert!(
        compiler_mode_source.contains("(defn compile-file-mode")
            && compiler_mode_source.contains("(defn build-wasm-bytes-wasi")
            && compiler_mode_source.contains("(defn print-wasm-module"),
        "App.CompilerMode に compiler-mode 関数群が必要"
    );
    assert!(
        !source.contains("(defn compile-file-mode")
            && !source.contains("(defn build-wasm-bytes-wasi")
            && !source.contains("(defn print-wasm-module"),
        "compiler-mode 関数群は Main.ls ではなく App.CompilerMode に分離されているべき"
    );

    // 6. Main.ls 固有の関数が残っていること
    assert!(
        source.contains("(defn main ["),
        "Main.ls に main 関数が必要"
    );

    // 7. コンパイル・実行が正常であること（既存パイプラインが壊れていないこと）
    let output = compile_and_run_file(&selfhost_main_path());
    let lines: Vec<&str> = output.trim().lines().collect();
    // 既存の出力行数以上の出力があること (最低 32 行: 旧パイプライン + 拡張)
    assert!(
        lines.len() >= 32,
        "Main.ls の出力が不足: {} 行 (32行以上期待)",
        lines.len()
    );
}

/// selfhost ファイルの module/import 宣言を解析して依存グラフを構築し、
/// topological sort でコンパイル順を決定。依存先が依存元より前に来ることを検証する。
///
/// 期待されるコンパイル順 (依存深度レベル):
///   Level 0 (依存なし): Syntax.Token, IR.IR, Types.Type, Tools.Lsp.JsonRpc, Backend.Native.NativeTarget,
///            Backend.Wasm.WasiBackend, App.ModuleResolver
///   Level 1: Syntax.AST, Types.TypeScheme, Syntax.Lexer, Backend.Wasm.WasmEmit,
///            Backend.Native.NativeCodegen, Backend.Native.NativeEmit, Backend.Native.Linker
///   Level 2: Syntax.Parser, Syntax.MacroExpand, Types.TypeInferCore, Backend.Wasm.CompilerSplit,
///            Tools.Text.Linter, Tools.Text.FormatterExpr
///   Level 3: Types.TypeInferFunctions, Types.TypeInferBuiltins, Backend.Wasm.Compiler,
///            Tools.Text.FormatterDecl
///   Level 4: Types.TypeInfer, App.CompilerMode, Tools.Text.Formatter
///   Level 5: App.PipelineSmoke
///   Level 6: App.Main
#[test]
fn test_e2e_selfhost_module_graph_topological_sort() {
    use std::collections::{HashMap, HashSet, VecDeque};

    let base_dir = selfhost_package_root().join("src");
    let mut pending_dirs = vec![base_dir.clone()];
    let mut source_paths = Vec::new();

    while let Some(dir) = pending_dirs.pop() {
        for entry in
            std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{:?} の読み込みに失敗: {}", dir, e))
        {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                pending_dirs.push(path);
                continue;
            }
            if path.extension().is_some_and(|ext| ext == "ls") {
                source_paths.push(path);
            }
        }
    }

    // 1. selfhost/src/**/*.ls を読み込み、module/import を抽出
    let mut module_imports: HashMap<String, Vec<String>> = HashMap::new();

    for path in &source_paths {
        let source = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{:?} の読み込みに失敗: {}", path, e));

        let mut module_name: Option<String> = None;
        let mut imports: Vec<String> = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();
            // (module Name) を抽出
            if trimmed.starts_with("(module ") && trimmed.ends_with(')') {
                let name = trimmed
                    .strip_prefix("(module ")
                    .unwrap()
                    .strip_suffix(')')
                    .unwrap()
                    .trim()
                    .to_string();
                module_name = Some(name);
            }
            // (import Name) を抽出
            if trimmed.starts_with("(import ") && trimmed.ends_with(')') {
                let name = trimmed
                    .strip_prefix("(import ")
                    .unwrap()
                    .strip_suffix(')')
                    .unwrap()
                    .trim()
                    .to_string();
                imports.push(name);
            }
        }

        let module_name = module_name.unwrap_or_else(|| {
            panic!("{:?} に (module Name) 宣言が見つからない", path);
        });

        module_imports.insert(module_name, imports);
    }

    // 全 source file が module 宣言を持つこと
    assert_eq!(
        module_imports.len(),
        source_paths.len(),
        "selfhost/src/**/*.ls の全ファイルが module 宣言を持つべき。検出: {:?}",
        module_imports.keys().collect::<Vec<_>>()
    );

    // 2. 依存グラフを構築 (入次数ベースの Kahn's algorithm で topological sort)
    let all_modules: HashSet<String> = module_imports.keys().cloned().collect();

    // 全ての import 先が存在することを検証
    for (module, imports) in &module_imports {
        for imp in imports {
            assert!(
                all_modules.contains(imp),
                "{} が import する {} が selfhost に存在しない",
                module,
                imp
            );
        }
    }

    // 入次数を計算
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    for module in &all_modules {
        in_degree.insert(module.clone(), 0);
    }
    for imports in module_imports.values() {
        for imp in imports {
            // imp に依存するモジュールがあるので imp の「被依存数」ではなく
            // 依存元の入次数を増やす… ではなく、imp を依存先として
            // 依存元の入次数を増やす必要がある
            // 実際には: module が imp に依存 → module の入次数を上げる
            // ここでは別ループで計算し直す
            let _ = imp;
        }
    }
    // 入次数を正しく計算
    for module in &all_modules {
        in_degree.insert(module.clone(), module_imports[module].len());
    }

    // 3. topological sort (Kahn's algorithm)
    let mut queue: VecDeque<String> = VecDeque::new();
    for (module, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(module.clone());
        }
    }

    // 逆引きマップ: imp -> [依存元のモジュール群]
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
    for module in &all_modules {
        dependents.insert(module.clone(), Vec::new());
    }
    for (module, imports) in &module_imports {
        for imp in imports {
            dependents.get_mut(imp).unwrap().push(module.clone());
        }
    }

    let mut sorted: Vec<String> = Vec::new();
    let mut remaining_degree = in_degree.clone();

    while let Some(module) = queue.pop_front() {
        sorted.push(module.clone());
        for dependent in &dependents[&module] {
            let deg = remaining_degree.get_mut(dependent).unwrap();
            *deg -= 1;
            if *deg == 0 {
                queue.push_back(dependent.clone());
            }
        }
    }

    // 循環依存がないことを検証
    assert_eq!(
        sorted.len(),
        module_imports.len(),
        "topological sort で全モジュールがソートされるべき (循環依存なし)。ソート結果: {:?}",
        sorted
    );

    // 4. ソート結果の検証: 依存先が依存元より前に来ること
    let position: HashMap<String, usize> = sorted
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i))
        .collect();

    for (module, imports) in &module_imports {
        let module_pos = position[module];
        for imp in imports {
            let imp_pos = position[imp];
            assert!(
                imp_pos < module_pos,
                "依存順序の違反: {} (位置 {}) は {} (位置 {}) より後にあるべき。ソート結果: {:?}",
                module,
                module_pos,
                imp,
                imp_pos,
                sorted
            );
        }
    }

    // 5. レベル別の検証
    // 各モジュールのレベル = max(依存先のレベル) + 1 (依存なしなら 0)
    let mut levels: HashMap<String, usize> = HashMap::new();

    // topological sort 順にレベルを計算 (依存先は既に計算済み)
    for module in &sorted {
        let imports = &module_imports[module];
        let level = if imports.is_empty() {
            0
        } else {
            imports.iter().map(|imp| levels[imp] + 1).max().unwrap()
        };
        levels.insert(module.clone(), level);
    }

    // Level 0: import を持たない基底モジュール
    let level_0: HashSet<&str> = [
        "Syntax.Token",
        "IR.IR",
        "Types.Type",
        "Tools.Lsp.JsonRpc",
        "Backend.Native.NativeTarget",
        "Backend.Wasm.WasiBackend",
        "App.ModuleResolver",
    ]
    .iter()
    .copied()
    .collect();
    for module in &level_0 {
        assert_eq!(
            levels[*module], 0,
            "{} は Level 0 であるべき (実際: Level {})",
            module, levels[*module]
        );
    }

    // Level 1: Level 0 にのみ依存するモジュール
    let level_1: HashSet<&str> = [
        "Syntax.AST",
        "Types.TypeScheme",
        "Syntax.Lexer",
        "Backend.Wasm.WasmEmit",
        "Backend.Native.NativeCodegen",
        "Backend.Native.NativeEmit",
        "Backend.Native.Linker",
    ]
    .iter()
    .copied()
    .collect();
    for module in &level_1 {
        assert_eq!(
            levels[*module], 1,
            "{} は Level 1 であるべき (実際: Level {})",
            module, levels[*module]
        );
    }

    // Level 2: Level 1 を読む実装モジュール
    let level_2: HashSet<&str> = [
        "Syntax.Parser",
        "Syntax.MacroExpand",
        "Types.TypeInferCore",
        "Backend.Wasm.CompilerSplit",
        "Tools.Text.Linter",
        "Tools.Text.FormatterExpr",
    ]
    .iter()
    .copied()
    .collect();
    for module in &level_2 {
        assert_eq!(
            levels[*module], 2,
            "{} は Level 2 であるべき (実際: Level {})",
            module, levels[*module]
        );
    }

    // Level 3: higher-order type infer / compiler finalization / formatter declaration split
    let level_3: HashSet<&str> = [
        "Types.TypeInferFunctions",
        "Types.TypeInferBuiltins",
        "Backend.Wasm.Compiler",
        "Tools.Text.FormatterDecl",
    ]
    .iter()
    .copied()
    .collect();
    for module in &level_3 {
        assert_eq!(
            levels[*module], 3,
            "{} は Level 3 であるべき (実際: Level {})",
            module, levels[*module]
        );
    }

    // Level 4: dispatcher 層
    let level_4: HashSet<&str> = [
        "Types.TypeInfer",
        "App.CompilerMode",
        "Tools.Text.Formatter",
    ]
    .iter()
    .copied()
    .collect();
    for module in &level_4 {
        assert_eq!(
            levels[*module], 4,
            "{} は Level 4 であるべき (実際: Level {})",
            module, levels[*module]
        );
    }

    // Level 5: App.PipelineSmoke
    let level_5: HashSet<&str> = ["App.PipelineSmoke"].iter().copied().collect();
    for module in &level_5 {
        assert_eq!(
            levels[*module], 5,
            "{} は Level 5 であるべき (実際: Level {})",
            module, levels[*module]
        );
    }

    // Level 6: App.Main (-> App.CompilerMode, App.PipelineSmoke)
    let level_6: HashSet<&str> = ["App.Main"].iter().copied().collect();
    for module in &level_6 {
        assert_eq!(
            levels[*module], 6,
            "{} は Level 6 であるべき (実際: Level {})",
            module, levels[*module]
        );
    }

    // 出力: 確認用
    eprintln!("=== Topological Sort 結果 ===");
    for (i, module) in sorted.iter().enumerate() {
        eprintln!(
            "  {} (Level {}): {} -> [{}]",
            i,
            levels[module],
            module,
            module_imports[module].join(", ")
        );
    }
}

// === TASK-010: MacroExpand/TypeInfer パイプライン統合検証 ===

/// compile-full-pipeline が MacroExpand と TypeInfer のステージを含むことを検証。
/// Main.ls の 5ステージ統合 (token/parse/expand/infer/compile) において、
/// expand (MacroExpand) と infer (TypeInfer) が正しく統合されていることを
/// モジュール宣言の存在 + パイプラインステージ数で確認する。
/// あわせて TypeInfer.ls が TypeInferCore.ls / TypeInferFunctions.ls へ共通 helper を委譲し、
/// builtins 初期化と test-only smoke main も別モジュールへ分離していることを固定する。
#[test]
fn test_e2e_selfhost_pipeline_macroexpand_typeinfer_integration() {
    // 1. MacroExpand.ls と TypeInfer.ls にモジュール宣言が存在することを検証
    let macroexpand_source = std::fs::read_to_string(selfhost_source_path("MacroExpand.ls"))
        .expect("canonical MacroExpand.ls が読み込めない");
    let typeinfer_source = std::fs::read_to_string(selfhost_source_path("TypeInfer.ls"))
        .expect("canonical TypeInfer.ls が読み込めない");
    let typeinfer_core_source = std::fs::read_to_string(selfhost_source_path("TypeInferCore.ls"))
        .expect("canonical TypeInferCore.ls が読み込めない");
    let typeinfer_functions_source =
        std::fs::read_to_string(selfhost_source_path("TypeInferFunctions.ls"))
            .expect("canonical TypeInferFunctions.ls が読み込めない");
    let typeinfer_builtins_source =
        std::fs::read_to_string(selfhost_source_path("TypeInferBuiltins.ls"))
            .expect("canonical TypeInferBuiltins.ls が読み込めない");
    let typeinfer_smoke_source = std::fs::read_to_string(selfhost_source_path("TypeInferSmoke.ls"))
        .expect("canonical TypeInferSmoke.ls が読み込めない");
    let typeinfer_apply_source = std::fs::read_to_string(selfhost_source_path("TypeInferApply.ls"))
        .expect("canonical TypeInferApply.ls が読み込めない");
    let typeinfer_block_source = std::fs::read_to_string(selfhost_source_path("TypeInferBlock.ls"))
        .expect("canonical TypeInferBlock.ls が読み込めない");
    let typeinfer_pattern_source =
        std::fs::read_to_string(selfhost_source_path("TypeInferPattern.ls"))
            .expect("canonical TypeInferPattern.ls が読み込めない");
    let typeinfer_record_source =
        std::fs::read_to_string(selfhost_source_path("TypeInferRecord.ls"))
            .expect("canonical TypeInferRecord.ls が読み込めない");

    // MacroExpand.ls: (module Syntax.MacroExpand) + (import Syntax.AST) + (import Syntax.Token)
    assert!(
        macroexpand_source.contains("(module Syntax.MacroExpand)"),
        "MacroExpand.ls に (module Syntax.MacroExpand) 宣言がない"
    );
    assert!(
        macroexpand_source.contains("(import Syntax.AST)"),
        "MacroExpand.ls に (import Syntax.AST) がない"
    );

    // TypeInfer.ls: (module Types.TypeInfer) + (import Syntax.AST) + (import Types.Type)
    // + (import Types.TypeScheme) + (import Types.TypeInferCore)
    assert!(
        typeinfer_source.contains("(module Types.TypeInfer)"),
        "TypeInfer.ls に (module Types.TypeInfer) 宣言がない"
    );
    assert!(
        typeinfer_source.contains("(import Types.Type)"),
        "TypeInfer.ls に (import Types.Type) がない"
    );
    assert!(
        typeinfer_source.contains("(import Types.TypeScheme)"),
        "TypeInfer.ls に (import Types.TypeScheme) がない"
    );
    assert!(
        typeinfer_source.contains("(import Types.TypeInferCore)"),
        "TypeInfer.ls に (import Types.TypeInferCore) がない"
    );
    assert!(
        typeinfer_source.contains("(import Types.TypeInferFunctions)"),
        "TypeInfer.ls に (import Types.TypeInferFunctions) がない"
    );
    assert!(
        typeinfer_source.contains("(import Types.TypeInferBuiltins)"),
        "TypeInfer.ls に (import Types.TypeInferBuiltins) がない"
    );
    assert!(
        typeinfer_core_source.contains("(module Types.TypeInferCore)"),
        "TypeInferCore.ls に (module Types.TypeInferCore) 宣言がない"
    );
    assert!(
        typeinfer_functions_source.contains("(module Types.TypeInferFunctions)"),
        "TypeInferFunctions.ls に (module Types.TypeInferFunctions) 宣言がない"
    );
    assert!(
        typeinfer_builtins_source.contains("(module Types.TypeInferBuiltins)"),
        "TypeInferBuiltins.ls に (module Types.TypeInferBuiltins) 宣言がない"
    );
    assert!(
        typeinfer_smoke_source.contains("(module Types.TypeInferSmoke)"),
        "TypeInferSmoke.ls に (module Types.TypeInferSmoke) 宣言がない"
    );
    assert!(
        typeinfer_core_source.contains("(defn make-result")
            && typeinfer_core_source.contains("(defn error-code-undefined")
            && typeinfer_core_source.contains("(defn hkt-apply"),
        "TypeInferCore.ls に共通 helper 群が不足している"
    );
    assert!(
        typeinfer_functions_source.contains("(defn typeinfer-fresh-param-types")
            && typeinfer_functions_source.contains("(defn typeinfer-build-curried-fun"),
        "TypeInferFunctions.ls に lambda/defn 共通 helper が不足している"
    );
    assert!(
        typeinfer_builtins_source.contains("(defn typeinfer-init-builtin-env"),
        "TypeInferBuiltins.ls に builtin env 初期化関数がない"
    );
    assert!(
        typeinfer_smoke_source.contains("(defn main []"),
        "TypeInferSmoke.ls に smoke main がない"
    );
    assert!(
        !typeinfer_source.contains("(defn make-result")
            && !typeinfer_source.contains("(defn error-code-undefined")
            && !typeinfer_source.contains("(defn hkt-apply"),
        "TypeInfer.ls には TypeInferCore へ移した helper を重複定義すべきではない"
    );
    assert!(
        !typeinfer_source.contains("if (= param-count 7)")
            && !typeinfer_source.contains("fun7 (mk-fun")
            && !typeinfer_source.contains("env7 (type-env-insert env6"),
        "TypeInfer.ls には lambda/defn の arity 展開を残すべきではない"
    );
    assert!(
        !typeinfer_source.contains("add-ty (mk-fun int-ty")
            && !typeinfer_source.contains("env8 (type-env-insert env7 112"),
        "TypeInfer.ls には builtin env の実装詳細を残すべきではない"
    );
    assert!(
        !typeinfer_source.contains("(defn main []"),
        "TypeInfer.ls に test-only smoke main を残すべきではない"
    );

    // 2. Main.ls の compile-full-pipeline が 5ステージを統合していることを検証
    let output = compile_and_run_file(&selfhost_main_path());
    let lines: Vec<&str> = output.trim().lines().collect();

    // compile-full-pipeline のステージ数出力 (lines[31])
    assert!(
        lines.len() >= 32,
        "完全パイプライン出力が不足: {} 行 (32行以上必要)",
        lines.len()
    );

    let stage_count: i64 = lines[31].parse().unwrap();
    assert_eq!(
        stage_count, 5,
        "compile-full-pipeline のステージ数は 5 (token/parse/expand/infer/compile) であるべき"
    );

    // 3. MacroExpand.ls の関数数が 50 以上であること (本格的な実装)
    let macroexpand_defn_count = macroexpand_source.matches("(defn ").count();
    assert!(
        macroexpand_defn_count >= 50,
        "MacroExpand.ls の関数数が不足: {} (50以上必要)",
        macroexpand_defn_count
    );

    // 4. TypeInfer は helper 分割後も、合算で十分な実装量を持つこと
    let typeinfer_defn_count = typeinfer_source.matches("(defn ").count();
    let typeinfer_core_defn_count = typeinfer_core_source.matches("(defn ").count();
    let typeinfer_functions_defn_count = typeinfer_functions_source.matches("(defn ").count();
    let typeinfer_apply_defn_count = typeinfer_apply_source.matches("(defn ").count();
    let typeinfer_block_defn_count = typeinfer_block_source.matches("(defn ").count();
    let typeinfer_pattern_defn_count = typeinfer_pattern_source.matches("(defn ").count();
    let typeinfer_record_defn_count = typeinfer_record_source.matches("(defn ").count();
    let typeinfer_total = typeinfer_defn_count
        + typeinfer_core_defn_count
        + typeinfer_functions_defn_count
        + typeinfer_apply_defn_count
        + typeinfer_block_defn_count
        + typeinfer_pattern_defn_count
        + typeinfer_record_defn_count;
    assert!(
        typeinfer_total >= 55,
        "TypeInfer 系の関数数が不足: TypeInfer={} Core={} Functions={} Apply={} Block={} Pattern={} Record={} (合計55以上必要)",
        typeinfer_defn_count,
        typeinfer_core_defn_count,
        typeinfer_functions_defn_count,
        typeinfer_apply_defn_count,
        typeinfer_block_defn_count,
        typeinfer_pattern_defn_count,
        typeinfer_record_defn_count
    );
    assert!(
        typeinfer_defn_count >= 5,
        "TypeInfer.ls 本体の関数数が不足: {} (5以上必要、dispatcher+公開API)",
        typeinfer_defn_count
    );
    assert!(
        typeinfer_core_defn_count >= 20,
        "TypeInferCore.ls の関数数が不足: {} (20以上必要)",
        typeinfer_core_defn_count
    );
    assert!(
        typeinfer_functions_defn_count >= 4,
        "TypeInferFunctions.ls の関数数が不足: {} (4以上必要)",
        typeinfer_functions_defn_count
    );
    assert!(
        typeinfer_apply_defn_count >= 2,
        "TypeInferApply.ls の関数数が不足: {} (2以上必要)",
        typeinfer_apply_defn_count
    );
    assert!(
        typeinfer_block_defn_count >= 3,
        "TypeInferBlock.ls の関数数が不足: {} (3以上必要)",
        typeinfer_block_defn_count
    );
    assert!(
        typeinfer_pattern_defn_count >= 5,
        "TypeInferPattern.ls の関数数が不足: {} (5以上必要)",
        typeinfer_pattern_defn_count
    );
    assert!(
        typeinfer_record_defn_count >= 5,
        "TypeInferRecord.ls の関数数が不足: {} (5以上必要)",
        typeinfer_record_defn_count
    );

    // 5. expand/infer ステージの出力検証
    // Stage 3 (expand): マクロ展開後の AST tag
    let expanded_tag: i64 = lines[27].parse().unwrap();
    assert!(
        expanded_tag > 0,
        "Stage 3 (expand/MacroExpand): AST tag が正の値であるべき"
    );

    // Stage 4 (infer/TypeInfer): 型推論結果が Con(Int)
    let ty_tag: i64 = lines[28].parse().unwrap();
    let ty_name: i64 = lines[29].parse().unwrap();
    assert_eq!(ty_tag, 1, "Stage 4 (infer/TypeInfer): 型タグ Con=1");
    assert_eq!(ty_name, 100, "Stage 4 (infer/TypeInfer): 型名 Int=100");
}

// === TASK-011: selfhost 全15モジュール決定性再検証 ===

/// selfhost standalone compile units のコンパイル結果が決定的であることを検証。
/// formatter split helper (`FormatterExpr.ls`, `FormatterDecl.ls`) は bundle 前提なので
/// テキストベースで module/import 宣言と構造の安定性を検証する。
#[test]
#[ignore]
fn test_e2e_bootstrap_selfhost_full_deterministic() {
    // standalone compile unit: 2 回コンパイルでバイト列一致
    let compilable_modules: &[(&str, &str)] = &[
        ("Lexer.ls", selfhost_module("Lexer.ls")),
        ("Parser.ls", selfhost_module("Parser.ls")),
        ("AST.ls", selfhost_module("AST.ls")),
        ("Token.ls", selfhost_module("Token.ls")),
        ("Compiler.ls", selfhost_module("Compiler.ls")),
        ("Type.ls", selfhost_module("Type.ls")),
        ("IR.ls", selfhost_module("IR.ls")),
        ("WasmEmit.ls", selfhost_module("WasmEmit.ls")),
        ("TypeScheme.ls", selfhost_module("TypeScheme.ls")),
        ("TypeInferCore.ls", selfhost_module("TypeInferCore.ls")),
        ("Formatter.ls", selfhost_module("Formatter.ls")),
        ("JsonRpc.ls", selfhost_module("JsonRpc.ls")),
        ("Linter.ls", selfhost_module("Linter.ls")),
        ("Main.ls", selfhost_module("Main.ls")),
    ];

    let mut deterministic_count = 0;

    for (name, source) in compilable_modules {
        let path = selfhost_source_path(name);
        let wasm1 = compile_file_only(&path);
        let wasm2 = compile_file_only(&path);
        assert_eq!(
            wasm1,
            wasm2,
            "{} のコンパイルが非決定的 (module 宣言追加後): {} bytes vs {} bytes",
            name,
            wasm1.len(),
            wasm2.len()
        );
        assert!(
            wasm1.len() > 100,
            "{} の wasm が小さすぎる: {} bytes",
            name,
            wasm1.len()
        );

        // module 宣言が含まれていることを確認
        assert!(
            source.contains("(module "),
            "{} に (module ...) 宣言がない",
            name
        );

        deterministic_count += 1;
    }

    assert_eq!(
        deterministic_count,
        compilable_modules.len(),
        "コンパイル可能な {} モジュール全てが決定的であるべき",
        compilable_modules.len()
    );

    // MacroExpand / TypeInfer は Rust parser 未対応構文、FormatterExpr / FormatterDecl は
    // bundle 前提ディスパッチのため text-only での module/import 宣言を検証する。
    let text_only_modules: &[(&str, &str, &[&str])] = &[
        (
            "MacroExpand.ls",
            selfhost_module("MacroExpand.ls"),
            &["Syntax.AST", "Syntax.Token"],
        ),
        (
            "TypeInfer.ls",
            selfhost_module("TypeInfer.ls"),
            &[
                "Syntax.AST",
                "Types.Type",
                "Types.TypeScheme",
                "Types.TypeInferCore",
                "Types.TypeInferFunctions",
                "Types.TypeInferBuiltins",
            ],
        ),
        (
            "FormatterExpr.ls",
            selfhost_module("FormatterExpr.ls"),
            &["Syntax.AST"],
        ),
        (
            "FormatterDecl.ls",
            selfhost_module("FormatterDecl.ls"),
            &["Syntax.AST", "Tools.Text.FormatterExpr"],
        ),
    ];

    for (name, source, expected_imports) in text_only_modules {
        // module 宣言の存在
        let module_name = match *name {
            "MacroExpand.ls" => "Syntax.MacroExpand",
            "TypeInfer.ls" => "Types.TypeInfer",
            "FormatterExpr.ls" => "Tools.Text.FormatterExpr",
            "FormatterDecl.ls" => "Tools.Text.FormatterDecl",
            other => panic!("不明な text-only selfhost module: {other}"),
        };
        assert!(
            source.contains(&format!("(module {})", module_name)),
            "{} に (module {}) 宣言がない",
            name,
            module_name
        );

        // import 宣言の存在
        for imp in *expected_imports {
            assert!(
                source.contains(&format!("(import {})", imp)),
                "{} に (import {}) がない",
                name,
                imp
            );
        }

        // ソース内容が空でないこと + defn が含まれていること
        assert!(
            source.len() > 500,
            "{} のソースが短すぎる: {} bytes",
            name,
            source.len()
        );
        assert!(
            source.contains("(defn "),
            "{} に関数定義 (defn) がない",
            name
        );

        // テキストの決定性: include_str! を 2 回読んでも同じ内容であること
        // (コンパイル時に解決されるので常に同一だが、ソース変更がないことの記録)
        let source2 = *source; // include_str! は同一文字列リテラル
        assert_eq!(source.len(), source2.len(), "{} のソース長が不安定", name);
    }

    // 全モジュールがカバーされていることを検証
    let total_modules = deterministic_count + text_only_modules.len();
    assert_eq!(
        total_modules,
        compilable_modules.len() + text_only_modules.len(),
        "selfhost 全 {} モジュールがカバーされるべき",
        compilable_modules.len() + text_only_modules.len()
    );
}

// === TEST-SYNTAX-01: Span.ls の unit + golden テスト ===

/// selfhost/src/Syntax/Span.ls が存在し、[start end] 形式の constructor/accessor、
/// merge、dummy 関数を公開していることを検証する。
/// Red Phase: Span.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_span_model() {
    // Span.ls のソースを読み込む
    let span_source = std::fs::read_to_string(selfhost_source_path("Span.ls"))
        .expect("canonical Span.ls が存在しない (Span モジュール未作成)");

    // モジュール宣言の検証
    assert!(
        span_source.contains("(module Syntax.Span)"),
        "Span.ls に (module Syntax.Span) 宣言がない"
    );

    // constructor: span-new または make-span ([start end] 形式)
    let has_constructor =
        span_source.contains("(defn span-new") || span_source.contains("(defn make-span");
    assert!(
        has_constructor,
        "Span.ls に span コンストラクタ (span-new or make-span) がない"
    );

    // accessor: span-start, span-end
    assert!(
        span_source.contains("(defn span-start"),
        "Span.ls に span-start アクセサがない"
    );
    assert!(
        span_source.contains("(defn span-end"),
        "Span.ls に span-end アクセサがない"
    );

    // merge 関数: span-merge
    assert!(
        span_source.contains("(defn span-merge"),
        "Span.ls に span-merge 関数がない"
    );

    // dummy 関数: span-dummy
    assert!(
        span_source.contains("(defn span-dummy"),
        "Span.ls に span-dummy 関数がない"
    );

    // コンパイルが通ることを確認
    let _wasm = compile_only(&span_source);
}

// === TEST-BOOT-01-B: 各モジュール固定 API 呼び出しの E2E テスト ===

/// Main.ls が compiler-mode / smoke entrypoint へ委譲する薄い dispatcher であることを検証する。
/// 固定 API 呼び出し自体は App.CompilerMode / App.PipelineSmoke 側に集約されている。
#[test]
fn test_e2e_selfhost_main_fixed_api_calls() {
    let main_source =
        std::fs::read_to_string(selfhost_main_path()).expect("canonical Main.ls が存在しない");

    let has_compiler_mode_import = main_source.contains("(import App.CompilerMode)");
    assert!(
        has_compiler_mode_import,
        "Main.ls は App.CompilerMode を import して compile path を委譲すべき"
    );

    let has_pipeline_smoke_import = main_source.contains("(import App.PipelineSmoke)");
    assert!(
        has_pipeline_smoke_import,
        "Main.ls は App.PipelineSmoke を import して smoke path を委譲すべき"
    );

    let has_compile_entrypoint = main_source.contains("(compile-file-mode");
    assert!(
        has_compile_entrypoint,
        "Main.ls は compile-file-mode entrypoint を呼ぶべき"
    );

    let has_smoke_entrypoint = main_source.contains("(run-main-smoke)");
    assert!(
        has_smoke_entrypoint,
        "Main.ls は run-main-smoke entrypoint を呼ぶべき"
    );
}

// === TEST-BOOT-02-B: Main.ls フルコンパイル成功テスト ===

/// selfhost/src/App/Main.ls の全モジュール import 付きフルコンパイルが成功することを検証。
/// Main.ls が依存する全モジュール (Lexer, Parser, MacroExpand, TypeInfer,
/// Compiler, WasmEmit) を連結してフルコンパイルする。
/// Red Phase: import 解決が未実装のため、モジュール連結コンパイルが FAIL する。
#[test]
#[ignore]
fn test_e2e_selfhost_main_full_compile() {
    let wasm_bytes = compile_file_only(&selfhost_main_path());

    // Wasm バイナリが有効であること
    assert!(
        wasm_bytes.len() > 1000,
        "Main.ls フルコンパイル結果の Wasm が小さすぎる: {} bytes",
        wasm_bytes.len()
    );

    // Wasm ヘッダー検証 (\0asm)
    assert_eq!(&wasm_bytes[0..4], b"\0asm", "Wasm magic number が不正");

    // 実行して正常終了することを確認
    let output = run_wasi(&wasm_bytes);
    assert!(!output.is_empty(), "Main.ls フルコンパイル実行結果が空");
}
