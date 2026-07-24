use std::path::{Path, PathBuf};

use crate::compile::{COMPILE_CACHE_KEY_SCHEMA, CompileCacheKey};
use crate::diagnostics::driver_io_error;
use lsharp_ir::SourceFingerprint;

const ARTIFACT_CACHE_SCHEMA: &str = "lsharp-compile-artifact-v1";

/// process 間 compile artifact を key 付き envelope で保存する明示的 cache。
///
/// caller が root を明示した場合だけ使用され、既定の compile path は変更しない。invalid な
/// envelope は cache miss として扱い、fresh compile が stale / corrupt bytes を成功扱いしない。
#[derive(Debug, Clone)]
pub struct ArtifactCache {
    root: PathBuf,
}

impl ArtifactCache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// key に対応する opaque artifact bytes を読み込む。
    ///
    /// ファイルがない、schema/key が一致しない、payload fingerprint が一致しない場合は
    /// `Ok(None)` を返す。permission や filesystem failure は caller が扱えるよう error とする。
    pub fn load(&self, key: &CompileCacheKey) -> miette::Result<Option<Vec<u8>>> {
        let path = self.path_for(key);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(driver_io_error(format!(
                    "compile artifact cache の読み込みに失敗しました ({}): {error}",
                    path.display()
                )));
            }
        };

        let fixed_prefix = fixed_prefix(key);
        if !bytes.starts_with(&fixed_prefix) {
            return Ok(None);
        }
        let payload_with_digest = &bytes[fixed_prefix.len()..];
        let Some(digest_end) = payload_with_digest.iter().position(|byte| *byte == b'\n') else {
            return Ok(None);
        };
        let Ok(expected_digest) = std::str::from_utf8(&payload_with_digest[..digest_end]) else {
            return Ok(None);
        };
        let payload = &payload_with_digest[digest_end + 1..];
        if expected_digest != SourceFingerprint::from_bytes(payload).to_string() {
            return Ok(None);
        }
        Ok(Some(payload.to_vec()))
    }

    /// opaque artifact bytes を key/schema/payload fingerprint 付きで atomic に保存する。
    pub fn store(&self, key: &CompileCacheKey, payload: &[u8]) -> miette::Result<()> {
        let directory = self.root.join(ARTIFACT_CACHE_SCHEMA);
        std::fs::create_dir_all(&directory).map_err(|error| {
            driver_io_error(format!(
                "compile artifact cache directory の作成に失敗しました ({}): {error}",
                directory.display()
            ))
        })?;

        let mut bytes = fixed_prefix(key);
        bytes.extend_from_slice(
            SourceFingerprint::from_bytes(payload)
                .to_string()
                .as_bytes(),
        );
        bytes.push(b'\n');
        bytes.extend_from_slice(payload);
        lsharp_wasm::component_adapter::write_wasm_artifact(&self.path_for(key), &bytes).map_err(
            |error| {
                driver_io_error(format!(
                    "compile artifact cache の保存に失敗しました: {error}"
                ))
            },
        )
    }

    /// `max_entries` を超えた artifact を deterministic に削除する。
    ///
    /// fingerprint は opaque なため、mtime/LRU を意味論として仮定せず、`.artifact` の
    /// file name が辞書順で小さいものから削除する。既定 compile path からは自動実行せず、
    /// 明示 root を管理する caller が頻度と上限を決める。cache directory がまだなければ
    /// no-op とし、schema 外のファイルや `.artifact` 以外の entry は変更しない。
    pub fn trim_to_entries(&self, max_entries: usize) -> miette::Result<usize> {
        let artifact_paths = self.artifact_paths()?;
        let remove_count = artifact_paths.len().saturating_sub(max_entries);
        remove_artifact_paths(artifact_paths.into_iter().take(remove_count))
    }

    /// `max_bytes` を超えた artifact を deterministic に削除する。
    ///
    /// file name の辞書順で小さいものから削除し、entry count trim と同じ deterministic policy
    /// を使う。単一 artifact が上限より大きい場合はその artifact 自体を削除する。cache directory
    /// がまだなければ no-op とし、非 `.artifact` entry は変更しない。
    pub fn trim_to_bytes(&self, max_bytes: u64) -> miette::Result<usize> {
        let mut sized_paths = Vec::new();
        let mut total_bytes = 0_u64;
        for path in self.artifact_paths()? {
            let size = match std::fs::metadata(&path) {
                Ok(metadata) => metadata.len(),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => {
                    return Err(driver_io_error(format!(
                        "compile artifact cache entry の metadata 取得に失敗しました ({}): {error}",
                        path.display()
                    )));
                }
            };
            total_bytes = total_bytes.saturating_add(size);
            sized_paths.push((path, size));
        }
        if total_bytes <= max_bytes {
            return Ok(0);
        }

        let mut removed = 0;
        for (path, size) in sized_paths {
            if total_bytes <= max_bytes {
                break;
            }
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    total_bytes = total_bytes.saturating_sub(size);
                    removed += 1;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(driver_io_error(format!(
                        "compile artifact cache entry の削除に失敗しました ({}): {error}",
                        path.display()
                    )));
                }
            }
        }
        Ok(removed)
    }

    fn artifact_paths(&self) -> miette::Result<Vec<PathBuf>> {
        let directory = self.root.join(ARTIFACT_CACHE_SCHEMA);
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(driver_io_error(format!(
                    "compile artifact cache directory の列挙に失敗しました ({}): {error}",
                    directory.display()
                )));
            }
        };
        let mut artifact_paths = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| {
                driver_io_error(format!(
                    "compile artifact cache entry の読み込みに失敗しました ({}): {error}",
                    directory.display()
                ))
            })?;
            let file_type = entry.file_type().map_err(|error| {
                driver_io_error(format!(
                    "compile artifact cache entry の種別取得に失敗しました ({}): {error}",
                    entry.path().display()
                ))
            })?;
            if file_type.is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension == "artifact")
            {
                artifact_paths.push(entry.path());
            }
        }
        artifact_paths.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
        Ok(artifact_paths)
    }

    fn path_for(&self, key: &CompileCacheKey) -> PathBuf {
        self.root
            .join(ARTIFACT_CACHE_SCHEMA)
            .join(format!("{}.artifact", key.fingerprint()))
    }
}

fn remove_artifact_paths(paths: impl Iterator<Item = PathBuf>) -> miette::Result<usize> {
    let mut removed = 0;
    for path in paths {
        match std::fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(driver_io_error(format!(
                    "compile artifact cache entry の削除に失敗しました ({}): {error}",
                    path.display()
                )));
            }
        }
    }
    Ok(removed)
}

fn fixed_prefix(key: &CompileCacheKey) -> Vec<u8> {
    format!(
        "{ARTIFACT_CACHE_SCHEMA}\n{COMPILE_CACHE_KEY_SCHEMA}\n{}\n",
        key.fingerprint()
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
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
}
