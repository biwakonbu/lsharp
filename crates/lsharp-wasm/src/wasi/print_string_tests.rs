use wasm_encoder::CodeSection;

use super::print_string::emit_print_string_func;

#[test]
fn print_string_module_emits_print_string_function_body() {
    let mut codes = CodeSection::new();

    emit_print_string_func(&mut codes);

    assert_eq!(codes.len(), 1);
}
