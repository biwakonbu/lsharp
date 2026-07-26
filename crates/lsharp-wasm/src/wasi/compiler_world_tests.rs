use lsharp_ir::Module;

use super::compiler_world::emit_wasm_wasi_with_options;

#[test]
fn compiler_world_module_emits_empty_wasi_core_module() {
    let module = Module {
        functions: Vec::new(),
        gc_types: Vec::new(),
        imports: Vec::new(),
        globals: Vec::new(),
        string_data: Vec::new(),
    };

    let wasm = emit_wasm_wasi_with_options(&module, false)
        .expect("empty compiler-world module should emit valid core Wasm");
    assert_eq!(&wasm[..4], b"\0asm");
}
