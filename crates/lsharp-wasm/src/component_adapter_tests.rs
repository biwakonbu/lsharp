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
fn test_artifact_sync_helpers_flush_file_and_parent_directory() {
    let dir = unique_temp_dir("artifact_sync");
    let path = dir.join("Main.wasm");
    fs::write(&path, b"durable-wasm").expect("durable artifact fixture を保存できる");

    sync_artifact_file(&path).expect("artifact file を sync できる");
    sync_artifact_parent(&path).expect("artifact parent directory を sync できる");
    assert_eq!(
        fs::read(&path).expect("synced artifact を再読込できる"),
        b"durable-wasm"
    );

    fs::remove_dir_all(&dir).expect("artifact sync directory を削除できる");
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

#[test]
fn test_sync_artifact_parent_accepts_bare_file_name_without_directory_component() {
    // ディレクトリ成分の無い path の `Path::parent()` は `None` ではなく `Some("")` を返す。
    // 空文字列を `.` へ正規化していないと `File::open("")` が ENOENT で落ちる (I-51)。
    let result = sync_artifact_parent(Path::new("out.wasm"));

    assert!(
        result.is_ok(),
        "bare file name の親 directory は cwd として同期できるべき: {result:?}"
    );
}

#[test]
fn test_artifact_parent_dir_normalizes_bare_and_dotted_and_absolute_forms() {
    // 3 形が同じ artifact を生むための前提: 親 directory の解決が一致すること。
    assert_eq!(artifact_parent_dir(Path::new("out.wasm")), Path::new("."));
    assert_eq!(artifact_parent_dir(Path::new("./out.wasm")), Path::new("."));
    assert_eq!(
        artifact_parent_dir(Path::new("/tmp/lsharp/out.wasm")),
        Path::new("/tmp/lsharp")
    );
}
