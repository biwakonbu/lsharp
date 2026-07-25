use wasm_encoder::CodeSection;

use super::stdin::emit_read_stdin_func;

#[test]
fn stdin_module_emits_read_stdin_function_body() {
    let mut codes = CodeSection::new();

    emit_read_stdin_func(&mut codes, 10, 11, 12);

    assert_eq!(codes.len(), 1);
}
