//! `crates/lsharp-driver/build.rs` が埋め込む component を content-addressed で再利用する cache。
//!
//! build.rs は `rerun-if-changed=selfhost/src` を張っているため、branch 切替や `git stash`、
//! mtime だけの変化でも 79,440 行の selfhost tree を丸ごと再コンパイルする。
//! ここでは source fingerprint と emitter identity から key を作り、一致したら生成済み bytes を
//! そのまま返す。実測 (Mac M-series, dev profile): miss 1m46s → hit 4.2s。
//!
//! ## `lsharp-tooling::ArtifactCache` を使わない理由
//!
//! envelope の形は `ArtifactCache` とほぼ同じだが、`lsharp-tooling` は `lsharp-lsp` 経由で
//! tower-lsp / tokio に依存する。build script の依存に足すと build script 自身のコンパイルが
//! 重くなり、この cache の目的と正面から衝突する。`lsharp-wasm` は既に driver の
//! `[build-dependencies]` にあり、`component_adapter::write_wasm_artifact` (atomic writer) も
//! ここが持っているので追加コストがゼロで済む。
//!
//! ## key に emitter を含める理由
//!
//! source だけを key にすると、`emit_wasm_wasi_p2` を書き換えても key が変わらず、古い emitter が
//! 出した bytes を新しい emitter の成果物として黙って埋め込んでしまう。workspace の
//! `CARGO_PKG_VERSION` は全 crate 共通で固定されているため識別子として使えない。build script の
//! 実行ファイル自身の fingerprint を使う: `lsharp-ir` / `lsharp-wasm` が変われば build script も
//! 再リンクされ、fingerprint が変わる。

use std::path::{Path, PathBuf};

use lsharp_ir::SourceFingerprint;
use thiserror::Error;

/// cache envelope の schema。`lsharp-compile-artifact-v1` とは互換性を持たせない。
pub const EMBEDDED_COMPONENT_CACHE_SCHEMA: &str = "lsharp-embed-component-v1";

#[derive(Debug, Error)]
pub enum EmbeddedComponentCacheError {
    #[error("embedded component cache の I/O に失敗しました ({path}): {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("embedded component cache の保存に失敗しました: {message}")]
    Write { message: String },
}

pub type Result<T> = std::result::Result<T, EmbeddedComponentCacheError>;

fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> EmbeddedComponentCacheError {
    EmbeddedComponentCacheError::Io {
        path: path.into(),
        source,
    }
}

/// source 群と emitter identity から導出した cache key。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EmbeddedComponentKey(SourceFingerprint);

impl EmbeddedComponentKey {
    /// key 導出の純関数。`sources` は `(label 相対 path, 内容 fingerprint)` の列。
    ///
    /// caller が渡す順序に依存しないよう、ここで必ず name 順に整列してから hash する。
    pub fn from_parts(
        sources: &[(String, SourceFingerprint)],
        emitter: &SourceFingerprint,
    ) -> Self {
        let mut sorted: Vec<&(String, SourceFingerprint)> = sources.iter().collect();
        sorted.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));

        let mut manifest = Vec::new();
        manifest.extend_from_slice(EMBEDDED_COMPONENT_CACHE_SCHEMA.as_bytes());
        manifest.push(b'\n');
        manifest.extend_from_slice(b"emitter\0");
        manifest.extend_from_slice(emitter.to_string().as_bytes());
        manifest.push(b'\n');
        for (name, fingerprint) in sorted {
            manifest.extend_from_slice(name.as_bytes());
            manifest.push(0);
            manifest.extend_from_slice(fingerprint.to_string().as_bytes());
            manifest.push(b'\n');
        }
        Self(SourceFingerprint::from_bytes(&manifest))
    }

    pub fn fingerprint(&self) -> &SourceFingerprint {
        &self.0
    }
}

impl std::fmt::Display for EmbeddedComponentKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// cache key の入力になる root。project root からの相対 path で並べる。
///
/// この配列は 2 つの用途の**単一の正本**である。
///
/// 1. `embedded_component_key_sources` が走査する root
/// 2. `crates/lsharp-driver/build.rs` が出す `cargo:rerun-if-changed`
///
/// 両者を別々に書くと、片方にだけ root が足された状態が生まれる。実際 `wit/` は
/// 2 にだけ載っていて 1 に無く、`stdlib/` はどちらにも無かった (`I-16`)。
/// 前者は「build script は再実行されるのに古い bytes を hit する」、後者は
/// 「build script がそもそも再実行されない」という別々の壊れ方をする。
///
/// 各 root は丸ごと走査し、拡張子で絞らない。絞り込みルールは module resolver の実装と
/// 二重管理になり、drift したときに**静かに under-invalidate する**方向へ壊れる。
/// over-invalidate は遅いだけなので、非対称な損失に対して保守側へ倒している。
pub const EMBEDDED_COMPONENT_KEY_ROOTS: [&str; 3] = ["selfhost/src", "stdlib", "wit"];

/// `EMBEDDED_COMPONENT_KEY_ROOTS` をすべて走査し、cache key の入力列を作る。
///
/// label には root の相対 path をそのまま使うので、entry 名は `stdlib/List.ls` のように
/// project root 相対になる。存在しない root は空として扱う (checkout の形に依存させない)。
pub fn embedded_component_key_sources(
    project_root: &Path,
) -> Result<Vec<(String, SourceFingerprint)>> {
    let mut sources = Vec::new();
    for root in EMBEDDED_COMPONENT_KEY_ROOTS {
        // root は `/` 区切りの相対 path なので、component ごとに join して platform 差を吸収する。
        let mut absolute = project_root.to_path_buf();
        for segment in root.split('/') {
            absolute.push(segment);
        }
        sources.extend(collect_source_entries(root, &absolute)?);
    }
    // `from_parts` 側でも整列するが、この関数単体の返り値も安定させておく。
    sources.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    Ok(sources)
}

/// `root` 配下の全 regular file を `(label/相対 path, 内容 fingerprint)` として集める。
///
/// 絶対 path は key に含めない。worktree ごとに path が違っても同じ source なら同じ key に
/// なるようにするためである。相対 path の区切りは常に `/` に正規化する。
pub fn collect_source_entries(
    label: &str,
    root: &Path,
) -> Result<Vec<(String, SourceFingerprint)>> {
    let mut entries = Vec::new();
    collect_into(label, root, root, &mut entries)?;
    entries.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
    Ok(entries)
}

fn collect_into(
    label: &str,
    root: &Path,
    directory: &Path,
    entries: &mut Vec<(String, SourceFingerprint)>,
) -> Result<()> {
    let read_dir = match std::fs::read_dir(directory) {
        Ok(read_dir) => read_dir,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(io_error(directory, error)),
    };
    // read_dir の順序は filesystem 依存なので、ここでは並べずに呼び出し元で整列する。
    for entry in read_dir {
        let entry = entry.map_err(|error| io_error(directory, error))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| io_error(path.clone(), error))?;
        if file_type.is_dir() {
            collect_into(label, root, &path, entries)?;
            continue;
        }
        if !file_type.is_file() {
            // symlink は追わない。build 対象の実体だけを key にする。
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|error| io_error(path.clone(), error))?;
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .components()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");
        entries.push((
            format!("{label}/{relative}"),
            SourceFingerprint::from_bytes(&bytes),
        ));
    }
    Ok(())
}

/// 現在の実行ファイル (build script として動いていれば build script binary) の fingerprint。
///
/// `lsharp-ir` / `lsharp-wasm` を書き換えると build script が再リンクされ、この値が変わる。
pub fn current_executable_fingerprint() -> Result<SourceFingerprint> {
    let path =
        std::env::current_exe().map_err(|error| io_error(PathBuf::from("<current_exe>"), error))?;
    let bytes = std::fs::read(&path).map_err(|error| io_error(path.clone(), error))?;
    Ok(SourceFingerprint::from_bytes(&bytes))
}

/// key 付き envelope で component bytes を保存する content-addressed store。
///
/// envelope が壊れている / key が違う場合は必ず `Ok(None)` を返す。stale bytes を成功として
/// 返さないことがこの型の唯一の安全性要件である。
#[derive(Debug, Clone)]
pub struct EmbeddedComponentCache {
    root: PathBuf,
}

impl EmbeddedComponentCache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path_for(&self, key: &EmbeddedComponentKey) -> PathBuf {
        self.root
            .join(EMBEDDED_COMPONENT_CACHE_SCHEMA)
            .join(format!("{key}.component.wasm"))
    }

    pub fn load(&self, key: &EmbeddedComponentKey) -> Result<Option<Vec<u8>>> {
        let path = self.path_for(key);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(io_error(path, error)),
        };

        let prefix = fixed_prefix(key);
        if !bytes.starts_with(&prefix) {
            return Ok(None);
        }
        let payload_with_digest = &bytes[prefix.len()..];
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

    pub fn store(&self, key: &EmbeddedComponentKey, payload: &[u8]) -> Result<()> {
        let directory = self.root.join(EMBEDDED_COMPONENT_CACHE_SCHEMA);
        std::fs::create_dir_all(&directory).map_err(|error| io_error(directory, error))?;

        let mut bytes = fixed_prefix(key);
        bytes.extend_from_slice(
            SourceFingerprint::from_bytes(payload)
                .to_string()
                .as_bytes(),
        );
        bytes.push(b'\n');
        bytes.extend_from_slice(payload);
        crate::component_adapter::write_wasm_artifact(&self.path_for(key), &bytes).map_err(
            |error| EmbeddedComponentCacheError::Write {
                message: format!("{error}"),
            },
        )
    }

    /// entry 数を `limit` 以下に刈り込む。新しいものから残す。
    ///
    /// 1 entry が 1MB 超あるので、source を編集するたび `target` 配下が膨らむのを防ぐ。
    /// 直前に store した entry を捨てるとその build が即 miss になるため、mtime の新しい順に残す。
    /// mtime が同値のときは名前の降順で決める (順序を filesystem 依存にしない)。
    pub fn trim_to_entries(&self, limit: usize) -> Result<()> {
        let directory = self.root.join(EMBEDDED_COMPONENT_CACHE_SCHEMA);
        let read_dir = match std::fs::read_dir(&directory) {
            Ok(read_dir) => read_dir,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(io_error(directory, error)),
        };

        let mut entries = Vec::new();
        for entry in read_dir {
            let entry = entry.map_err(|error| io_error(directory.clone(), error))?;
            let path = entry.path();
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            entries.push((modified, entry.file_name(), path));
        }
        if entries.len() <= limit {
            return Ok(());
        }

        entries.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| right.1.as_encoded_bytes().cmp(left.1.as_encoded_bytes()))
        });
        for (_, _, path) in entries.into_iter().skip(limit) {
            std::fs::remove_file(&path).map_err(|error| io_error(path, error))?;
        }
        Ok(())
    }
}

/// cache directory の名前。target dir 直下に置く。
pub const EMBEDDED_COMPONENT_CACHE_DIR: &str = "lsharp-embed-cache";

/// `OUT_DIR` から cache の置き場を逆算する。
///
/// `OUT_DIR` は `<target>/<profile>/build/<crate>-<hash>/out` の形なので、`build` という名前の
/// 祖先を見つけてその 2 つ上を target dir とみなす。`CARGO_TARGET_DIR` が別名でも成立する。
/// 見つからない場合は `None` — 推測で変な場所に書かない。
pub fn cache_root_from_out_dir(out_dir: &Path) -> Option<PathBuf> {
    let build_dir = out_dir
        .ancestors()
        .find(|ancestor| ancestor.file_name().is_some_and(|name| name == "build"))?;
    let target_dir = build_dir.parent()?.parent()?;
    Some(target_dir.join(EMBEDDED_COMPONENT_CACHE_DIR))
}

fn fixed_prefix(key: &EmbeddedComponentKey) -> Vec<u8> {
    format!("{EMBEDDED_COMPONENT_CACHE_SCHEMA}\n{key}\n").into_bytes()
}

#[cfg(test)]
#[path = "embedded_component_cache_tests.rs"]
mod tests;
