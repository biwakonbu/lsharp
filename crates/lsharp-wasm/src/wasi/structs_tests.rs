use lsharp_ir::Module;

use super::structs;

#[test]
fn structs_helper_reserves_one_scratch_field_for_empty_modules() {
    let module = Module {
        functions: Vec::new(),
        gc_types: Vec::new(),
        imports: Vec::new(),
        globals: Vec::new(),
        string_data: Vec::new(),
    };

    assert_eq!(structs::max_struct_field_count(&module), 1);
}
