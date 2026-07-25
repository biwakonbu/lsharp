use wasm_encoder::CodeSection;

use super::hash::emit_fnv1a_hash_func;

#[test]
fn hash_module_emits_fnv1a_function_body() {
    let mut codes = CodeSection::new();

    emit_fnv1a_hash_func(&mut codes);

    assert_eq!(codes.len(), 1);
}
