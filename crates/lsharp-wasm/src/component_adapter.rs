use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use wit_component::{embed_component_metadata, ComponentEncoder, StringEncoding};
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
        fs::write(&temporary_path, bytes).map_err(|err| ComponentAdapterError::Error {
            msg: format!(
                "Wasm artifact の一時保存に失敗しました ({}): {err:#}",
                temporary_path.display()
            ),
        })?;
        fs::rename(&temporary_path, path).map_err(|err| ComponentAdapterError::Error {
            msg: format!(
                "Wasm artifact の置換に失敗しました ({}): {err:#}",
                path.display()
            ),
        })
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
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use wasmtime::component::{Component, Linker};
    use wasmtime::{Engine, Store};

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "lsharp_component_adapter_{name}_{}_{}",
            std::process::id(),
            nonce
        ));
        fs::create_dir_all(&dir).expect("temp dir creation failed");
        dir
    }

    #[test]
    fn test_component_artifact_round_trip_replaces_without_temp_residue() {
        let dir = unique_temp_dir("artifact_round_trip");
        let path = dir.join("Main.component.wasm");
        write_component_artifact(&path, b"first").expect("first Component artifact を保存できる");
        write_component_artifact(&path, b"second").expect("既存 Component artifact を置換できる");

        assert_eq!(
            read_component_artifact(&path).expect("Component artifact を再読込できる"),
            b"second"
        );
        let entries = fs::read_dir(&dir)
            .expect("artifact directory を列挙できる")
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("artifact directory entry を取得できる");
        assert_eq!(entries.len(), 1, "atomic 保存後に一時 artifact を残さない");
        assert_eq!(entries[0].file_name(), "Main.component.wasm");

        fs::remove_dir_all(&dir).expect("artifact directory を削除できる");
    }

    #[test]
    fn test_wasm_artifact_round_trip_supports_non_component_output() {
        let dir = unique_temp_dir("wasm_artifact_round_trip");
        let path = dir.join("Main.wasm");
        write_wasm_artifact(&path, b"core-wasm")
            .expect("non-component Wasm artifact を atomic に保存できる");

        assert_eq!(
            read_wasm_artifact(&path).expect("non-component Wasm artifact を再読込できる"),
            b"core-wasm"
        );
        fs::remove_dir_all(&dir).expect("non-component artifact directory を削除できる");
    }

    #[test]
    fn test_embed_component_metadata_for_world_reports_missing_world() {
        let wit_dir = unique_temp_dir("missing_world");
        fs::write(
            wit_dir.join("worlds.wit"),
            r#"
package test:adapter;

world present {
  export run: func();
}
"#,
        )
        .unwrap();

        let mut module = wat::parse_str("(module (func (export \"run\")))").unwrap();
        let err = embed_component_metadata_for_world(&mut module, &wit_dir, "missing")
            .expect_err("missing world should be rejected");
        assert!(
            err.to_string().contains("world `missing` の解決に失敗"),
            "error should mention world resolution failure: {err}"
        );

        let _ = fs::remove_dir_all(&wit_dir);
    }

    #[test]
    fn test_componentize_core_module_with_preview1_adapter() {
        let wit_dir = unique_temp_dir("wit");
        fs::write(
            wit_dir.join("adapter-worlds.wit"),
            r#"
package test:adapter;

interface host-exit {
  exit: func(code: u32);
}

world app {
  import host-exit;
  export run: func();
}

world preview1-adapter {
  import host-exit;
}
"#,
        )
        .unwrap();

        let main_wasm = wat::parse_str(
            r#"
(module
  (type (func (param i32)))
  (type (func))
  (import "wasi_snapshot_preview1" "proc_exit" (func $proc_exit (type 0)))
  (func (export "run") (type 1)
    i32.const 7
    call $proc_exit)
)
"#,
        )
        .unwrap();

        let mut adapter_wasm = wat::parse_str(
            r#"
(module
  (type (func (param i32)))
  (import "test:adapter/host-exit" "exit" (func $exit (type 0)))
  (func (export "proc_exit") (type 0)
    local.get 0
    call $exit)
)
"#,
        )
        .unwrap();
        embed_component_metadata_for_world(&mut adapter_wasm, &wit_dir, "preview1-adapter")
            .expect("adapter metadata embedding should succeed");

        let component = componentize_core_module(
            &main_wasm,
            &wit_dir,
            "app",
            &[NamedAdapter {
                name: "wasi_snapshot_preview1",
                bytes: &adapter_wasm,
            }],
        )
        .expect("componentization should succeed");

        let engine = Engine::default();
        let component = Component::new(&engine, &component).expect("component should validate");
        let mut store = Store::new(&engine, Vec::<u32>::new());
        let mut linker: Linker<Vec<u32>> = Linker::new(&engine);
        linker
            .instance("test:adapter/host-exit")
            .expect("instance should be definable")
            .func_wrap("exit", |mut store, (code,): (u32,)| {
                store.data_mut().push(code);
                Ok(())
            })
            .expect("host exit should be linkable");

        let instance = linker
            .instantiate(&mut store, &component)
            .expect("component should instantiate");
        let run = instance
            .get_func(&mut store, "run")
            .expect("run export should exist");
        run.call(&mut store, &[], &mut [])
            .expect("run export should execute");
        assert_eq!(
            *store.data(),
            vec![7],
            "adapter should bridge proc_exit to host exit"
        );

        let _ = fs::remove_dir_all(&wit_dir);
    }

    #[test]
    fn test_embed_component_metadata_for_http_handler_world_resolves_vendored_http_deps() {
        let wit_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("wit")
            .join("lsharp-http-handler.wit");
        let mut module = wat::parse_str("(module)").unwrap();
        embed_component_metadata_for_world(&mut module, &wit_dir, "lsharp-http-handler")
            .expect("http handler world は vendored wasi:http deps で解決できるべき");
    }

    #[test]
    fn test_componentize_linear_list_u8_output_exposes_canonical_pair_contract() {
        let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("wit")
            .join("lsharp-wasmgc-output.wit");

        let core_wasm = wat::parse_str(
            r#"
(module
  (type (func (param i32 i32)))
  (type (func (result i64)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (memory (export "memory") 1)
  (func (export "main") (type 1)
    i64.const 0)
)
"#,
        )
        .unwrap();

        let component = componentize_core_module(&core_wasm, &wit_file, "wasmgc-output", &[])
            .expect("linear list<u8> import は canonical pair へ lower できるべき");
        Component::new(&Engine::default(), &component)
            .expect("canonical list<u8> の component は validation に成功するべき");
    }

    #[test]
    fn test_componentize_wasmgc_core_reports_missing_gc_component_bridge() {
        let wit_dir = unique_temp_dir("wasmgc_bridge");
        fs::write(
            wit_dir.join("world.wit"),
            r#"
package test:adapter;

world app {
  export main: func() -> s64;
}
"#,
        )
        .unwrap();

        let core_wasm = crate::wasmgc::emit_wasm_wasmgc(&lsharp_ir::Module {
            functions: vec![lsharp_ir::Function {
                name: "main".to_string(),
                params: vec![],
                result: lsharp_ir::IrType::I64,
                locals: vec![],
                body: vec![
                    lsharp_ir::Instruction::I32Const(65),
                    lsharp_ir::Instruction::ArrayNewFixed(0, 1),
                    lsharp_ir::Instruction::Call(4),
                    lsharp_ir::Instruction::I64Const(0),
                ],
                is_export: true,
            }],
            gc_types: vec![lsharp_ir::GcTypeDef {
                name: "StringBytes".to_string(),
                kind: lsharp_ir::GcTypeKind::PackedByteArray,
            }],
            imports: vec![],
            globals: vec![],
            string_data: vec![],
        })
        .expect("WasmGC print-string core module should be generated");
        let error = componentize_core_module(&core_wasm, &wit_dir, "app", &[])
            .expect_err("WasmGC core は GC bridge 未実装のまま component 化してはならない");

        assert!(
            error.to_string().contains("WasmGC")
                && error.to_string().contains("GC")
                && error.to_string().contains("component"),
            "WasmGC component bridge の失敗境界を明示するべき: {error}"
        );

        let _ = fs::remove_dir_all(&wit_dir);
    }
}
