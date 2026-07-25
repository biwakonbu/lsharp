use wasm_encoder::CodeSection;

use super::read_file::emit_read_file_func;

#[test]
fn read_file_module_emits_read_file_function_body() {
    let mut codes = CodeSection::new();

    emit_read_file_func(&mut codes, 1, 2, 3, 4, 5);

    assert_eq!(codes.len(), 1);
}
