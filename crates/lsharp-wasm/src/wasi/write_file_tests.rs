use wasm_encoder::CodeSection;

use super::write_file::emit_write_file_func;

#[test]
fn write_file_module_emits_write_file_function_body() {
    let mut codes = CodeSection::new();

    emit_write_file_func(&mut codes, 1, 2, 3);

    assert_eq!(codes.len(), 1);
}
