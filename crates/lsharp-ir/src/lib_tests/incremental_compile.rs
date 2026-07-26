use super::*;

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
fn test_compile_multi_file_incremental_empty_cache_matches_full_compile() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_empty_cache");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    let lib_source = "(module Lib)\n(defn helper [] 7)\n";
    let main_source = "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n";
    std::fs::write(dir.join("Lib.ls"), lib_source).unwrap();
    std::fs::write(dir.join("Main.ls"), main_source).unwrap();

    let full = compile_multi_file(&dir.join("Main.ls")).unwrap();
    let mut cache = CompilationCache::new();
    let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
    let main_entry = cache.get("Main").expect("Main module should be cached");

    assert_eq!(
        full.dump(),
        incremental.dump(),
        "空キャッシュ初回コンパイルは既存のフルコンパイルと同一結果になるべき"
    );
    assert_eq!(
        cache.len(),
        2,
        "初回 incremental compile は通過したモジュールを cache に記録するべき"
    );
    assert!(
        main_entry.type_result_len() > 0,
        "cache entry は型サーフェスも保持するべき"
    );
    assert_eq!(
        main_entry.fingerprint(),
        SourceFingerprint::from_source(main_source),
        "cache entry は読み込んだソースの fingerprint を保持するべき"
    );
    assert_eq!(
        main_entry.imports(),
        ["Lib"],
        "cache entry は direct import module 名を保持するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_with_cache_matches_fresh_and_warm_compile() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_compile_multi_file_with_cache_api_{}",
        std::process::id()
    ));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    let lib_source = "(module Lib)\n(defn helper [] 7)\n";
    let main_source = "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n";
    std::fs::write(dir.join("Lib.ls"), lib_source).unwrap();
    std::fs::write(dir.join("Main.ls"), main_source).unwrap();

    let fresh = compile_multi_file(&dir.join("Main.ls")).unwrap();
    let mut cache = CompilationCache::new();
    let tracker = IncrementalTypeInferTracker::new();
    let cold = compile_multi_file_with_cache(&dir.join("Main.ls"), &mut cache).unwrap();
    assert_eq!(fresh.dump(), cold.dump());
    assert_eq!(cache.len(), 2);

    tracker.reset();
    let warm = compile_multi_file_with_cache(&dir.join("Main.ls"), &mut cache).unwrap();
    assert_eq!(cold.dump(), warm.dump());
    assert_eq!(
        tracker.count(),
        0,
        "warm cache compile は再型推論しないべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_with_cache_isolated_by_entry_root() {
    let base = std::env::temp_dir().join(format!(
        "lsharp_compile_multi_file_with_cache_scope_{}",
        std::process::id()
    ));
    if base.exists() {
        std::fs::remove_dir_all(&base).unwrap();
    }
    let first = base.join("first");
    let second = base.join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();

    std::fs::write(first.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(
        first.join("Main.ls"),
        "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
    )
    .unwrap();
    std::fs::write(second.join("Main.ls"), "(module Main)\n(defn main [] 42)\n").unwrap();

    let mut cache = CompilationCache::new();
    compile_multi_file_with_cache(&first.join("Main.ls"), &mut cache).unwrap();
    assert_eq!(
        cache.len(),
        2,
        "first project は Main と Lib を cache するべき"
    );

    let second_module = compile_multi_file_with_cache(&second.join("Main.ls"), &mut cache)
        .expect("entry root が変わっても compile できるべき");
    assert!(matches!(
        main_function(&second_module).body.as_slice(),
        [Instruction::I64Const(42)]
    ));
    assert_eq!(
        cache.len(),
        1,
        "別 project の stale module は cache に残さないべき"
    );

    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn test_compile_multi_file_with_cache_tracks_dependency_surface_key() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_compile_multi_file_with_cache_dependency_key_{}",
        std::process::id()
    ));
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();
    let lib_path = dir.join("Lib.ls");
    let main_path = dir.join("Main.ls");
    std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(
        &main_path,
        "(module Main)\n(import Lib)\n(defn main [] (helper))\n",
    )
    .unwrap();

    let mut cache = CompilationCache::new();
    compile_multi_file_with_cache(&main_path, &mut cache).unwrap();
    let initial_key = cache.get("Main").unwrap().deps_key();

    std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 8)\n").unwrap();
    compile_multi_file_with_cache(&main_path, &mut cache).unwrap();
    let implementation_only_key = cache.get("Main").unwrap().deps_key();
    assert_eq!(
        initial_key, implementation_only_key,
        "依存 module の実装だけが変わった場合、公開型 key は維持するべき"
    );

    std::fs::write(&lib_path, "(module Lib)\n(defn helper [] true)\n").unwrap();
    compile_multi_file_with_cache(&main_path, &mut cache).unwrap();
    let surface_changed_key = cache.get("Main").unwrap().deps_key();
    assert_ne!(
        implementation_only_key, surface_changed_key,
        "依存 module の公開型が変わった場合、依存 key も変わるべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_roots_local_generic_closure_result_argument() {
    let dir = std::env::temp_dir()
        .join("lsharp_compile_multi_file_incremental_closure_generic_result_rooting");
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

    let full = compile_multi_file(&dir.join("Main.ls")).unwrap();
    let mut cache = CompilationCache::new();
    let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    assert_eq!(
        call_positions(&main_function(&full).body, 14).len(),
        4,
        "full multi-file compile は local generic closure result を使う closure call で outer arg 用まで root_push するべき: {:?}",
        main_function(&full).body
    );
    assert_eq!(
        call_positions(&main_function(&incremental).body, 14).len(),
        4,
        "incremental multi-file compile も expr-type cache を通して outer arg 用まで root_push するべき: {:?}",
        main_function(&incremental).body
    );
    assert_eq!(
        full.dump(),
        incremental.dump(),
        "incremental multi-file compile も expr-type table を含めて full compile と同一 IR を維持するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_skips_parse_on_cache_hit() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_parse_cache_hit");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    let lib_source = "(module Lib)\n(defn helper [] 7)\n";
    let main_source = "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n";
    std::fs::write(dir.join("Lib.ls"), lib_source).unwrap();
    std::fs::write(dir.join("Main.ls"), main_source).unwrap();

    let mut cache = CompilationCache::new();
    let tracker = IncrementalParseTracker::new();
    tracker.reset();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    tracker.reset();
    cached_program_or_parse(
        "Lib",
        lib_source,
        SourceFingerprint::from_source(lib_source),
        &cache,
    )
    .unwrap();
    cached_program_or_parse(
        "Main",
        main_source,
        SourceFingerprint::from_source(main_source),
        &cache,
    )
    .unwrap();
    assert_eq!(
        tracker.count(),
        0,
        "事前確認として cache helper 単体では両モジュールとも hit するべき"
    );

    tracker.reset();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    assert_eq!(
        tracker.count(),
        0,
        "fingerprint が不変な再コンパイルでは AST cache hit により parse をスキップするべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_reparses_only_changed_module() {
    let dir =
        std::env::temp_dir().join("lsharp_compile_multi_file_incremental_parse_single_change");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
    )
    .unwrap();

    let mut cache = CompilationCache::new();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 2))\n",
    )
    .unwrap();

    let tracker = IncrementalParseTracker::new();
    tracker.reset();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    assert_eq!(
        tracker.count(),
        1,
        "1 モジュールだけ fingerprint が変わった場合はその AST だけ再パースするべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_reuses_cached_ast_arc_on_cache_hit() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_ast_arc_hit");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    let main_source = "(module Main)\n(defn main [] 1)\n";
    std::fs::write(dir.join("Main.ls"), main_source).unwrap();

    let mut cache = CompilationCache::new();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    let cached = cache
        .get("Main")
        .expect("Main module should be cached")
        .ast_arc();
    let reused = cached_program_or_parse(
        "Main",
        main_source,
        SourceFingerprint::from_source(main_source),
        &cache,
    )
    .unwrap();

    assert!(
        std::sync::Arc::ptr_eq(&cached, &reused),
        "AST cache hit では同じ Arc<Program> を再利用するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_skips_type_inference_on_clean_cache_hit() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_infer_cache_hit");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
    )
    .unwrap();

    let mut cache = CompilationCache::new();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    let tracker = IncrementalTypeInferTracker::new();
    tracker.reset();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    assert_eq!(
        tracker.count(),
        0,
        "dirty set が空なら cached ModuleTypeSurface を再利用して型推論をスキップするべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_skips_ir_generation_on_clean_cache_hit() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_ir_cache_hit");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
    )
    .unwrap();

    let mut cache = CompilationCache::new();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    let tracker = IncrementalLowerTracker::new();
    tracker.reset();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    assert_eq!(
        tracker.count(),
        0,
        "dirty set が空なら cached IR を再利用して lowering をスキップするべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}
