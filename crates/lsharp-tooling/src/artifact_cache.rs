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
#[path = "artifact_cache_tests.rs"]
mod tests;
