use wasm_encoder::CodeSection;

use super::argv::{emit_command_line_arg_func, emit_command_line_args_func};

#[test]
fn argv_module_emits_command_line_function_bodies() {
    let mut codes = CodeSection::new();

    emit_command_line_args_func(&mut codes, 3);
    emit_command_line_arg_func(&mut codes, 10, 2, 3);

    assert_eq!(codes.len(), 2);
}
