use wasm_encoder::CodeSection;

use super::int_to_string::emit_int_to_string_func;

#[test]
fn int_to_string_module_emits_int_to_string_function_body() {
    let mut codes = CodeSection::new();

    emit_int_to_string_func(&mut codes, 1);

    assert_eq!(codes.len(), 1);
}
