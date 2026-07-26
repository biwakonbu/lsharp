use super::*;

#[test]
fn test_analyze_single_file_incremental_skips_parse_and_infer_on_clean_cache_hit() {
    let mut cache = CompilationCache::new();
    let source = "(module Main)\n(defn main [] 1)\n";

    analyze_single_file_incremental("lsp://Main", source, &mut cache).unwrap();

    let parse_tracker = IncrementalParseTracker::new();
    let infer_tracker = IncrementalTypeInferTracker::new();
    parse_tracker.reset();
    infer_tracker.reset();

    analyze_single_file_incremental("lsp://Main", source, &mut cache).unwrap();

    assert_eq!(
        parse_tracker.count(),
        0,
        "single-file incremental analysis は clean hit で parse を再実行しないべき"
    );
    assert_eq!(
        infer_tracker.count(),
        0,
        "single-file incremental analysis は clean hit で type infer を再実行しないべき"
    );
}

#[test]
fn test_analyze_single_file_incremental_reparses_and_reinfers_on_source_change() {
    let mut cache = CompilationCache::new();
    analyze_single_file_incremental(
        "lsp://Main",
        "(module Main)\n(defn main [] 1)\n",
        &mut cache,
    )
    .unwrap();

    let parse_tracker = IncrementalParseTracker::new();
    let infer_tracker = IncrementalTypeInferTracker::new();
    parse_tracker.reset();
    infer_tracker.reset();

    analyze_single_file_incremental(
        "lsp://Main",
        "(module Main)\n(defn main [] 2)\n",
        &mut cache,
    )
    .unwrap();

    assert_eq!(
        parse_tracker.count(),
        1,
        "single-file incremental analysis は fingerprint が変わった source を再パースするべき"
    );
    assert_eq!(
        infer_tracker.count(),
        1,
        "single-file incremental analysis は fingerprint が変わった source を再推論するべき"
    );
}

#[test]
fn test_analyze_multi_file_incremental_with_overrides_reports_unsaved_missing_import() {
    use std::collections::HashMap;

    let dir =
        std::env::temp_dir().join("lsharp_analyze_multi_file_incremental_overlay_missing_import");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("Helpers.ls"),
        "(module Helpers)\n(defn helper [] 1)\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Helpers)\n(defn main [] 1)\n",
    )
    .unwrap();

    let mut overrides = HashMap::new();
    overrides.insert(
        dir.join("Main.ls"),
        "(module Main)\n(import Missing)\n(defn main [] 1)\n".to_string(),
    );
    let mut cache = CompilationCache::new();

    let result =
        analyze_multi_file_incremental_with_overrides(&dir.join("Main.ls"), &overrides, &mut cache);

    let _ = std::fs::remove_dir_all(&dir);

    let error = result.expect_err("unsaved import override は missing module error を返すべき");
    assert!(
        error.contains("Missing"),
        "error は unsaved source の import 先 Missing を含むべき: {error}"
    );
}

#[test]
fn test_analyze_multi_file_incremental_with_overrides_isolated_by_entry_root() {
    use std::collections::HashMap;

    let base = std::env::temp_dir().join(format!(
        "lsharp_analyze_multi_file_incremental_overlay_scope_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&base);
    let first = base.join("first");
    let second = base.join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    std::fs::write(first.join("Lib.ls"), "(module Lib)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(
        first.join("Main.ls"),
        "(module Main)\n(import Lib)\n(defn main [] (helper))\n",
    )
    .unwrap();
    std::fs::write(second.join("Main.ls"), "(module Main)\n(defn main [] 42)\n").unwrap();

    let overrides = HashMap::new();
    let mut cache = CompilationCache::new();
    analyze_multi_file_incremental_with_overrides(&first.join("Main.ls"), &overrides, &mut cache)
        .unwrap();
    assert_eq!(
        cache.len(),
        2,
        "first workspace は Main と Lib を cache するべき"
    );

    analyze_multi_file_incremental_with_overrides(&second.join("Main.ls"), &overrides, &mut cache)
        .unwrap();
    assert_eq!(
        cache.len(),
        1,
        "別 workspace の override analysis は stale module を残さないべき"
    );

    std::fs::remove_dir_all(&base).unwrap();
}

#[test]
fn test_analyze_multi_file_incremental_with_overrides_infers_mutual_recursive_scc() {
    use std::collections::HashMap;

    let dir = std::env::temp_dir().join(format!(
        "lsharp_analyze_multi_file_incremental_overlay_scc_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
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

    let mut overrides = HashMap::new();
    overrides.insert(
        dir.join("A.ls"),
        "(module A)\n(import B)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n".to_string(),
    );
    let mut cache = CompilationCache::new();
    let result =
        analyze_multi_file_incremental_with_overrides(&dir.join("Main.ls"), &overrides, &mut cache);

    assert!(
        result.is_ok(),
        "source override analysis も相互再帰 SCC を受理するべき: {result:?}"
    );
    assert_eq!(cache.len(), 3, "SCC 内外の 3 module を cache するべき");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_analyze_multi_file_incremental_with_overrides_reuses_clean_scc_type_surfaces() {
    use std::collections::HashMap;

    let dir = std::env::temp_dir().join(format!(
        "lsharp_analyze_multi_file_incremental_overlay_type_cache_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
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

    let mut overrides = HashMap::new();
    overrides.insert(
        dir.join("A.ls"),
        "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 1 (b-step (- n 1))))\n"
            .to_string(),
    );
    let mut cache = CompilationCache::new();
    let tracker = IncrementalSccInferTracker::new();
    tracker.reset();
    analyze_multi_file_incremental_with_overrides(&dir.join("Main.ls"), &overrides, &mut cache)
        .unwrap();
    assert_eq!(
        tracker.count(),
        3,
        "override の初回分析は Base / A↔B / Main の3 SCCを型推論するべき"
    );

    overrides.insert(
        dir.join("A.ls"),
        "(module A)\n(import B)\n(import Base)\n(defn a-step [n] (if (= n 0) 2 (b-step (- n 1))))\n"
            .to_string(),
    );
    tracker.reset();
    analyze_multi_file_incremental_with_overrides(&dir.join("Main.ls"), &overrides, &mut cache)
        .unwrap();
    assert_eq!(
        tracker.count(),
        1,
        "override で A の実装だけが dirty な場合は A↔B SCC だけを再推論するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_reuses_prefix_module_ir_segments_before_first_dirty_module()
{
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_module_ir_hit");
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
        dir.join("Mid.ls"),
        "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 1))\n",
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
        "warm cache 後は prefix module の IR segment が保存されるべき"
    );

    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 2))\n",
    )
    .unwrap();

    let tracker = IncrementalModuleSegmentLowerTracker::new();
    tracker.reset();
    let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
    let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

    assert_eq!(
        tracker.count(),
        1,
        "tail module だけ dirty な場合は clean prefix module の IR segment を再利用し、fresh lower は dirty suffix のみで済むべき"
    );
    assert_eq!(
        incremental.dump(),
        full.dump(),
        "prefix IR segment reuse 後も final linked IR は full compile と一致するべき"
    );
    assert_eq!(
        incremental.string_data, full.string_data,
        "prefix IR segment reuse 後も string_data は full compile と一致するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_reuses_clean_suffix_module_when_dirty_middle_layout_is_stable()
 {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_suffix_ir_hit");
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
        dir.join("Mid.ls"),
        "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 1))\n",
    )
    .unwrap();

    let mut cache = CompilationCache::new();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    std::fs::write(
        dir.join("Mid.ls"),
        "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 21))\n",
    )
    .unwrap();

    let tracker = IncrementalModuleSegmentLowerTracker::new();
    tracker.reset();
    let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
    let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

    assert_eq!(
        tracker.count(),
        1,
        "dirty middle module が layout 不変なら clean suffix module の IR segment も再利用し、fresh defn lower は dirty module のみで済むべき"
    );
    assert_eq!(
        incremental.dump(),
        full.dump(),
        "clean suffix IR segment reuse 後も final linked IR は full compile と一致するべき"
    );
    assert_eq!(
        incremental.string_data, full.string_data,
        "clean suffix IR segment reuse 後も string_data は full compile と一致するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_reuses_clean_suffix_when_dirty_middle_only_changes_string_state()
 {
    let dir =
        std::env::temp_dir().join("lsharp_compile_multi_file_incremental_suffix_string_state");
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
        dir.join("Mid.ls"),
        "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n(defn mid-label [] \"a\")\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 1))\n",
    )
    .unwrap();

    let mut cache = CompilationCache::new();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    std::fs::write(
        dir.join("Mid.ls"),
        "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n(defn mid-label [] \"alphabet\")\n",
    )
    .unwrap();

    let tracker = IncrementalModuleSegmentLowerTracker::new();
    tracker.reset();
    let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
    let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

    assert_eq!(
        tracker.count(),
        1,
        "dirty middle module の defn string state だけ変わる場合は clean suffix module の defn を再 lower しないべき"
    );
    assert_eq!(
        incremental.dump(),
        full.dump(),
        "suffix defn reuse 後も final linked IR は full compile と一致するべき"
    );
    assert_eq!(
        incremental.string_data, full.string_data,
        "suffix defn reuse 後も string_data は full compile と一致するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_patches_cached_final_link_when_segment_lengths_match() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_link_cache_hit");
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
        dir.join("Mid.ls"),
        "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 20))\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Mid)\n(defn main [] (+ (mid-val) 1))\n",
    )
    .unwrap();

    let mut cache = CompilationCache::new();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    std::fs::write(
        dir.join("Mid.ls"),
        "(module Mid)\n(import Base)\n(defn mid-val [] (+ (base-val) 21))\n",
    )
    .unwrap();

    let tracker = IncrementalLinkTracker::new();
    tracker.reset();
    let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
    let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

    assert_eq!(
        tracker.cache_hit_count(),
        1,
        "module order と segment 長が不変なら cached final module を range patch して full relink を避けるべき"
    );
    assert_eq!(
        tracker.full_count(),
        0,
        "range patch が成立する変更では full relink を再実行しないべき"
    );
    assert_eq!(
        incremental.dump(),
        full.dump(),
        "link cache hit 後も final linked IR は full compile と一致するべき"
    );
    assert_eq!(
        incremental.string_data, full.string_data,
        "link cache hit 後も string_data は full compile と一致するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_invalidates_link_cache_when_segment_lengths_change() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_link_cache_miss");
    if dir.exists() {
        std::fs::remove_dir_all(&dir).unwrap();
    }
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("Mid.ls"),
        "(module Mid)\n(defn mid-val [] \"a\")\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("Main.ls"),
        "(module Main)\n(import Mid)\n(defn main [] (mid-val))\n",
    )
    .unwrap();

    let mut cache = CompilationCache::new();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    std::fs::write(
        dir.join("Mid.ls"),
        "(module Mid)\n(defn mid-val [] (string-concat \"a\" \"b\"))\n",
    )
    .unwrap();

    let tracker = IncrementalLinkTracker::new();
    tracker.reset();
    let incremental = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();
    let full = compile_multi_file(&dir.join("Main.ls")).unwrap();

    assert_eq!(
        tracker.cache_hit_count(),
        0,
        "string_data segment 長が変わる変更では cached final module patch は使わないべき"
    );
    assert_eq!(
        tracker.full_count(),
        1,
        "segment 長が変わる変更では full relink にフォールバックするべき"
    );
    assert_eq!(
        incremental.dump(),
        full.dump(),
        "link cache miss 後も final linked IR は full compile と一致するべき"
    );
    assert_eq!(
        incremental.string_data, full.string_data,
        "link cache miss 後も string_data は full compile と一致するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_skips_dependent_reinfer_when_surface_unchanged() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_infer_impl_change");
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
    std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] 8)\n").unwrap();

    let tracker = IncrementalTypeInferTracker::new();
    tracker.reset();
    compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache).unwrap();

    assert_eq!(
        tracker.count(),
        1,
        "依存先の実装変更で型サーフェスが不変なら dependency のみ再型推論し、dependent は再利用するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_reinfers_on_dependency_signature_change() {
    let dir = std::env::temp_dir().join("lsharp_compile_multi_file_incremental_infer_sig_change");
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
    std::fs::write(dir.join("Lib.ls"), "(module Lib)\n(defn helper [] true)\n").unwrap();

    let tracker = IncrementalTypeInferTracker::new();
    tracker.reset();
    let result = compile_multi_file_incremental(&dir.join("Main.ls"), &mut cache);

    assert!(
        result.is_err(),
        "依存先シグネチャ変更で不整合になれば compile は失敗するべき"
    );
    assert_eq!(
        tracker.count(),
        2,
        "依存先シグネチャ変更では dependency + dependent を再型推論するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds() {
    let cli_path =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../selfhost/src/App/Cli.ls");
    let mut cache = CompilationCache::new();

    compile_multi_file_incremental(&cli_path, &mut cache)
        .expect("first incremental compile of selfhost Cli.ls should succeed");
    let second = compile_multi_file_incremental(&cli_path, &mut cache);

    assert!(
        second.is_ok(),
        "clean rebuild with formatter trio cache should not fail: {second:?}"
    );
}

#[test]
fn test_formatter_modules_declare_cross_module_dispatch_imports() {
    let source_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../selfhost/src/Tools/Text");
    let expr = std::fs::read_to_string(source_root.join("FormatterExpr.ls")).unwrap();
    let decl = std::fs::read_to_string(source_root.join("FormatterDecl.ls")).unwrap();

    assert!(
        expr.lines()
            .any(|line| line.trim() == "(import Tools.Text.Formatter)"),
        "FormatterExpr は dispatch 関数の提供元を明示 import するべき"
    );
    assert!(
        decl.lines()
            .any(|line| line.trim() == "(import Tools.Text.Formatter)"),
        "FormatterDecl は dispatch 関数の提供元を明示 import するべき"
    );
}
