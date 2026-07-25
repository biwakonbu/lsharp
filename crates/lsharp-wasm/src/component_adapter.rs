use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use wit_component::{ComponentEncoder, StringEncoding, embed_component_metadata};
use wit_parser::{Resolve, WorldId};

/// Component adapter 変換エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum ComponentAdapterError {
    #[error("component adapter エラー: {msg}")]
    Error { msg: String },
}

type Result<T> = std::result::Result<T, ComponentAdapterError>;

static STAGED_WIT_COUNTER: AtomicU64 = AtomicU64::new(0);

struct ResolvedWorld {
    resolve: Resolve,
    world: WorldId,
    staged_root: PathBuf,
}

/// `wit-component` へ渡す adapter module。
pub struct NamedAdapter<'a> {
    pub name: &'a str,
    pub bytes: &'a [u8],
}

fn componentize_error(world_name: &str, phase: &str, rendered: String) -> ComponentAdapterError {
    let msg = if rendered.contains("env::print-string") {
        format!(
            "WasmGC component bridge は未実装です: `env.print-string` の GC array reference を WIT import interface へ変換できません (world `{world_name}`): {rendered}"
        )
    } else {
        format!("{phase} (world `{world_name}`): {rendered}")
    };
    ComponentAdapterError::Error { msg }
}

fn resolve_world(wit_dir: &Path, world_name: &str) -> Result<ResolvedWorld> {
    let mut candidates: Vec<PathBuf> = if wit_dir.is_dir() {
        fs::read_dir(wit_dir)
            .map_err(|err| ComponentAdapterError::Error {
                msg: format!(
                    "WIT directory の読み込みに失敗しました ({}): {err:#}",
                    wit_dir.display()
                ),
            })?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                (path.extension().and_then(|ext| ext.to_str()) == Some("wit")).then_some(path)
            })
            .collect()
    } else {
        vec![wit_dir.to_path_buf()]
    };
    candidates.sort();

    let mut failures = Vec::new();
    for candidate in candidates {
        let staged_root = stage_wit_workspace(&candidate)?;
        let mut resolve = Resolve::default();
        let (package_id, _) = match resolve.push_dir(&staged_root) {
            Ok(result) => result,
            Err(err) => {
                failures.push(format!("{}: {err:#}", candidate.display()));
                let _ = fs::remove_dir_all(&staged_root);
                continue;
            }
        };
        match resolve.select_world(&[package_id], Some(world_name)) {
            Ok(world) => {
                return Ok(ResolvedWorld {
                    resolve,
                    world,
                    staged_root,
                });
            }
            Err(err) => {
                failures.push(format!("{}: {err:#}", candidate.display()));
                let _ = fs::remove_dir_all(&staged_root);
            }
        }
    }

    Err(ComponentAdapterError::Error {
        msg: format!(
            "world `{world_name}` の解決に失敗しました ({}): {}",
            wit_dir.display(),
            failures.join(" / ")
        ),
    })
}

fn stage_wit_workspace(source: &Path) -> Result<PathBuf> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| ComponentAdapterError::Error {
            msg: format!("一時 WIT workspace の時刻取得に失敗しました: {err:#}"),
        })?
        .as_nanos();
    let sequence = STAGED_WIT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let staged_root = std::env::temp_dir().join(format!(
        "lsharp_component_wit_{}_{}_{}",
        std::process::id(),
        nonce,
        sequence
    ));
    fs::create_dir_all(&staged_root).map_err(|err| ComponentAdapterError::Error {
        msg: format!(
            "一時 WIT workspace の作成に失敗しました ({}): {err:#}",
            staged_root.display()
        ),
    })?;

    let file_name = source
        .file_name()
        .ok_or_else(|| ComponentAdapterError::Error {
            msg: format!(
                "WIT source file 名の取得に失敗しました: {}",
                source.display()
            ),
        })?;
    fs::copy(source, staged_root.join(file_name)).map_err(|err| ComponentAdapterError::Error {
        msg: format!(
            "WIT source file の staging に失敗しました ({}): {err:#}",
            source.display()
        ),
    })?;

    if let Some(parent) = source.parent() {
        let deps_dir = parent.join("deps");
        if deps_dir.is_dir() {
            copy_dir_all(&deps_dir, &staged_root.join("deps"))?;
        }
    }

    Ok(staged_root)
}

fn copy_dir_all(source: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).map_err(|err| ComponentAdapterError::Error {
        msg: format!(
            "WIT dependency directory の作成に失敗しました ({}): {err:#}",
            dest.display()
        ),
    })?;

    for entry in fs::read_dir(source).map_err(|err| ComponentAdapterError::Error {
        msg: format!(
            "WIT dependency directory の読み込みに失敗しました ({}): {err:#}",
            source.display()
        ),
    })? {
        let entry = entry.map_err(|err| ComponentAdapterError::Error {
            msg: format!(
                "WIT dependency entry の読み込みに失敗しました ({}): {err:#}",
                source.display()
            ),
        })?;
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            copy_dir_all(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|err| ComponentAdapterError::Error {
                msg: format!(
                    "WIT dependency file の staging に失敗しました ({}): {err:#}",
                    path.display()
                ),
            })?;
        }
    }

    Ok(())
}

/// core Wasm module に world metadata を埋め込む。
pub fn embed_component_metadata_for_world(
    wasm_bytes: &mut Vec<u8>,
    wit_dir: &Path,
    world_name: &str,
) -> Result<()> {
    let resolved = resolve_world(wit_dir, world_name)?;
    let result = embed_component_metadata(
        wasm_bytes,
        &resolved.resolve,
        resolved.world,
        StringEncoding::UTF8,
    )
    .map_err(|err| ComponentAdapterError::Error {
        msg: format!(
            "component metadata の埋め込みに失敗しました (world `{world_name}` / {}): {err:#}",
            wit_dir.display()
        ),
    });
    let _ = fs::remove_dir_all(&resolved.staged_root);
    result?;
    Ok(())
}

/// core Wasm module を component へ変換する。
pub fn componentize_core_module(
    core_wasm: &[u8],
    wit_dir: &Path,
    world_name: &str,
    adapters: &[NamedAdapter<'_>],
) -> Result<Vec<u8>> {
    let mut main_module = core_wasm.to_vec();
    embed_component_metadata_for_world(&mut main_module, wit_dir, world_name)?;

    let encoder = ComponentEncoder::default()
        .module(&main_module)
        .map_err(|err| {
            componentize_error(
                world_name,
                "main core module の component 化準備に失敗しました",
                format!("{err:#}"),
            )
        })?;

    let encoder = adapters.iter().try_fold(encoder, |encoder, adapter| {
        encoder.adapter(adapter.name, adapter.bytes).map_err(|err| {
            componentize_error(
                world_name,
                &format!("adapter `{}` の登録に失敗しました", adapter.name),
                format!("{err:#}"),
            )
        })
    })?;

    let mut encoder = encoder;
    encoder.encode().map_err(|err| {
        componentize_error(
            world_name,
            "component の生成に失敗しました",
            format!("{err:#}"),
        )
    })
}

/// artifact file の内容を durable storage へ flush する。
pub fn sync_artifact_file(path: &Path) -> Result<()> {
    let file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|err| ComponentAdapterError::Error {
            msg: format!(
                "artifact file の同期対象を開けません ({}): {err:#}",
                path.display()
            ),
        })?;
    file.sync_all().map_err(|err| ComponentAdapterError::Error {
        msg: format!(
            "artifact file の同期に失敗しました ({}): {err:#}",
            path.display()
        ),
    })
}

/// artifact の rename を含む親 directory metadata を durable storage へ flush する。
#[cfg(unix)]
pub fn sync_artifact_parent(path: &Path) -> Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let directory = fs::File::open(parent).map_err(|err| ComponentAdapterError::Error {
        msg: format!(
            "artifact parent directory の同期対象を開けません ({}): {err:#}",
            parent.display()
        ),
    })?;
    directory
        .sync_all()
        .map_err(|err| ComponentAdapterError::Error {
            msg: format!(
                "artifact parent directory の同期に失敗しました ({}): {err:#}",
                parent.display()
            ),
        })
}

/// directory metadata の同期 API がない target では file sync までを durability 境界とする。
#[cfg(not(unix))]
pub fn sync_artifact_parent(_path: &Path) -> Result<()> {
    Ok(())
}

/// Wasm bytes を一時ファイル経由で atomic に artifact path へ保存する。
pub fn write_wasm_artifact(path: &Path, bytes: &[u8]) -> Result<()> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| ComponentAdapterError::Error {
            msg: format!(
                "Wasm artifact の file name を取得できません: {}",
                path.display()
            ),
        })?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| ComponentAdapterError::Error {
            msg: format!("Wasm artifact の一時 path 用時刻取得に失敗しました: {err:#}"),
        })?
        .as_nanos();
    let sequence = STAGED_WIT_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary_path = parent.join(format!(
        ".{file_name}.tmp-{}-{nonce}-{sequence}",
        std::process::id()
    ));
    let result = (|| {
        let mut file =
            fs::File::create(&temporary_path).map_err(|err| ComponentAdapterError::Error {
                msg: format!(
                    "Wasm artifact の一時保存に失敗しました ({}): {err:#}",
                    temporary_path.display()
                ),
            })?;
        file.write_all(bytes)
            .map_err(|err| ComponentAdapterError::Error {
                msg: format!(
                    "Wasm artifact の一時保存に失敗しました ({}): {err:#}",
                    temporary_path.display()
                ),
            })?;
        file.sync_all()
            .map_err(|err| ComponentAdapterError::Error {
                msg: format!(
                    "Wasm artifact の一時保存同期に失敗しました ({}): {err:#}",
                    temporary_path.display()
                ),
            })?;
        drop(file);
        fs::rename(&temporary_path, path).map_err(|err| ComponentAdapterError::Error {
            msg: format!(
                "Wasm artifact の置換に失敗しました ({}): {err:#}",
                path.display()
            ),
        })?;
        sync_artifact_parent(path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

/// 保存済み Wasm artifact を bytes として読み込む。
pub fn read_wasm_artifact(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|err| ComponentAdapterError::Error {
        msg: format!(
            "Wasm artifact の再読込に失敗しました ({}): {err:#}",
            path.display()
        ),
    })
}

/// Component bytes を一時ファイル経由で atomic に artifact path へ保存する。
pub fn write_component_artifact(path: &Path, bytes: &[u8]) -> Result<()> {
    write_wasm_artifact(path, bytes)
}

/// 保存済み Component artifact を bytes として読み込む。
pub fn read_component_artifact(path: &Path) -> Result<Vec<u8>> {
    read_wasm_artifact(path)
}

#[cfg(test)]
#[path = "component_adapter_tests.rs"]
mod tests;
