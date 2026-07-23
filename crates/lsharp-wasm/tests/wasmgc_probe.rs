use lsharp_ir::{
    Function, GcField, GcTypeDef, GcTypeKind, Instruction, IrType, Module as IrModule,
};
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
