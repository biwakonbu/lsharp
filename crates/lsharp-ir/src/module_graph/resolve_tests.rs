use super::*;

#[test]
fn test_module_name_to_paths_simple() {
    let paths = ModuleGraph::module_name_to_paths("Utils");
    assert!(paths.contains(&"Utils.ls".to_string()));
    assert!(paths.contains(&"utils.ls".to_string()));
}

#[test]
fn test_module_name_to_paths_pascal_case() {
    let paths = ModuleGraph::module_name_to_paths("MathUtils");
    assert!(paths.contains(&"MathUtils.ls".to_string()));
    assert!(paths.contains(&"math_utils.ls".to_string()));
}

#[test]
fn test_module_name_to_paths_nested() {
    let paths = ModuleGraph::module_name_to_paths("App.Utils");
    assert!(paths.contains(&"App/Utils.ls".to_string()));
    assert!(paths.contains(&"app/utils.ls".to_string()));
}

#[test]
fn test_to_snake_case() {
    assert_eq!(ModuleGraph::to_snake_case("Utils"), "utils");
    assert_eq!(ModuleGraph::to_snake_case("MathUtils"), "math_utils");
    assert_eq!(ModuleGraph::to_snake_case("A"), "a");
    assert_eq!(ModuleGraph::to_snake_case("abc"), "abc");
}

#[test]
fn test_to_pascal_case() {
    assert_eq!(ModuleGraph::to_pascal_case("utils"), "Utils");
    assert_eq!(ModuleGraph::to_pascal_case("math_utils"), "MathUtils");
    assert_eq!(ModuleGraph::to_pascal_case("main"), "Main");
}

#[test]
fn test_resolve_module_file_not_found() {
    let result = ModuleGraph::resolve_module_file(
        "NonExistent",
        std::path::Path::new("/tmp/lsharp_nonexistent"),
    );
    assert!(result.is_none());
}

#[test]
fn test_resolve_module_file_found() {
    // 一時ディレクトリにファイルを作成してテスト
    let dir = std::env::temp_dir().join("lsharp_resolve_test");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("Utils.ls"), "(module Utils)\n(defn helper [x] x)").unwrap();

    let result = ModuleGraph::resolve_module_file("Utils", &dir);
    assert!(result.is_some());
    assert!(result.unwrap().ends_with("Utils.ls"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_resolve_module_file_snake_case() {
    // snake_case ファイル名でも探索可能
    let dir = std::env::temp_dir().join("lsharp_resolve_snake");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("math_utils.ls"),
        "(module MathUtils)\n(defn add [x y] (+ x y))",
    )
    .unwrap();

    let result = ModuleGraph::resolve_module_file("MathUtils", &dir);
    assert!(result.is_some());
    assert!(result.unwrap().ends_with("math_utils.ls"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_build_from_entry_single_file() {
    let dir = std::env::temp_dir().join("lsharp_build_single");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("main.ls"),
        "(module Main)\n(defn main [] (print 42))",
    )
    .unwrap();

    let (graph, files) = ModuleGraph::build_from_entry(&dir.join("main.ls")).unwrap();
    assert_eq!(graph.len(), 1);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "Main");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_build_from_entry_with_import() {
    let dir = std::env::temp_dir().join("lsharp_build_import");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("Utils.ls"),
        "(module Utils)\n(defn helper [x] (+ x 1))",
    )
    .unwrap();
    std::fs::write(
        dir.join("main.ls"),
        "(module Main)\n(import Utils)\n(defn main [] (print (helper 41)))",
    )
    .unwrap();

    let (graph, files) = ModuleGraph::build_from_entry(&dir.join("main.ls")).unwrap();
    assert_eq!(graph.len(), 2);
    assert_eq!(files.len(), 2);
    // トポロジカルソート順: Utils が Main より前
    let pos_utils = files.iter().position(|(n, _)| n == "Utils").unwrap();
    let pos_main = files.iter().position(|(n, _)| n == "Main").unwrap();
    assert!(pos_utils < pos_main);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_build_from_entry_missing_import() {
    let dir = std::env::temp_dir().join("lsharp_build_missing");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("main.ls"),
        "(module Main)\n(import NonExistent)\n(defn main [] (print 1))",
    )
    .unwrap();

    let result = ModuleGraph::build_from_entry(&dir.join("main.ls"));
    assert!(result.is_err());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_build_from_entry_prefers_package_src_root() {
    let dir = std::env::temp_dir().join("lsharp_build_package_src_root");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join("examples/demo")).unwrap();
    std::fs::write(dir.join("lsharp.toml"), "[project]\nname=\"demo\"\n").unwrap();
    std::fs::write(
        dir.join("src/Utils.ls"),
        "(module Utils)\n(defn helper [x] (+ x 1))",
    )
    .unwrap();
    std::fs::write(
        dir.join("examples/demo/Main.ls"),
        "(module Main)\n(import Utils)\n(defn main [] (helper 1))",
    )
    .unwrap();

    let (graph, files) = ModuleGraph::build_from_entry(&dir.join("examples/demo/Main.ls")).unwrap();
    assert_eq!(graph.len(), 2);
    assert!(
        files
            .iter()
            .any(|(name, path)| name == "Utils" && path.ends_with("src/Utils.ls"))
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_build_from_entry_prefers_nearest_src_ancestor_without_manifest() {
    let dir = std::env::temp_dir().join("lsharp_build_nearest_src_ancestor");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("selfhost/src/App")).unwrap();
    std::fs::create_dir_all(dir.join("selfhost/src/Syntax")).unwrap();
    std::fs::write(
        dir.join("selfhost/src/Syntax/Token.ls"),
        "(module Syntax.Token)\n(defn token-tag [] 1)",
    )
    .unwrap();
    std::fs::write(
        dir.join("selfhost/src/App/Main.ls"),
        "(module App.Main)\n(import Syntax.Token)\n(defn main [] (token-tag))",
    )
    .unwrap();

    let (graph, files) =
        ModuleGraph::build_from_entry(&dir.join("selfhost/src/App/Main.ls")).unwrap();
    assert_eq!(graph.len(), 2);
    assert!(files.iter().any(|(name, path)| {
        name == "Syntax.Token" && path.ends_with("selfhost/src/Syntax/Token.ls")
    }));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_resolve_module_file_with_search_paths_uses_packages_then_stdlib() {
    let dir = std::env::temp_dir().join("lsharp_resolve_search_paths");
    let _ = std::fs::remove_dir_all(&dir);
    let pkg_src = dir.join(".lsharp/packages/demo-123/src");
    let stdlib = dir.join("custom-stdlib");
    std::fs::create_dir_all(&pkg_src).unwrap();
    std::fs::create_dir_all(&stdlib).unwrap();
    std::fs::write(
        pkg_src.join("Helpers.ls"),
        "(module Helpers)\n(defn helper [] 1)",
    )
    .unwrap();
    std::fs::write(
        stdlib.join("List.ls"),
        "(module List)\n(defn length [xs] 0)",
    )
    .unwrap();

    let search_paths = ModuleSearchPaths {
        package_root: dir.clone(),
        source_root: dir.join("src"),
        package_sources: vec![pkg_src],
        stdlib_root: Some(stdlib.clone()),
    };

    let pkg_result = ModuleGraph::resolve_module_file_with_search_paths("Helpers", &search_paths);
    let stdlib_result = ModuleGraph::resolve_module_file_with_search_paths("List", &search_paths);

    assert!(
        pkg_result
            .as_ref()
            .is_some_and(|path| path.ends_with("Helpers.ls"))
    );
    assert!(
        stdlib_result
            .as_ref()
            .is_some_and(|path| path.ends_with("List.ls"))
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_build_from_entry_resolves_packages_from_project_root() {
    let dir = std::env::temp_dir().join("lsharp_build_project_root_packages");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("examples/demo")).unwrap();
    std::fs::create_dir_all(dir.join(".lsharp/packages/pkg-123/src")).unwrap();
    std::fs::write(dir.join("lsharp.toml"), "[project]\nname=\"demo\"\n").unwrap();
    std::fs::write(
        dir.join(".lsharp/packages/pkg-123/src/Helpers.ls"),
        "(module Helpers)\n(defn helper [] 1)",
    )
    .unwrap();
    std::fs::write(
        dir.join("examples/demo/Main.ls"),
        "(module Main)\n(import Helpers)\n(defn main [] (helper))",
    )
    .unwrap();

    let (graph, files) = ModuleGraph::build_from_entry(&dir.join("examples/demo/Main.ls")).unwrap();

    assert_eq!(graph.len(), 2);
    assert!(files.iter().any(|(name, path)| {
        name == "Helpers" && path.ends_with(".lsharp/packages/pkg-123/src/Helpers.ls")
    }));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_build_from_entry_rejects_non_exported_package_module() {
    let dir = std::env::temp_dir().join("lsharp_build_non_exported_package_module");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::create_dir_all(dir.join(".lsharp/packages/demo-123/src")).unwrap();
    std::fs::write(dir.join("lsharp.toml"), "[project]\nname=\"app\"\n").unwrap();
    std::fs::write(
        dir.join(".lsharp/packages/demo-123/lsharp.toml"),
        "[project]\nname=\"demo\"\n[project.exports]\nmodules=[\"Public\"]\n",
    )
    .unwrap();
    std::fs::write(
        dir.join(".lsharp/packages/demo-123/src/Hidden.ls"),
        "(module Hidden)\n(defn helper [] 1)",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/Main.ls"),
        "(module Main)\n(import Hidden)\n(defn main [] 0)",
    )
    .unwrap();

    let result = ModuleGraph::build_from_entry(&dir.join("src/Main.ls"));
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        result.is_err(),
        "非公開 package module の import は失敗するべき"
    );
}
