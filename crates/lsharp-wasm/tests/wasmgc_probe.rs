use lsharp_ir::{
    Function, GcField, GcTypeDef, GcTypeKind, Instruction, IrType, Module as IrModule,
};
use std::sync::{Arc, Mutex};
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
