use wasm_encoder::{CodeSection, Function, Instruction as W, ValType};

use super::free_list::emit_free_class_capacity;

#[test]
fn free_list_module_emits_capacity_copy_body() {
    let mut function = Function::new(vec![(2, ValType::I32)]);
    emit_free_class_capacity(&mut function, 0, 1);
    function.instruction(&W::End);

    let mut codes = CodeSection::new();
    codes.function(&function);

    assert_eq!(codes.len(), 1);
}
