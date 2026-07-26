use wasm_encoder::CodeSection;

use super::string_eq::emit_string_eq_func;

#[test]
fn string_eq_module_emits_string_eq_function_body() {
    let mut codes = CodeSection::new();

    emit_string_eq_func(&mut codes);

    assert_eq!(codes.len(), 1);
}
