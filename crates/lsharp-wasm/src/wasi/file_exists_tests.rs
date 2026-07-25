use wasm_encoder::CodeSection;

use super::file_exists::emit_file_exists_func;

#[test]
fn file_exists_module_emits_file_exists_function_body() {
    let mut codes = CodeSection::new();

    emit_file_exists_func(&mut codes, 1, 2);

    assert_eq!(codes.len(), 1);
}
