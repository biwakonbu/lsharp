use super::*;

#[test]
fn test_merged_scc_declarations_deduplicate_identical_imports() {
    let module_a = lsharp_syntax::parse("(module A) (import Shared) (import Shared) (defn a [] 1)")
        .expect("module A should parse");
    let module_b = lsharp_syntax::parse("(module B) (import Shared) (defn b [] 2)")
        .expect("module B should parse");
    let parsed_modules = HashMap::from([("A".to_string(), module_a), ("B".to_string(), module_b)]);
    let group = vec!["A".to_string(), "B".to_string()];

    let (merged_decls, defn_origins) =
        merge_scc_declarations(&group, &parsed_modules).expect("SCC declarations should merge");
    let imports = merged_decls
        .iter()
        .filter_map(|decl| match decl {
            lsharp_syntax::ast::Decl::ImportDecl { module, .. } => Some(module.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(imports, vec!["Shared"]);
    assert_eq!(defn_origins, vec!["A", "B"]);
}

#[test]
fn test_merged_scc_declarations_keep_distinct_import_visibility() {
    let module_a = lsharp_syntax::parse("(module A) (import Shared :only [x]) (defn a [] 1)")
        .expect("module A should parse");
    let module_b = lsharp_syntax::parse("(module B) (import Shared :only [y]) (defn b [] 2)")
        .expect("module B should parse");
    let parsed_modules = HashMap::from([("A".to_string(), module_a), ("B".to_string(), module_b)]);
    let group = vec!["A".to_string(), "B".to_string()];

    let (merged_decls, _) =
        merge_scc_declarations(&group, &parsed_modules).expect("SCC declarations should merge");
    let imports = merged_decls
        .iter()
        .filter_map(|decl| match decl {
            lsharp_syntax::ast::Decl::ImportDecl { only, .. } => only.clone(),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(imports, vec![vec!["x".to_string()], vec!["y".to_string()]]);
}

fn main_function(module: &Module) -> &Function {
    module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main function should exist")
}

fn call_positions(body: &[Instruction], target: u32) -> Vec<usize> {
    body.iter()
        .enumerate()
        .filter_map(|(idx, instr)| match instr {
            Instruction::Call(actual) if *actual == target => Some(idx),
            _ => None,
        })
        .collect()
}

#[test]
fn test_compile_multi_file_injects_only_dependency_closure() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_dependency_closure");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(dir.join("A.ls"), "(module A)\n(defn status [x] x)\n").unwrap();
    std::fs::write(
        dir.join("Noise.ls"),
        "(module Noise)\n(defn status [x] true)\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("ZConsumer.ls"),
        "(module ZConsumer)\n(import A)\n(defn check [x] (= (status x) 1))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import A)\n(import Noise)\n(import ZConsumer)\n(defn main [] (if (check 1) 1 0))\n",
    )
    .unwrap();

    let result = compile_multi_file(&dir.join("Main.ls"));
    assert!(
        result.is_ok(),
        "unrelated sibling module types should not pollute dependency inference: {result:?}"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_import_only_blocks_non_selected_symbol() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_import_only_blocks");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("Utils.ls"),
        "(module Utils)\n(defn helper [] 1)\n(defn secret [] 2)\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Utils :only [helper])\n(defn main [] (secret))\n",
    )
    .unwrap();

    let result = compile_multi_file(&dir.join("Main.ls"));
    assert!(
        result.is_err(),
        ":only で除外されたシンボルは compile でも参照できないべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_infers_mutual_recursive_scc() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_compile_multi_file_mutual_recursive_scc_{}",
        std::process::id()
    ));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("A.ls"),
        "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("B.ls"),
        "(module B)\n(import A)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
    )
    .unwrap();

    let result = compile_multi_file(&dir.join("Main.ls"));
    assert!(
        result.is_ok(),
        "相互再帰 SCC はモジュール単位の循環エラーではなく一括推論へ進めるべき: {result:?}"
    );
    let module = result.unwrap();
    assert!(
        module
            .functions
            .iter()
            .any(|function| function.name == "a-step")
    );
    assert!(
        module
            .functions
            .iter()
            .any(|function| function.name == "b-step")
    );
    assert!(
        module
            .functions
            .iter()
            .any(|function| function.name == "main")
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_unrestricted_scc_uses_merged_surface_fast_path() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_compile_multi_file_unrestricted_scc_fast_path_{}",
        std::process::id()
    ));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("A.ls"),
        "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("B.ls"),
        "(module B)\n(import A)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
    )
    .unwrap();

    let tracker = IncrementalSccMergedFastPathTracker::new();
    tracker.reset();
    let result = compile_multi_file(&dir.join("Main.ls"));
    assert!(
        result.is_ok(),
        "公開 import の SCC は compile できるべき: {result:?}"
    );
    assert_eq!(
        tracker.count(),
        1,
        "可視性制約のない A↔B SCC は merged inference の surface を再検証なしで利用するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_infers_mutual_recursive_scc() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_compile_multi_file_incremental_mutual_recursive_scc_{}",
        std::process::id()
    ));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("A.ls"),
        "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("B.ls"),
        "(module B)\n(import A)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
    )
    .unwrap();

    let mut cache = CompilationCache::new();
    let type_tracker = IncrementalTypeInferTracker::new();
    type_tracker.reset();
    let first = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache);
    assert!(
        first.is_ok(),
        "incremental compile も相互再帰 SCC を受理するべき: {first:?}"
    );
    assert_eq!(
        type_tracker.count(),
        1,
        "singleton SCC は module-local inference を 1 回だけ実行するべき"
    );
    drop(type_tracker);
    let first = first.unwrap();
    let tracker = IncrementalSccInferTracker::new();
    tracker.reset();
    let second = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
    assert_eq!(
        tracker.count(),
        0,
        "SCC の clean rebuild は module inference を再実行しないべき"
    );
    assert_eq!(
        first.dump(),
        second.dump(),
        "SCC の clean rebuild は同じ linked IR を返すべき"
    );

    std::fs::write(
        dir.join("A.ls"),
        "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 2 (b-step (- n 1))))\n",
    )
    .unwrap();
    let tracker = IncrementalSccInferTracker::new();
    tracker.reset();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
    assert!(
        tracker.count() > 0,
        "SCC の dirty rebuild は型推論を再実行するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_scc_reuses_clean_ir_segments_after_dirty_module() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_compile_multi_file_incremental_scc_segments_{}",
        std::process::id()
    ));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("Base.ls"),
        "(module Base)\n(defn base-val [] 10)\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("A.ls"),
        "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("B.ls"),
        "(module B)\n(import A)\n(import Base)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
    )
    .unwrap();

    let mut cache = CompilationCache::new();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
    assert!(
        !cache
            .get("Base")
            .expect("Base module should be cached")
            .ir_segments()
            .is_empty(),
        "SCC compile 後も独立した module の IR segment を cache するべき"
    );

    std::fs::write(
        dir.join("A.ls"),
        "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 2 (b-step (- n 1))))\n",
    )
    .unwrap();

    let tracker = IncrementalModuleSegmentLowerTracker::new();
    tracker.reset();
    let link_tracker = IncrementalLinkTracker::new();
    link_tracker.reset();
    let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
    let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

    assert_eq!(
        tracker.count(),
        1,
        "SCC 内の A だけが dirty なら clean module の segment を再利用し、fresh lower は A のみにするべき"
    );
    assert_eq!(
        link_tracker.cache_hit_count(),
        1,
        "SCC の segment 長が不変なら cached final module を range patch するべき"
    );
    assert_eq!(
        link_tracker.full_count(),
        0,
        "SCC の segment 長が不変なら full relink を再実行しないべき"
    );
    assert_eq!(
        incremental.dump(),
        full.dump(),
        "SCC の dirty segment reuse 後も final linked IR は full compile と一致するべき"
    );
    assert_eq!(
        incremental.string_data, full.string_data,
        "SCC の dirty segment reuse 後も string_data は full compile と一致するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_scc_reuses_clean_type_surfaces_after_impl_change() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_compile_multi_file_incremental_scc_type_cache_{}",
        std::process::id()
    ));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("Base.ls"),
        "(module Base)\n(defn base-val [] 10)\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("A.ls"),
        "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("B.ls"),
        "(module B)\n(import A)\n(import Base)\n(defn b-step [n] (if (= n 0) 0 (a-step (- n 1))))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import A)\n(defn main [] (a-step 4))\n",
    )
    .unwrap();

    let mut cache = CompilationCache::new();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    std::fs::write(
        dir.join("A.ls"),
        "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 2 (b-step (- n 1))))\n",
    )
    .unwrap();

    let tracker = IncrementalSccInferTracker::new();
    tracker.reset();
    let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
    let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

    assert_eq!(
        tracker.count(),
        1,
        "A の実装だけが dirty な場合は Base/Main の clean SCC を再推論せず、A↔B SCC だけを再推論するべき"
    );
    assert_eq!(
        incremental.dump(),
        full.dump(),
        "SCC type surface reuse 後も final linked IR は full compile と一致するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_scc_preserves_import_only_visibility() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_compile_multi_file_scc_import_only_{}",
        std::process::id()
    ));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("A.ls"),
        "(module A)\n(import B :only [b-step])\n(defn a-step [] (secret))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("B.ls"),
        "(module B)\n(import A)\n(defn b-step [] (a-step))\n(defn secret [] 2)\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import A)\n(defn main [] (a-step))\n",
    )
    .unwrap();

    let result = compile_multi_file(&dir.join("Main.ls"));
    assert!(
        result.is_err(),
        "SCC 内でも import :only の境界を越えてはならない"
    );
    let error = result.unwrap_err();
    assert!(
        error.contains("secret"),
        "診断に拒否された symbol を含めるべき: {error}"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_private_import_blocks_symbol() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_private_blocks");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("Utils.ls"),
        "(module Utils)\n(private (defn secret [] 2))\n(defn helper [] 1)\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Utils)\n(defn main [] (secret))\n",
    )
    .unwrap();

    let result = compile_multi_file(&dir.join("Main.ls"));
    assert!(
        result.is_err(),
        "private なシンボルは compile でも参照できないべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_modular_lowering_matches_merged_reference_with_strings() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_modular_matches_merged");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("Lib.ls"),
        "(module Lib)\n(defn helper [] \"lib\")\n(defn helper2 [] \"++\")\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Suffix.ls"),
        "(module Suffix)\n(defn bang [] \"!\")\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Lib)\n(import Suffix)\n(defn main [] (string-concat (string-concat (helper) (helper2)) (bang)))\n",
    )
    .unwrap();

    let merged =
        compile_multi_file_with_mode(&dir.join("Main.ls"), MultiFileLoweringMode::Merged).unwrap();
    let modular =
        compile_multi_file_with_mode(&dir.join("Main.ls"), MultiFileLoweringMode::Modular).unwrap();

    assert_eq!(
        merged.dump(),
        modular.dump(),
        "module-local lowering は merged lowering と同じ関数順序・命令列を維持するべき"
    );
    assert_eq!(
        merged.string_data, modular.string_data,
        "module-local lowering は merged lowering と同じ string_data 配列を維持するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_closure_call_roots_local_generic_result_argument() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_closure_generic_result_rooting");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("Lib.ls"),
        "(module Lib)\n(defn make-show [] (fn [s] (string-length s)))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Lib)\n(defn main [] (let [id (fn [x] x) f (make-show)] (f (id \"hello\"))))\n",
    )
    .unwrap();

    let merged =
        compile_multi_file_with_mode(&dir.join("Main.ls"), MultiFileLoweringMode::Merged).unwrap();
    let modular =
        compile_multi_file_with_mode(&dir.join("Main.ls"), MultiFileLoweringMode::Modular).unwrap();

    assert_eq!(
        call_positions(&main_function(&merged).body, 14).len(),
        4,
        "multi-file merged lowering でも local generic closure result を使う closure call は outer arg 用まで root_push するべき: {:?}",
        main_function(&merged).body
    );
    assert_eq!(
        call_positions(&main_function(&modular).body, 14).len(),
        4,
        "multi-file modular lowering でも local generic closure result を使う closure call は outer arg 用まで root_push するべき: {:?}",
        main_function(&modular).body
    );
    assert_eq!(
        merged.dump(),
        modular.dump(),
        "expr-type table を通した modular lowering も merged lowering と同一 IR を維持するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}
