use wasm_encoder::CodeSection;

use super::string_concat::emit_string_concat_func;

#[test]
fn string_concat_module_emits_string_concat_function_body() {
    let mut codes = CodeSection::new();

    emit_string_concat_func(&mut codes, 1);

    assert_eq!(codes.len(), 1);
}
