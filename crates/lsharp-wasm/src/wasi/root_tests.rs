use wasm_encoder::CodeSection;

use super::root::{emit_root_pop_func, emit_root_push_func, emit_root_set_func};

#[test]
fn root_module_emits_push_pop_and_set_function_bodies() {
    let mut codes = CodeSection::new();

    emit_root_push_func(&mut codes, 1, 2, 3, 4);
    emit_root_pop_func(&mut codes, 2, 3);
    emit_root_set_func(&mut codes, 2, 3, 4, 5, 6);

    assert_eq!(codes.len(), 3);
}
