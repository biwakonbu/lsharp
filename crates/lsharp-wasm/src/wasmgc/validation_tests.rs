use lsharp_ir::IrType;
use wasm_encoder::ValType;

use super::validation::{validate_gc_type_index, wasm_gc_valtype};

#[test]
fn validation_module_converts_scalar_and_checks_gc_indices() {
    assert_eq!(wasm_gc_valtype(IrType::I64, 0), ValType::I64);
    validate_gc_type_index(0, 1, "test").expect("in-range GC type index should pass");
    assert!(validate_gc_type_index(1, 1, "test").is_err());
}
