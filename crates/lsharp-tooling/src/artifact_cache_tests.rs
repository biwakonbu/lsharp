use super::*;
use crate::compile::{CompileBackend, CompileTarget};

#[test]
fn test_artifact_cache_stores_and_loads_bytes_for_matching_key() {
    let dir = unique_temp_dir("roundtrip");
    let cache = ArtifactCache::new(&dir);
    let key = test_key(&dir, CompileTarget::WasiPreview1, CompileBackend::Linear);

    cache
        .store(&key, b"compiled-wasm")
        .expect("artifact cache は bytes を保存できるべき");
    assert_eq!(
        cache
            .load(&key)
            .expect("artifact cache は bytes を読み込めるべき"),
        Some(b"compiled-wasm".to_vec())
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_artifact_cache_misses_when_key_changes() {
    let dir = unique_temp_dir("key-miss");
    let cache = ArtifactCache::new(&dir);
    let first = test_key(&dir, CompileTarget::WasiPreview1, CompileBackend::Linear);
    let second = test_key(&dir, CompileTarget::WasiComponent, CompileBackend::Linear);

    cache.store(&first, b"compiled-wasm").unwrap();
    assert_eq!(
        cache
            .load(&second)
            .expect("別 key の cache lookup は失敗扱いにしない"),
        None,
        "target が変わった artifact は再利用してはいけない"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_artifact_cache_rejects_corrupt_envelope_and_leaves_no_temp_file() {
    let dir = unique_temp_dir("corrupt");
    let cache = ArtifactCache::new(&dir);
    let key = test_key(&dir, CompileTarget::WasiPreview1, CompileBackend::Linear);

    cache.store(&key, b"compiled-wasm").unwrap();
    let path = cache.path_for(&key);
    let mut corrupt = std::fs::read(&path).unwrap();
    *corrupt
        .last_mut()
        .expect("artifact envelope は payload を含む") ^= 1;
    std::fs::write(&path, corrupt).unwrap();

    assert_eq!(
        cache
            .load(&key)
            .expect("破損 cache は fresh compile へ戻れるべき"),
        None,
        "破損 envelope を artifact bytes として返してはいけない"
    );
    let entries = std::fs::read_dir(dir.join("lsharp-compile-artifact-v1"))
        .expect("cache directory を列挙できる")
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        entries.len(),
        1,
        "破損 cache の lookup で一時 file を残さない"
    );
    assert!(
        entries[0]
            .file_name()
            .to_string_lossy()
            .ends_with(".artifact"),
        "cache entry は deterministic artifact path を使うべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_artifact_cache_trim_to_entries_removes_deterministic_lowest_keys() {
    let dir = unique_temp_dir("trim");
    let cache = ArtifactCache::new(&dir);
    let first = test_key(&dir, CompileTarget::WasiPreview1, CompileBackend::Linear);
    std::fs::write(dir.join("Second.ls"), "(module Second)\n(defn main [] 8)\n").unwrap();
    let second = CompileCacheKey::from_entry(
        &dir.join("Second.ls"),
        CompileTarget::WasiPreview1,
        CompileBackend::Linear,
    )
    .unwrap();
    std::fs::write(dir.join("Third.ls"), "(module Third)\n(defn main [] 9)\n").unwrap();
    let third = CompileCacheKey::from_entry(
        &dir.join("Third.ls"),
        CompileTarget::WasiPreview1,
        CompileBackend::Linear,
    )
    .unwrap();
    cache.store(&first, b"first").unwrap();
    cache.store(&second, b"second").unwrap();
    cache.store(&third, b"third").unwrap();
    std::fs::write(
        dir.join("lsharp-compile-artifact-v1").join("notes.txt"),
        "caller-owned metadata",
    )
    .unwrap();

    let mut fingerprints = [
        first.fingerprint().to_string(),
        second.fingerprint().to_string(),
        third.fingerprint().to_string(),
    ];
    fingerprints.sort();
    let removed = cache
        .trim_to_entries(1)
        .expect("bounded cache maintenance は成功するべき");
    assert_eq!(removed, 2);
    assert_eq!(
        cache.load(&first).unwrap(),
        if first.fingerprint().to_string() == fingerprints[2] {
            Some(b"first".to_vec())
        } else {
            None
        }
    );
    assert_eq!(
        cache.load(&second).unwrap(),
        if second.fingerprint().to_string() == fingerprints[2] {
            Some(b"second".to_vec())
        } else {
            None
        }
    );
    assert_eq!(
        cache.load(&third).unwrap(),
        if third.fingerprint().to_string() == fingerprints[2] {
            Some(b"third".to_vec())
        } else {
            None
        }
    );
    assert!(
        dir.join("lsharp-compile-artifact-v1/notes.txt").exists(),
        "schema directory内の非artifact file は caller-owned として残すべき"
    );

    let missing = ArtifactCache::new(dir.join("missing"));
    assert_eq!(missing.trim_to_entries(0).unwrap(), 0);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_artifact_cache_trim_to_bytes_removes_deterministic_lowest_keys() {
    let dir = unique_temp_dir("byte-trim");
    let cache = ArtifactCache::new(&dir);
    let first = test_key(&dir, CompileTarget::WasiPreview1, CompileBackend::Linear);
    std::fs::write(dir.join("Second.ls"), "(module Second)\n(defn main [] 8)\n").unwrap();
    let second = CompileCacheKey::from_entry(
        &dir.join("Second.ls"),
        CompileTarget::WasiPreview1,
        CompileBackend::Linear,
    )
    .unwrap();
    std::fs::write(dir.join("Third.ls"), "(module Third)\n(defn main [] 9)\n").unwrap();
    let third = CompileCacheKey::from_entry(
        &dir.join("Third.ls"),
        CompileTarget::WasiPreview1,
        CompileBackend::Linear,
    )
    .unwrap();
    for key in [&first, &second, &third] {
        cache.store(key, b"same-payload").unwrap();
    }
    std::fs::write(
        dir.join("lsharp-compile-artifact-v1").join("notes.txt"),
        "caller-owned metadata",
    )
    .unwrap();
    let one_entry_bytes = std::fs::metadata(cache.path_for(&first)).unwrap().len();

    assert_eq!(
        cache.trim_to_bytes(one_entry_bytes).unwrap(),
        2,
        "byte budget は deterministic entry を上限まで削除するべき"
    );
    assert_eq!(
        std::fs::read_dir(dir.join("lsharp-compile-artifact-v1"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "artifact"))
            .count(),
        1
    );
    assert!(dir.join("lsharp-compile-artifact-v1/notes.txt").exists());
    assert_eq!(cache.trim_to_bytes(0).unwrap(), 1);

    let missing = ArtifactCache::new(dir.join("missing"));
    assert_eq!(missing.trim_to_bytes(0).unwrap(), 0);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_artifact_cache_store_failure_preserves_driver_io_error_code() {
    let dir = unique_temp_dir("store-failure");
    let key = test_key(&dir, CompileTarget::WasiPreview1, CompileBackend::Linear);
    let cache_root = dir.join("cache-file");
    std::fs::write(&cache_root, "not a directory").unwrap();

    let error = ArtifactCache::new(&cache_root)
        .store(&key, b"compiled-wasm")
        .expect_err("file を cache root に使うと store は失敗するべき");
    assert!(
        error.to_string().starts_with("[LS5001]"),
        "artifact cache file I/O diagnostics は stable code を含むべき: {error}"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

fn test_key(
    dir: &std::path::Path,
    target: CompileTarget,
    backend: CompileBackend,
) -> CompileCacheKey {
    let source = dir.join("Main.ls");
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(&source, "(module Main)\n(defn main [] 7)\n").unwrap();
    CompileCacheKey::from_entry(&source, target, backend).unwrap()
}

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "lsharp_tooling_artifact_cache_{name}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ))
}
