use lsharp_ir::{
    Function, GcField, GcTypeDef, GcTypeKind, Instruction, IrType, Module as IrModule,
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

#[test]
fn wasm_gc_component_output_cli_backend_emits_canonical_run_export() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I32Const(67),
                Instruction::I32Const(76),
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
    let core = lsharp_wasm::wasmgc::emit_wasm_wasmgc_component_cli(&module)
        .expect("WasmGC CLI backend が canonical run export を生成できる");
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
    .expect("WasmGC CLI backend の core を componentize できる");
    wasmparser::Validator::new()
        .validate_all(&component)
        .expect("WasmGC CLI component が validation に成功する");
}

#[test]
fn wasm_gc_component_cli_runner_executes_wasi_cli_run_with_preview2_stdout() {
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I32Const(67),
                Instruction::I32Const(76),
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
    let core = lsharp_wasm::wasmgc::emit_wasm_wasmgc_component_cli(&module)
        .expect("WasmGC CLI backend が core を生成できる");
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
    .expect("WasmGC CLI core を componentize できる");

    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout(
        &component,
        None,
        &[],
        "",
    )
    .expect("WASI Preview2 wasi:cli/run で WasmGC Component を実行できる");

    assert_eq!(output.stdout, "CL");
    assert_eq!(output.exit_code, 0);
}

#[test]
fn wasm_gc_component_cli_runner_maps_wasi_cli_exit_to_exit_status() {
    let core = emit_component_output_cli_exit_probe_module(1);
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
    .expect("wasi:cli/exit を使う WasmGC CLI core を componentize できる");

    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout(
        &component,
        None,
        &[],
        "",
    )
    .expect("wasi:cli/exit は終了コードとして扱える");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 1);
}

#[test]
fn wasm_gc_component_cli_runner_maps_failed_wasi_cli_run_result_to_exit_status() {
    let core = emit_component_output_cli_run_probe_module_with_result(1);
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
    .expect("失敗 result を返す WasmGC CLI core を componentize できる");

    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout(
        &component,
        None,
        &[],
        "",
    )
    .expect("wasi:cli/run の失敗 result は終了コードとして扱える");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 1);
}

#[test]
fn wasm_gc_component_cli_fs_runner_enforces_preopen_rights() {
    let core = emit_component_cli_preopen_write_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs",
        &[],
    )
    .expect("filesystem capability を持つ WasmGC CLI core を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_fd_rights_{nonce}"));
    std::fs::create_dir_all(&dir).expect("fd rights fixture directory を作成できる");
    let probe_file = dir.join("rights.txt");

    let no_preopen = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopen_rights(
        &component,
        None,
        &[],
        "",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    )
    .expect("preopen がない Component も明示的な失敗 result を返せる");
    assert_eq!(no_preopen.exit_code, 1);

    let read_only = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopen_rights(
        &component,
        Some(&dir),
        &[],
        "",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    )
    .expect("read-only preopen は Component を実行できる");
    assert_eq!(read_only.exit_code, 1);
    assert!(
        !probe_file.exists(),
        "read-only preopen は create を許可しない"
    );

    let read_write = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopen_rights(
        &component,
        Some(&dir),
        &[],
        "",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    )
    .expect("read-write preopen は Component を実行できる");
    assert_eq!(read_write.exit_code, 0);
    assert!(
        probe_file.exists(),
        "read-write preopen は create を許可する"
    );

    std::fs::remove_dir_all(&dir).expect("fd rights fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_reads_named_preopen_stream_and_drops_resources() {
    let core = emit_component_cli_named_preopen_stream_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs-streams",
        &[],
    )
    .expect("named preopen stream probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_named_preopen_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_named_preopen_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("named preopen fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second named preopen fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("stream fixture file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("named preopen の input stream を実行できる");

    assert_eq!(output.stdout, "hello");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("named preopen fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second named preopen fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_reads_descriptor_directly_and_reports_eof() {
    let core = emit_component_cli_direct_read_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs",
        &[],
    )
    .expect("descriptor direct read probe を componentize できる");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_direct_read_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_direct_read_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("direct read fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second direct read fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello").expect("direct read fixture file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("descriptor direct read を実行できる");

    assert_eq!(output.stdout, "hello");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("direct read fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second direct read fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_writes_and_appends_streams_then_drops_resources() {
    let core = emit_component_cli_write_stream_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs-streams",
        &[],
    )
    .expect("write/append stream probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_write_stream_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_write_stream_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("write stream fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second write stream fixture directory を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("write/append stream を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("output.txt")).expect("write stream の成果物を読める"),
        b"hello!"
    );
    std::fs::remove_dir_all(&dir).expect("write stream fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second write stream fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_writes_descriptor_directly_and_stats_file() {
    let core = emit_component_cli_direct_write_stat_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs",
        &[],
    )
    .expect("direct write/stat probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_direct_write_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_direct_write_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("direct write fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second direct write fixture directory を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("descriptor direct write/stat を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("output.txt")).expect("direct write の成果物を読める"),
        b"hello"
    );
    std::fs::remove_dir_all(&dir).expect("direct write fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second direct write fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_drops_descriptor_after_direct_write_error() {
    let core = emit_component_cli_direct_write_error_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs",
        &[],
    )
    .expect("direct write error probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_write_error_{nonce}"));
    let extra_dir = std::env::temp_dir().join(format!("lsharp_wasmgc_write_error_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("write error fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir).expect("second write error fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"seed").expect("write error fixture file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_write(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("read-only descriptor の direct write error を実行できる");

    assert_eq!(output.stdout, "");
    assert_eq!(output.exit_code, 0);
    assert_eq!(
        std::fs::read(dir.join("input.txt")).expect("write error fixture の成果物を読める"),
        b"seed"
    );
    std::fs::remove_dir_all(&dir).expect("write error fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir).expect("second write error fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_cli_fs_runner_reads_directory_entries_and_drops_stream() {
    let core = emit_component_cli_read_directory_probe_module();
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-wasmgc-output.wit");
    let component = lsharp_wasm::component_adapter::componentize_core_module(
        &core,
        &wit_file,
        "wasmgc-cli-fs",
        &[],
    )
    .expect("read-directory probe を componentize できる");

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock は unix epoch より後であるべき")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_wasmgc_read_directory_{nonce}"));
    let extra_dir =
        std::env::temp_dir().join(format!("lsharp_wasmgc_read_directory_extra_{nonce}"));
    std::fs::create_dir_all(&dir).expect("read-directory fixture directory を作成できる");
    std::fs::create_dir_all(&extra_dir)
        .expect("second read-directory fixture directory を作成できる");
    std::fs::write(dir.join("input.txt"), b"hello")
        .expect("read-directory fixture file を作成できる");

    let preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &dir,
        "data",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let extra_preopen = lsharp_wasm::wasmgc_runner::Preview2Preopen::new(
        &extra_dir,
        "extra",
        lsharp_wasm::wasmgc_runner::Preview2PreopenRights::read_only(),
    );
    let output = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_cli_with_preview2_stdout_and_preopens(
        &component,
        &[],
        "",
        &[preopen, extra_preopen],
    )
    .expect("read-directory と directory-entry stream を実行できる");

    assert_eq!(output.stdout, "input.txt");
    assert_eq!(output.exit_code, 0);
    std::fs::remove_dir_all(&dir).expect("read-directory fixture directory を削除できる");
    std::fs::remove_dir_all(&extra_dir)
        .expect("second read-directory fixture directory を削除できる");
}

#[test]
fn wasm_gc_component_output_propagates_sink_failure_as_trap() {
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
        .expect("component output sink failure module を生成できる");
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_with_stdout_sink(
        &core,
        |_bytes| Err("stdout closed".to_string()),
    )
    .expect_err("component output sink error は trap になる");
    assert!(error.contains("stdout closed"), "{error}");
}

#[test]
fn wasm_gc_component_output_rejects_invalid_linear_memory_range() {
    let core = wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (result i64)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (memory (export "memory") 1)
  (func (export "main") (type 1)
    i32.const 65536
    i32.const 1
    call $write
    i64.const 0)
)
"#,
    )
    .expect("invalid range module を生成できる");
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_with_stdout_sink(
        &core,
        |_bytes| Ok(()),
    )
    .expect_err("linear memory 外の canonical pair は拒否する");
    assert!(error.contains("linear memory 外"), "{error}");
}

#[test]
fn wasm_gc_runner_connects_print_string_to_stdout_sink() {
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
                Instruction::I64Const(7),
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
    let bytes =
        lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module).expect("runner sink module を生成できる");
    let printed = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let printed_for_sink = Arc::clone(&printed);
    let exit_code =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_with_stdout_sink(&bytes, move |bytes| {
            printed_for_sink.lock().unwrap().push(bytes.to_vec());
            Ok(())
        })
        .expect("runner が print-string sink を接続できる");

    assert_eq!(exit_code, 7);
    assert_eq!(*printed.lock().unwrap(), vec![vec![195, 169]]);

    let captured = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_capture(&bytes)
        .expect("runner が stdout と exit code を capture できる");
    assert_eq!(captured.stdout, "é");
    assert_eq!(captured.exit_code, 7);
}

#[test]
fn wasm_gc_runner_propagates_stdout_sink_failure() {
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
        .expect("runner sink failure module を生成できる");
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_with_stdout_sink(&bytes, |_bytes| {
        Err("stdout closed".to_string())
    })
    .expect_err("sink failure は runner error になる");

    assert!(error.contains("stdout closed"), "{error}");
}

#[test]
fn wasm_gc_runner_rejects_non_print_string_import_without_wasi_fallback() {
    let bytes = wat::parse_str(
        r#"
        (module
          (import "env" "unsupported" (func))
          (func (export "main") (result i64)
            i64.const 0))
        "#,
    )
    .expect("unsupported import module を生成できる");
    let error =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_with_stdout_sink(&bytes, |_bytes| Ok(()))
            .expect_err("unsupported import は WASI fallback せず拒否する");

    assert!(error.contains("未対応"), "{error}");
}

#[test]
fn wasm_gc_runner_write_adapter_retries_partial_writes_until_chunk_is_consumed() {
    let bytes = emit_print_string_probe_module(&[195, 169], 0);
    let output = Arc::new(Mutex::new(Vec::new()));
    let writer = OneByteWriter {
        output: Arc::clone(&output),
    };
    let exit_code = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_to_writer(&bytes, writer)
        .expect("partial writer adapter が chunk 全体を書き切れる");

    assert_eq!(exit_code, 0);
    assert_eq!(*output.lock().unwrap(), vec![195, 169]);
}

#[test]
fn wasm_gc_runner_write_adapter_propagates_write_error() {
    let bytes = emit_print_string_probe_module(&[65], 0);
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_to_writer(&bytes, FailingWriter)
        .expect_err("writer error は runner error になる");

    assert!(error.contains("stdout closed"), "{error}");
}

#[test]
fn wasm_gc_runner_write_adapter_rejects_write_zero() {
    let bytes = emit_print_string_probe_module(&[65], 0);
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_to_writer(&bytes, ZeroWriter)
        .expect_err("WriteZero は runner error になる");

    assert!(error.contains("failed"), "{error}");
}

#[test]
fn wasm_gc_runner_write_adapter_propagates_flush_error_after_execution() {
    let bytes = emit_print_string_probe_module(&[65], 0);
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_to_writer(&bytes, FlushFailingWriter)
        .expect_err("flush error は runner error になる");

    assert!(error.contains("flush failed"), "{error}");
}

#[test]
fn wasm_gc_component_output_writer_retries_partial_writes_until_chunk_is_consumed() {
    let bytes = emit_component_output_probe_module(&[195, 169], 13);
    let output = Arc::new(Mutex::new(Vec::new()));
    let writer = OneByteWriter {
        output: Arc::clone(&output),
    };
    let exit_code =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_writer(&bytes, writer)
            .expect("component output writer adapter が chunk 全体を書き切れる");

    assert_eq!(exit_code, 13);
    assert_eq!(*output.lock().unwrap(), vec![195, 169]);
}

#[test]
fn wasm_gc_component_output_writer_propagates_write_error() {
    let bytes = emit_component_output_probe_module(&[65], 0);
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_writer(
        &bytes,
        FailingWriter,
    )
    .expect_err("component output writer error は runner error になる");

    assert!(error.contains("stdout closed"), "{error}");
}

#[test]
fn wasm_gc_component_output_writer_rejects_write_zero() {
    let bytes = emit_component_output_probe_module(&[65], 0);
    let error =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_writer(&bytes, ZeroWriter)
            .expect_err("component output WriteZero は runner error になる");

    assert!(error.contains("failed"), "{error}");
}

#[test]
fn wasm_gc_component_output_writer_propagates_flush_error_after_execution() {
    let bytes = emit_component_output_probe_module(&[65], 0);
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_writer(
        &bytes,
        FlushFailingWriter,
    )
    .expect_err("component output flush error は runner error になる");

    assert!(error.contains("flush failed"), "{error}");
}

#[test]
fn wasm_gc_component_output_writer_flushes_after_nonzero_exit() {
    let bytes = emit_component_output_probe_module(&[65], 7);
    let events = Arc::new(Mutex::new(Vec::new()));
    let exit_code = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_writer(
        &bytes,
        EventWriter {
            events: Arc::clone(&events),
        },
    )
    .expect("nonzero exit 後も component output writer を flush できる");

    assert_eq!(exit_code, 7);
    assert_eq!(*events.lock().unwrap(), vec!["write", "flush"]);
}

#[test]
fn wasm_gc_component_output_fd_write_retries_partial_writes() {
    let bytes = emit_component_output_probe_module(&[195, 169], 17);
    let output = Arc::new(Mutex::new(Vec::new()));
    let calls = Arc::new(Mutex::new(Vec::<(u32, usize)>::new()));
    let output_for_write = Arc::clone(&output);
    let calls_for_write = Arc::clone(&calls);
    let exit_code = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_fd_write(
        &bytes,
        1,
        move |fd, chunk| {
            calls_for_write.lock().unwrap().push((fd, chunk.len()));
            if let Some(byte) = chunk.first() {
                output_for_write.lock().unwrap().push(*byte);
                Ok(1)
            } else {
                Ok(0)
            }
        },
    )
    .expect("component output fd_write adapter が partial write を再試行できる");

    assert_eq!(exit_code, 17);
    assert_eq!(*output.lock().unwrap(), vec![195, 169]);
    assert_eq!(*calls.lock().unwrap(), vec![(1, 2), (1, 1)]);
}

#[test]
fn wasm_gc_component_output_fd_write_propagates_errno() {
    let bytes = emit_component_output_probe_module(&[65], 0);
    let error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_fd_write(
        &bytes,
        1,
        |_fd, _chunk| Err(28),
    )
    .expect_err("component output fd_write errno は runner error になる");

    assert!(error.contains("28"), "{error}");
}

#[test]
fn wasm_gc_component_output_fd_write_rejects_zero_and_overreported_counts() {
    let bytes = emit_component_output_probe_module(&[65], 0);
    let zero_error = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_fd_write(
        &bytes,
        1,
        |_fd, _chunk| Ok(0),
    )
    .expect_err("component output fd_write zero は拒否する");
    assert!(zero_error.contains("failed"), "{zero_error}");

    let overreported_error =
        lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_component_output_to_fd_write(
            &bytes,
            1,
            |_fd, chunk| Ok(chunk.len() + 1),
        )
        .expect_err("component output fd_write over-report は拒否する");
    assert!(
        overreported_error.contains("over-reported"),
        "{overreported_error}"
    );
}

struct OneByteWriter {
    output: Arc<Mutex<Vec<u8>>>,
}

impl Write for OneByteWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = usize::from(!bytes.is_empty());
        if count != 0 {
            self.output.lock().unwrap().push(bytes[0]);
        }
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "stdout closed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct ZeroWriter;

impl Write for ZeroWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FlushFailingWriter;

impl Write for FlushFailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("flush closed"))
    }
}

struct EventWriter {
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl Write for EventWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.events.lock().unwrap().push("write");
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.events.lock().unwrap().push("flush");
        Ok(())
    }
}

fn emit_print_string_probe_module(bytes: &[i32], exit_code: i64) -> Vec<u8> {
    let mut body = bytes
        .iter()
        .copied()
        .map(Instruction::I32Const)
        .collect::<Vec<_>>();
    body.push(Instruction::ArrayNewFixed(0, bytes.len() as u32));
    body.push(Instruction::Call(4));
    body.push(Instruction::I64Const(exit_code));
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body,
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
    lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module).expect("writer adapter module を生成できる")
}

fn emit_component_output_probe_module(bytes: &[i32], exit_code: i64) -> Vec<u8> {
    let mut body = bytes
        .iter()
        .copied()
        .map(Instruction::I32Const)
        .collect::<Vec<_>>();
    body.push(Instruction::ArrayNewFixed(0, bytes.len() as u32));
    body.push(Instruction::Call(4));
    body.push(Instruction::I64Const(exit_code));
    let module = IrModule {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body,
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
    lsharp_wasm::wasmgc::emit_wasm_wasmgc_component_output(&module)
        .expect("component output writer adapter module を生成できる")
}

fn emit_component_output_cli_run_probe_module() -> Vec<u8> {
    emit_component_output_cli_run_probe_module_with_result(0)
}

fn emit_component_output_cli_run_probe_module_with_result(result: i32) -> Vec<u8> {
    wat::parse_str(format!(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (result i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (memory (export "memory") 1)
  (func (export "wasi:cli/run@0.2.3#run") (type 1)
    i32.const {result})
)
"#
    ))
    .expect("canonical wasi:cli/run probe module を生成できる")
}

fn emit_component_output_cli_exit_probe_module(exit_code: i32) -> Vec<u8> {
    wat::parse_str(format!(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (result i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (import "wasi:cli/exit@0.2.3" "exit" (func $exit (param i32)))
  (memory (export "memory") 1)
  (func (export "wasi:cli/run@0.2.3#run") (type 1)
    i32.const {exit_code}
    call $exit
    i32.const 0)
)
"#
    ))
    .expect("wasi:cli/exit probe module を生成できる")
}

fn emit_component_cli_preopen_write_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "rights.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.eqz
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      i32.const 0
      i32.const 128
      i32.const 10
      i32.const 1
      i32.const 2
      i32.const 32
      call $open-at
      i32.const 32
      i32.load
    end)
)
"#,
    )
    .expect("preopen rights probe module を生成できる")
}

fn emit_component_cli_named_preopen_stream_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-via-stream" (func $read-via-stream (type 4)))
  (import "wasi:io/streams@0.2.3" "[method]input-stream.blocking-read" (func $blocking-read (param i32 i64 i32)))
  (import "wasi:io/streams@0.2.3" "[resource-drop]input-stream" (func $drop-input-stream (param i32)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "input.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    (local $stream i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 9
      i32.const 0
      i32.const 1
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 2
        return
      end
      local.get $preopen
      call $drop-descriptor
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i64.const 0
      i32.const 40
      call $read-via-stream
      i32.const 40
      i32.load8_u
      if
        i32.const 3
        return
      end
      i32.const 44
      i32.load
      local.set $stream
      local.get $stream
      i64.const 5
      i32.const 48
      call $blocking-read
      i32.const 48
      i32.load8_u
      if
        i32.const 4
        return
      end
      i32.const 52
      i32.load
      i32.const 56
      i32.load
      call $write
      local.get $stream
      call $drop-input-stream
      local.get $descriptor
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("named preopen stream probe module を生成できる")
}

fn emit_component_cli_direct_read_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i64 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read" (func $read (type 4)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "input.txt")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 9
      i32.const 0
      i32.const 1
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 2
        return
      end
      local.get $preopen
      call $drop-descriptor
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i64.const 5
      i64.const 0
      i32.const 40
      call $read
      i32.const 40
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 48
      i32.load
      i32.const 5
      i32.ne
      if
        local.get $descriptor
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 52
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 44
      i32.load
      i32.const 48
      i32.load
      call $write
      local.get $descriptor
      i64.const 1
      i64.const 5
      i32.const 40
      call $read
      i32.const 40
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 48
      i32.load
      i32.const 0
      i32.ne
      if
        local.get $descriptor
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 52
      i32.load8_u
      i32.eqz
      if
        local.get $descriptor
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $descriptor
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("descriptor direct read probe module を生成できる")
}

fn emit_component_cli_direct_write_stat_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i32 i32 i64 i32)))
  (type (func (param i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write-stdout (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.write" (func $write (type 4)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.stat" (func $stat (type 5)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "output.txt")
  (data (i32.const 256) "hello")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 10
      i32.const 5
      i32.const 2
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i32.const 256
      i32.const 5
      i64.const 0
      i32.const 40
      call $write
      i32.const 40
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 48
      i64.load
      i64.const 5
      i64.ne
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $descriptor
      i32.const 64
      call $stat
      i32.const 64
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 72
      i32.load
      i32.const 6
      i32.ne
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 88
      i64.load
      i64.const 5
      i64.ne
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("descriptor direct write/stat probe module を生成できる")
}

fn emit_component_cli_direct_write_error_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i32 i32 i64 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $write-stdout (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.write" (func $write (type 4)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "input.txt")
  (data (i32.const 256) "!")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 9
      i32.const 0
      i32.const 1
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i32.const 256
      i32.const 1
      i64.const 0
      i32.const 40
      call $write
      i32.const 40
      i32.load8_u
      i32.eqz
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("descriptor direct write error probe module を生成できる")
}

fn emit_component_cli_read_directory_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.read-directory" (func $read-directory (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]directory-entry-stream.read-directory-entry" (func $read-directory-entry (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]directory-entry-stream" (func $drop-directory-entry-stream (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $stream i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 24
      call $read-directory
      i32.const 24
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 28
      i32.load
      local.set $stream
      local.get $stream
      i32.const 32
      call $read-directory-entry
      i32.const 32
      i32.load8_u
      if
        local.get $stream
        call $drop-directory-entry-stream
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 36
      i32.load8_u
      i32.const 1
      i32.ne
      if
        local.get $stream
        call $drop-directory-entry-stream
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 40
      i32.load
      i32.const 6
      i32.ne
      if
        local.get $stream
        call $drop-directory-entry-stream
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 44
      i32.load
      i32.const 48
      i32.load
      call $stdout-write
      local.get $stream
      i32.const 64
      call $read-directory-entry
      i32.const 64
      i32.load8_u
      if
        local.get $stream
        call $drop-directory-entry-stream
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 68
      i32.load8_u
      if
        local.get $stream
        call $drop-directory-entry-stream
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $stream
      call $drop-directory-entry-stream
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("read-directory probe module を生成できる")
}

fn emit_component_cli_write_stream_probe_module() -> Vec<u8> {
    wat::parse_str(
        r#"
(module
  (type (func (param i32 i32)))
  (type (func (param i32)))
  (type (func (result i32)))
  (type (func (param i32 i32 i32 i32 i32 i32 i32)))
  (type (func (param i32 i64 i32)))
  (type (func (param i32 i32 i32 i32)))
  (type (func (param i32 i32)))
  (import "lsharp:wasmgc-output/stdout@0.1.0" "write" (func $stdout-write (type 0)))
  (import "wasi:filesystem/preopens@0.2.3" "get-directories" (func $get-directories (type 1)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.open-at" (func $open-at (type 3)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.write-via-stream" (func $write-via-stream (type 4)))
  (import "wasi:filesystem/types@0.2.3" "[method]descriptor.append-via-stream" (func $append-via-stream (type 6)))
  (import "wasi:filesystem/types@0.2.3" "[resource-drop]descriptor" (func $drop-descriptor (param i32)))
  (import "wasi:io/streams@0.2.3" "[method]output-stream.blocking-write-and-flush" (func $blocking-write-and-flush (type 5)))
  (import "wasi:io/streams@0.2.3" "[resource-drop]output-stream" (func $drop-output-stream (param i32)))
  (memory (export "memory") 2)
  (global $heap (mut i32) (i32.const 1024))
  (func (export "cabi_realloc")
    (param $old i32) (param $old-len i32) (param $align i32) (param $new-len i32)
    (result i32)
    (local $mask i32)
    (local $ptr i32)
    local.get $align
    i32.const 1
    i32.sub
    local.set $mask
    global.get $heap
    local.get $mask
    i32.add
    local.get $mask
    i32.const -1
    i32.xor
    i32.and
    local.set $ptr
    local.get $ptr
    local.get $new-len
    i32.add
    global.set $heap
    local.get $ptr)
  (data (i32.const 128) "output.txt")
  (data (i32.const 256) "hello")
  (data (i32.const 264) "!")
  (func (export "wasi:cli/run@0.2.3#run") (type 2)
    (local $preopen i32)
    (local $descriptor i32)
    (local $stream i32)
    i32.const 16
    call $get-directories
    i32.const 20
    i32.load
    i32.const 2
    i32.ne
    if (result i32)
      i32.const 1
    else
      i32.const 16
      i32.load
      i32.load
      local.set $preopen
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 10
      i32.const 5
      i32.const 2
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i64.const 0
      i32.const 40
      call $write-via-stream
      i32.const 40
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 44
      i32.load
      local.set $stream
      local.get $stream
      i32.const 256
      i32.const 5
      i32.const 48
      call $blocking-write-and-flush
      i32.const 48
      i32.load8_u
      if
        local.get $stream
        call $drop-output-stream
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      i32.const 0
      i32.const 128
      i32.const 10
      i32.const 0
      i32.const 2
      i32.const 32
      call $open-at
      i32.const 32
      i32.load8_u
      if
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 36
      i32.load
      local.set $descriptor
      local.get $descriptor
      i32.const 40
      call $append-via-stream
      i32.const 40
      i32.load8_u
      if
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      i32.const 44
      i32.load
      local.set $stream
      local.get $stream
      i32.const 264
      i32.const 1
      i32.const 48
      call $blocking-write-and-flush
      i32.const 48
      i32.load8_u
      if
        local.get $stream
        call $drop-output-stream
        local.get $descriptor
        call $drop-descriptor
        local.get $preopen
        call $drop-descriptor
        i32.const 1
        return
      end
      local.get $stream
      call $drop-output-stream
      local.get $descriptor
      call $drop-descriptor
      local.get $preopen
      call $drop-descriptor
      i32.const 0
      return
    end)
)
"#,
    )
    .expect("write/append stream probe module を生成できる")
}

#[test]
fn wasm_gc_emitter_maps_reference_typed_struct_fields() {
    let module = IrModule {
        functions: vec![Function {
            name: "read-nested-field".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I64Const(42),
                Instruction::StructNew(0),
                Instruction::StructNew(1),
                Instruction::StructGet(1, 0),
                Instruction::StructGet(0, 0),
            ],
            is_export: true,
        }],
        gc_types: vec![
            GcTypeDef {
                name: "Point".to_string(),
                kind: GcTypeKind::Struct(vec![GcField {
                    name: "value".to_string(),
                    ty: IrType::I64,
                    mutable: false,
                }]),
            },
            GcTypeDef {
                name: "Box".to_string(),
                kind: GcTypeKind::Struct(vec![GcField {
                    name: "point".to_string(),
                    ty: IrType::Ref(0),
                    mutable: false,
                }]),
            },
        ],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("reference typed field を含む IR module を生成できる");

    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("nested struct module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance =
        Instance::new(&mut store, &module, &[]).expect("IR module を instantiate できる");
    let read_nested_field = instance
        .get_typed_func::<(), i64>(&mut store, "read-nested-field")
        .expect("read-nested-field export が存在する");

    assert_eq!(read_nested_field.call(&mut store, ()).unwrap(), 42);
}

#[test]
fn wasm_gc_emitter_remaps_lowered_user_call_indices() {
    let module = IrModule {
        functions: vec![
            Function {
                name: "callee".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![Instruction::I64Const(7)],
                is_export: false,
            },
            Function {
                name: "main".to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                // Lower は runtime import 17 個の後ろを user function index として持つ。
                body: vec![Instruction::Call(17)],
                is_export: true,
            },
        ],
        gc_types: vec![],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
        .expect("lowered user call index を core Wasm index へ変換できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("user call を含む module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("module を instantiate できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("main export が存在する");

    assert_eq!(main.call(&mut store, ()).unwrap(), 7);
}
