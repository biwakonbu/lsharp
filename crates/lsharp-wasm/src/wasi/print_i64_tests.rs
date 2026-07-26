use wasm_encoder::CodeSection;

use super::print_i64::emit_print_i64_func;

#[test]
fn print_i64_module_emits_print_function_body() {
    let mut codes = CodeSection::new();

    emit_print_i64_func(&mut codes);

    assert_eq!(codes.len(), 1);
}
