use lsharp_ir::{Function, GcTypeDef, GcTypeKind, Instruction, IrType, Module as IrModule};
use wasmtime::{Config, Engine, Module};

#[test]
fn wasm_gc_component_output_materializes_canonical_write_boundary() {
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

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc_component_output(&module)
        .expect("Component output の canonical write boundary を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("Component output core module を検証できる");

    let import = module
        .imports()
        .next()
        .expect("canonical write import が存在する");
    assert_eq!(import.module(), "lsharp:wasmgc-output/stdout@0.1.0");
    assert_eq!(import.name(), "write");
    assert!(
        module.get_export("memory").is_some(),
        "linear memory を export する"
    );
    assert!(
        module.get_export("main").is_some(),
        "main export を保持する"
    );
}
