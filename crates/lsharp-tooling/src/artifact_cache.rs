use std::path::{Path, PathBuf};

use crate::compile::{COMPILE_CACHE_KEY_SCHEMA, CompileCacheKey};
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
                return Err(miette::miette!(
                    "compile artifact cache の読み込みに失敗しました ({}): {error}",
                    path.display()
                ));
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
            miette::miette!(
                "compile artifact cache directory の作成に失敗しました ({}): {error}",
                directory.display()
            )
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
            |error| miette::miette!("compile artifact cache の保存に失敗しました: {error}"),
        )
    }

    fn path_for(&self, key: &CompileCacheKey) -> PathBuf {
        self.root
            .join(ARTIFACT_CACHE_SCHEMA)
            .join(format!("{}.artifact", key.fingerprint()))
    }
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
