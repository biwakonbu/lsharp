use wasm_encoder::CodeSection;

use super::write_file_bytes::emit_write_file_bytes_func;

#[test]
fn write_file_bytes_module_emits_write_file_bytes_function_body() {
    let mut codes = CodeSection::new();

    emit_write_file_bytes_func(&mut codes, 1, 2, 3, 4);

    assert_eq!(codes.len(), 1);
}
