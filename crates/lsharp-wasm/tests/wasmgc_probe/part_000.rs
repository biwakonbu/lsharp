use lsharp_ir::{
    Function, GcField, GcTypeDef, GcTypeKind, Instruction, IrType, Module as IrModule, link_modules,
};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use wasmtime::{Config, Engine, Instance, Module, Store};

#[test]
fn wasm_gc_struct_probe_executes_with_wasmtime_29() {
    let bytes = lsharp_wasm::wasmgc::emit_minimal_struct_probe();

    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC を有効化した engine を作成できる");
    let module = Module::new(&engine, bytes).expect("WasmGC struct module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance =
        Instance::new(&mut store, &module, &[]).expect("WasmGC module を instantiate できる");
    let read_field = instance
        .get_typed_func::<(), i64>(&mut store, "read-field")
        .expect("read-field export が存在する");

    let value = read_field
        .call(&mut store, ())
        .expect("struct.new と struct.get が実行できる");
    assert_eq!(value, 42);
}

#[test]
fn wasm_gc_struct_probe_is_rejected_without_gc_feature() {
    let bytes = lsharp_wasm::wasmgc::emit_minimal_struct_probe();
    let engine = Engine::default();

    assert!(Module::new(&engine, bytes).is_err());
}

#[test]
fn wasm_gc_emitter_lowers_ir_struct_new_and_get() {
    let module = IrModule {
        functions: vec![Function {
            name: "read-field".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I64Const(42),
                Instruction::StructNew(0),
                Instruction::StructGet(0, 0),
            ],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "Point".to_string(),
            kind: GcTypeKind::Struct(vec![GcField {
                name: "value".to_string(),
                ty: IrType::I64,
                mutable: false,
            }]),
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("L# IR の GC struct module を生成できる");

    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("生成した IR module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance =
        Instance::new(&mut store, &module, &[]).expect("IR module を instantiate できる");
    let read_field = instance
        .get_typed_func::<(), i64>(&mut store, "read-field")
        .expect("read-field export が存在する");

    assert_eq!(read_field.call(&mut store, ()).unwrap(), 42);
}

#[test]
fn wasm_gc_emitter_lowers_ir_struct_set_and_ref_cast() {
    let module = IrModule {
        functions: vec![Function {
            name: "update-field".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![IrType::Ref(0)],
            body: vec![
                Instruction::I64Const(1),
                Instruction::StructNew(0),
                Instruction::LocalSet(0),
                Instruction::LocalGet(0),
                Instruction::I64Const(42),
                Instruction::StructSet(0, 0),
                Instruction::LocalGet(0),
                Instruction::RefCast(0),
                Instruction::StructGet(0, 0),
            ],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "MutablePoint".to_string(),
            kind: GcTypeKind::Struct(vec![GcField {
                name: "value".to_string(),
                ty: IrType::I64,
                mutable: true,
            }]),
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("StructSet / RefCast を含む IR module を生成できる");

    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("生成した mutable struct module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance =
        Instance::new(&mut store, &module, &[]).expect("IR module を instantiate できる");
    let update_field = instance
        .get_typed_func::<(), i64>(&mut store, "update-field")
        .expect("update-field export が存在する");

    assert_eq!(update_field.call(&mut store, ()).unwrap(), 42);
}

#[test]
fn wasm_gc_emitter_rejects_linear_memory_instruction_instead_of_fallback() {
    let module = IrModule {
        functions: vec![Function {
            name: "read-memory".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![Instruction::I64Const(0), Instruction::I64Load { offset: 0 }],
            is_export: true,
        }],
        gc_types: vec![],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let error = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect_err("Stage 1 backend は linear-memory 命令を受け入れてはならない");
    assert!(error.to_string().contains("未対応の命令"));
}

#[test]
fn wasm_gc_emitter_rejects_lowered_runtime_call_without_import_boundary() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![Instruction::Call(0)],
            is_export: true,
        }],
        gc_types: vec![],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let error = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect_err("runtime import を local user function として偽装してはならない");
    assert!(error.to_string().contains("runtime import"));
}

#[test]
fn wasm_gc_emitter_uses_unsigned_get_for_packed_byte_array() {
    let module = IrModule {
        functions: vec![Function {
            name: "read-byte".to_string(),
            params: vec![],
            result: IrType::I32,
            locals: vec![],
            body: vec![
                Instruction::I32Const(255),
                Instruction::ArrayNewFixed(0, 1),
                Instruction::I32Const(0),
                Instruction::ArrayGet(0),
            ],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "StringBytes".to_string(),
            kind: GcTypeKind::PackedByteArray,
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("packed byte array を含む WasmGC module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("packed byte array module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])
        .expect("packed byte array module を instantiate できる");
    let read_byte = instance
        .get_typed_func::<(), i32>(&mut store, "read-byte")
        .expect("read-byte export が存在する");

    assert_eq!(read_byte.call(&mut store, ()).unwrap(), 255);
}

#[test]
fn wasm_gc_emitter_materializes_print_string_import_boundary() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I32Const(65),
                Instruction::ArrayNewFixed(0, 1),
                Instruction::Call(4),
                Instruction::I64Const(0),
            ],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "StringBytes".to_string(),
            kind: GcTypeKind::PackedByteArray,
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("print-string の external import boundary を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("print-string import module を検証できる");
    let import = module
        .imports()
        .next()
        .expect("print-string import が存在する");
    assert_eq!(import.module(), "env");
    assert_eq!(import.name(), "print-string");

    let mut store = Store::new(&engine, ());
    let wasmtime::ExternType::Func(func_type) = import.ty() else {
        panic!("print-string import は function であるべき");
    };
    let print_string = wasmtime::Func::new(
        &mut store,
        func_type.clone(),
        |_caller, _params, _results| Ok(()),
    );
    let instance = Instance::new(&mut store, &module, &[print_string.into()])
        .expect("print-string import を stub で解決できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("main export が存在する");
    assert_eq!(main.call(&mut store, ()).unwrap(), 0);
}

#[test]
fn wasm_gc_emitter_offsets_user_calls_after_print_string_import() {
    let module = IrModule {
        functions: vec![
            Function {
                name: "main".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![
                    Instruction::I32Const(65),
                    Instruction::ArrayNewFixed(0, 1),
                    Instruction::Call(4),
                    Instruction::I64Const(41),
                    Instruction::Call(18),
                ],
                is_export: true,
            },
            Function {
                name: "answer".to_string(),
                params: vec![IrType::I64],
                result: IrType::I64,
                locals: vec![],
                body: vec![Instruction::I64Const(42)],
                is_export: false,
            },
        ],
        gc_types: vec![GcTypeDef {
            name: "StringBytes".to_string(),
            kind: GcTypeKind::PackedByteArray,
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("print-string import 後の user call を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("user call module を検証できる");
    let mut store = Store::new(&engine, ());
    let import = module
        .imports()
        .next()
        .expect("print-string import が存在する");
    let wasmtime::ExternType::Func(func_type) = import.ty() else {
        panic!("print-string import は function であるべき");
    };
    let print_string = wasmtime::Func::new(
        &mut store,
        func_type.clone(),
        |_caller, _params, _results| Ok(()),
    );
    let instance = Instance::new(&mut store, &module, &[print_string.into()])
        .expect("print-string import と user call を解決できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("main export が存在する");
    assert_eq!(main.call(&mut store, ()).unwrap(), 42);
}

#[test]
fn wasm_gc_host_print_string_reads_packed_bytes() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I32Const(195),
                Instruction::I32Const(169),
                Instruction::ArrayNewFixed(0, 2),
                Instruction::Call(4),
                Instruction::I64Const(0),
            ],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "StringBytes".to_string(),
            kind: GcTypeKind::PackedByteArray,
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("print-string host read module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("print-string module を検証できる");
    let mut store = Store::new(&engine, ());
    let import = module
        .imports()
        .next()
        .expect("print-string import が存在する");
    let wasmtime::ExternType::Func(func_type) = import.ty() else {
        panic!("print-string import は function であるべき");
    };
    let printed = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let printed_for_host = Arc::clone(&printed);
    let print_string = lsharp_wasm::wasmgc_host::create_print_string_import(
        &mut store,
        func_type.clone(),
        move |bytes| {
            printed_for_host.lock().unwrap().push(bytes.to_vec());
            Ok(())
        },
    )
    .expect("packed StringBytes 用 host import を作成できる");
    let instance = Instance::new(&mut store, &module, &[print_string.into()])
        .expect("print-string host import を解決できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("main export が存在する");
    assert_eq!(main.call(&mut store, ()).unwrap(), 0);
    assert_eq!(*printed.lock().unwrap(), vec![vec![195, 169]]);
}

#[test]
fn wasm_gc_host_print_string_rejects_null_reference_at_runtime() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::RefNull(0),
                Instruction::Call(4),
                Instruction::I64Const(0),
            ],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "StringBytes".to_string(),
            kind: GcTypeKind::PackedByteArray,
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("null StringBytes host boundary module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("null print-string module を検証できる");
    let mut store = Store::new(&engine, ());
    let import = module
        .imports()
        .next()
        .expect("print-string import が存在する");
    let wasmtime::ExternType::Func(func_type) = import.ty() else {
        panic!("print-string import は function であるべき");
    };
    let print_string = lsharp_wasm::wasmgc_host::create_print_string_import(
        &mut store,
        func_type.clone(),
        |_bytes| Ok(()),
    )
    .expect("packed StringBytes 用 host import を作成できる");
    let instance = Instance::new(&mut store, &module, &[print_string.into()])
        .expect("null print-string host import を解決できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("main export が存在する");
    let error = main
        .call(&mut store, ())
        .expect_err("null reference は trap になる");
    assert!(format!("{error:#}").contains("null reference"), "{error:?}");
}

#[test]
fn wasm_gc_host_print_string_rejects_non_packed_import_signature() {
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let array_type = wasmtime::ArrayType::new(
        &engine,
        wasmtime::FieldType::new(
            wasmtime::Mutability::Var,
            wasmtime::StorageType::ValType(wasmtime::ValType::I32),
        ),
    );
    let func_type = wasmtime::FuncType::new(
        &engine,
        [wasmtime::ValType::Ref(wasmtime::RefType::new(
            true,
            wasmtime::HeapType::ConcreteArray(array_type),
        ))],
        [],
    );
    let mut store = Store::new(&engine, ());
    let error = lsharp_wasm::wasmgc_host::create_print_string_import(
        &mut store,
        func_type,
        |_bytes| Ok(()),
    )
    .expect_err("i32 array signature は拒否する");
    assert!(error.contains("i8"), "{error}");
}

#[test]
fn wasm_gc_component_output_copies_packed_array_to_linear_memory_import() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I32Const(195),
                Instruction::I32Const(169),
                Instruction::ArrayNewFixed(0, 2),
                Instruction::Call(4),
                Instruction::I64Const(11),
            ],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "StringBytes".to_string(),
            kind: GcTypeKind::PackedByteArray,
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc_component_output(&module)
        .expect("WasmGC component output module を生成できる");
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_capture(&bytes)
        .expect("GC array を linear memory へ copy して canonical import を実行できる");

    assert_eq!(output.stdout, "é");
    assert_eq!(output.exit_code, 11);
}

#[test]
fn wasm_gc_component_output_componentizes_against_wit_world() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I32Const(65),
                Instruction::ArrayNewFixed(0, 1),
                Instruction::Call(4),
                Instruction::I64Const(0),
            ],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "StringBytes".to_string(),
            kind: GcTypeKind::PackedByteArray,
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let core = lsharp_wasm::wasmgc::emit_wasm_wasmgc_component_output(&module)
        .expect("component output core module を生成できる");
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-output",
        &[],
    )
    .expect("canonical output core module を WIT component へ変換できる");

    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC component engine を作成できる");
    wasmtime::component::Component::new(&engine, component)
        .expect("canonical output component が validation に成功する");
}

#[test]
fn wasm_gc_component_output_component_runner_executes_wit_host() {
    let core = emit_component_output_probe_module(&[195, 169], 23);
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-output",
        &[],
    )
    .expect("WasmGC output core を componentize できる");
    let output =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_component_capture(&component)
            .expect("WIT stdout host を link して Component を実行できる");

    assert_eq!(output.stdout, "é");
    assert_eq!(output.exit_code, 23);
}

#[test]
fn wasm_gc_component_output_artifact_round_trip_preserves_preview2_runtime() {
    let core = emit_component_output_probe_module(&[65, 66], 37);
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-output",
        &[],
    )
    .expect("WasmGC output core を componentize できる");
    let direct =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_component_capture(&component)
            .expect("in-memory Component を実行できる");

    let artifact = persist_and_reload_wasmgc_component_artifact(&component)
        .expect("Component artifact を保存して再読込できる");
    assert_eq!(
        artifact, component,
        "保存・再読込で Component bytes を変質させない"
    );
    wasmparser::Validator::new()
        .validate_all(&artifact)
        .expect("再読込した Component artifact が validation に成功する");
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("artifact runtime 用 WasmGC engine を作成できる");
    wasmtime::component::Component::new(&engine, &artifact)
        .expect("再読込した Component artifact を instantiate 可能な形で検証できる");

    let round_trip =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_component_capture(&artifact)
            .expect("再読込した Component artifact を同じ host runtime で実行できる");
    assert_eq!(round_trip.stdout, direct.stdout);
    assert_eq!(round_trip.exit_code, direct.exit_code);
    assert_eq!(round_trip.stdout, "AB");
    assert_eq!(round_trip.exit_code, 37);
}

#[test]
fn wasm_gc_component_output_component_runner_propagates_sink_failure() {
    let core = emit_component_output_probe_module(&[65], 0);
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-output",
        &[],
    )
    .expect("WasmGC output core を componentize できる");
    let error =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_component_with_stdout_sink(
            &component,
            |_bytes| Err("component stdout closed".to_string()),
        )
        .expect_err("Component host sink error は trap になる");

    assert!(error.contains("component stdout closed"), "{error}");
}

#[test]
fn wasm_gc_component_output_component_runner_connects_preview2_stdout_stream() {
    let core = emit_component_output_probe_module(&[80, 50], 29);
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-output",
        &[],
    )
    .expect("WasmGC output core を componentize できる");

    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_component_with_preview2_stdout(
        &component,
        None,
        &[],
        "",
    )
    .expect("WASI Preview2 stdout stream を使って Component を実行できる");

    assert_eq!(output.stdout, "P2");
    assert_eq!(output.exit_code, 29);
}

#[test]
fn wasm_gc_component_output_cli_world_rejects_core_without_wasi_cli_run_export() {
    let core = emit_component_output_probe_module(&[67, 76, 73], 0);
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");

    let error = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli",
        &[],
    )
    .expect_err("wasi:cli/run export のない core は CLI world に変換できない");

    assert!(error.to_string().contains("wasi:cli/run"), "{error}");
}

#[test]
fn wasm_gc_component_output_cli_world_accepts_canonical_run_export() {
    let core = emit_component_output_cli_run_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");

    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli",
        &[],
    )
    .expect("canonical wasi:cli/run export を CLI world に変換できる");
    wasmparser::Validator::new()
        .validate_all(&component)
        .expect("WasmGC CLI component が validation に成功する");
}
