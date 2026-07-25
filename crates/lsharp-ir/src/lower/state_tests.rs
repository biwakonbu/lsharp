use lsharp_syntax::ast::Program;

use super::Lower;

#[test]
fn state_module_exposes_program_state_preparation() {
    let mut lower = Lower::new();
    lower.prepare_program_state(&Program { decls: Vec::new() }, &[]);

    assert_eq!(lower.func_indices.get("print"), Some(&0));
    assert_eq!(lower.import_count, 17);
}
