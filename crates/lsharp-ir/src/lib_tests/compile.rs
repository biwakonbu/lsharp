use super::*;

#[test]
fn test_compile_module_seam_preserves_full_and_cached_entrypoints() {
    let dir =
        std::env::temp_dir().join(format!("lsharp_compile_module_seam_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("compile seam fixture directory should be created");
    let entry = dir.join("Main.ls");
    std::fs::write(&entry, "(module Main) (defn main [] 42)\n")
        .expect("compile seam fixture should be written");

    let full = compile_multi_file(&entry).expect("full compile entrypoint should succeed");
    let mut cache = CompilationCache::new();
    let cached = compile_multi_file_with_cache(&entry, &mut cache)
        .expect("cached compile entrypoint should succeed");

    assert_eq!(full.dump(), cached.dump());
    assert_eq!(cache.len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}
