use super::support::{compile_and_run_with_dir, selfhost_module};

fn module_resolver_runtime_source(harness: &str) -> String {
    format!("{}\n{}", selfhost_module("ModuleResolver.ls"), harness)
}

/// TEST-SYNTAX-02c8: ModuleResolver は local nested path を clean hit で再解決しない
#[test]
fn test_e2e_selfhost_module_resolver_cache_hits_local_nested_path() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_selfhost_module_resolver_cache_local_{}",
        std::process::id()
    ));
    let app_dir = dir.join("src/App");
    std::fs::create_dir_all(&app_dir).unwrap();
    std::fs::write(
        app_dir.join("Lib.ls"),
        "(module App.Lib)\n(defn helper [] 7)\n",
    )
    .unwrap();

    let harness = r#"
(defn main []
  (let [cache-ref (ref-new (map-new))
        resolve-count-ref (ref-new 0)
        path1 (resolve-module-path-with-cache-counted "App.Lib" "src" "." cache-ref resolve-count-ref)
        count1 (ref-get resolve-count-ref)
        path2 (resolve-module-path-with-cache-counted "App.Lib" "src" "." cache-ref resolve-count-ref)
        count2 (ref-get resolve-count-ref)]
    (do
      (print (if (text-eq path1 "src/App/Lib.ls") 1 0))
      (print (if (text-eq path2 "src/App/Lib.ls") 1 0))
      (print count1)
      (print count2)
      0)))
"#;

    let output = compile_and_run_with_dir(&module_resolver_runtime_source(harness), &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "module resolver local cache 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "local nested path を返すべき");
    assert_eq!(lines[1], "1", "cache hit でも同じ path を返すべき");
    assert_eq!(lines[2], "1", "初回 resolve では count が 1 になるべき");
    assert_eq!(lines[3], "1", "clean hit では resolve-count が増えないべき");
}

/// TEST-SYNTAX-02c9: ModuleResolver は module-index path を clean hit で再解決しない
#[test]
fn test_e2e_selfhost_module_resolver_cache_hits_module_index_path() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_selfhost_module_resolver_cache_index_{}",
        std::process::id()
    ));
    let index_dir = dir.join(".lsharp/module-index");
    let vendor_dir = dir.join("vendor");
    std::fs::create_dir_all(&index_dir).unwrap();
    std::fs::create_dir_all(&vendor_dir).unwrap();
    std::fs::write(index_dir.join("Geometry.path"), "vendor/Geometry.ls\n").unwrap();
    std::fs::write(
        vendor_dir.join("Geometry.ls"),
        "(module Geometry)\n(defn area [] 42)\n",
    )
    .unwrap();

    let harness = r#"
(defn main []
  (let [cache-ref (ref-new (map-new))
        resolve-count-ref (ref-new 0)
        path1 (resolve-module-path-with-cache-counted "Geometry" "src" "." cache-ref resolve-count-ref)
        count1 (ref-get resolve-count-ref)
        path2 (resolve-module-path-with-cache-counted "Geometry" "src" "." cache-ref resolve-count-ref)
        count2 (ref-get resolve-count-ref)]
    (do
      (print (if (text-eq path1 "vendor/Geometry.ls") 1 0))
      (print (if (text-eq path2 "vendor/Geometry.ls") 1 0))
      (print count1)
      (print count2)
      0)))
"#;

    let output = compile_and_run_with_dir(&module_resolver_runtime_source(harness), &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "module resolver index cache 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "module-index target を返すべき");
    assert_eq!(lines[1], "1", "cache hit でも同じ index target を返すべき");
    assert_eq!(lines[2], "1", "初回 resolve では count が 1 になるべき");
    assert_eq!(lines[3], "1", "clean hit では resolve-count が増えないべき");
}

/// TEST-SYNTAX-02ca: ModuleResolver は local source が現れたら cache を invalidation すること
#[test]
fn test_e2e_selfhost_module_resolver_cache_invalidates_when_local_source_appears() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_selfhost_module_resolver_cache_local_invalidation_{}",
        std::process::id()
    ));
    let app_dir = dir.join("src/App");
    let index_dir = dir.join(".lsharp/module-index/App");
    let vendor_dir = dir.join("vendor/App");
    std::fs::create_dir_all(&app_dir).unwrap();
    std::fs::create_dir_all(&index_dir).unwrap();
    std::fs::create_dir_all(&vendor_dir).unwrap();
    std::fs::write(index_dir.join("Lib.path"), "vendor/App/Lib.ls").unwrap();
    std::fs::write(
        vendor_dir.join("Lib.ls"),
        "(module App.Lib) (defn helper [] 7)",
    )
    .unwrap();

    let harness = r#"
(defn main []
  (let [cache-ref (ref-new (map-new))
        resolve-count-ref (ref-new 0)
        path1 (resolve-module-path-with-cache-counted "App.Lib" "src" "." cache-ref resolve-count-ref)
        count1 (ref-get resolve-count-ref)
        _ (write-file "src/App/Lib.ls" "(module App.Lib) (defn helper [] 9)")
        path2 (resolve-module-path-with-cache-counted "App.Lib" "src" "." cache-ref resolve-count-ref)
        count2 (ref-get resolve-count-ref)
        path3 (resolve-module-path-with-cache-counted "App.Lib" "src" "." cache-ref resolve-count-ref)
        count3 (ref-get resolve-count-ref)]
    (do
      (print (if (text-eq path1 "vendor/App/Lib.ls") 1 0))
      (print count1)
      (print (if (text-eq path2 "src/App/Lib.ls") 1 0))
      (print count2)
      (print (if (text-eq path3 "src/App/Lib.ls") 1 0))
      (print count3)
      0)))
"#;

    let output = compile_and_run_with_dir(&module_resolver_runtime_source(harness), &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "module resolver local invalidation 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "初回は module-index target を返すべき");
    assert_eq!(lines[1], "1", "初回 resolve では count が 1 になるべき");
    assert_eq!(lines[2], "1", "local source 出現後は local path を返すべき");
    assert_eq!(
        lines[3], "2",
        "invalidation 後の resolve で count が増えるべき"
    );
    assert_eq!(
        lines[4], "1",
        "更新後の clean hit でも local path を維持するべき"
    );
    assert_eq!(
        lines[5], "2",
        "mutation がなければ resolve-count は増えないべき"
    );
}

/// TEST-SYNTAX-02cb: ModuleResolver は stdlib fallback が現れたら cache を invalidation すること
#[test]
fn test_e2e_selfhost_module_resolver_cache_invalidates_when_stdlib_fallback_appears() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_selfhost_module_resolver_cache_stdlib_invalidation_{}",
        std::process::id()
    ));
    let source_dir = dir.join("pkg/src");
    let stdlib_dir = dir.join("pkg/stdlib");
    std::fs::create_dir_all(&source_dir).unwrap();
    std::fs::create_dir_all(&stdlib_dir).unwrap();

    let harness = r#"
(defn main []
  (let [cache-ref (ref-new (map-new))
        resolve-count-ref (ref-new 0)
        path1 (resolve-module-path-with-cache-counted "Geometry" "pkg/src" "pkg" cache-ref resolve-count-ref)
        count1 (ref-get resolve-count-ref)
        _ (write-file "pkg/stdlib/Geometry.ls" "(module Geometry) (defn area [] 2)")
        path2 (resolve-module-path-with-cache-counted "Geometry" "pkg/src" "pkg" cache-ref resolve-count-ref)
        count2 (ref-get resolve-count-ref)
        path3 (resolve-module-path-with-cache-counted "Geometry" "pkg/src" "pkg" cache-ref resolve-count-ref)
        count3 (ref-get resolve-count-ref)]
    (do
      (print (if (text-eq path1 "pkg/src/Geometry.ls") 1 0))
      (print count1)
      (print (if (text-eq path2 "pkg/stdlib/Geometry.ls") 1 0))
      (print count2)
      (print (if (text-eq path3 "pkg/stdlib/Geometry.ls") 1 0))
      (print count3)
      0)))
"#;

    let output = compile_and_run_with_dir(&module_resolver_runtime_source(harness), &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "module resolver stdlib invalidation 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "初回は missing local fallback を返すべき");
    assert_eq!(lines[1], "1", "初回 resolve では count が 1 になるべき");
    assert_eq!(lines[2], "1", "stdlib 出現後は stdlib path を返すべき");
    assert_eq!(
        lines[3], "2",
        "stdlib invalidation 後の resolve で count が増えるべき"
    );
    assert_eq!(
        lines[4], "1",
        "更新後の clean hit でも stdlib path を維持するべき"
    );
    assert_eq!(
        lines[5], "2",
        "mutation がなければ resolve-count は増えないべき"
    );
}
