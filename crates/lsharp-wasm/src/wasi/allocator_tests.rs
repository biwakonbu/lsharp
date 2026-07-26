use wasm_encoder::CodeSection;

use super::{AllocatorGlobals, allocator::emit_alloc_func};

#[test]
fn allocator_module_emits_function_body() {
    let mut codes = CodeSection::new();
    emit_alloc_func(
        &mut codes,
        AllocatorGlobals {
            heap_ptr_global_idx: 0,
            alloc_count_global_idx: 1,
            object_count_global_idx: 2,
            free_list_count_global_idx: 3,
            free_list_base_global_idx: 4,
            object_table_base_global_idx: 5,
            object_table_capacity_global_idx: 6,
            free_class_heads_base_global_idx: 7,
            free_list_scan_steps_global_idx: 8,
        },
    );

    assert_eq!(codes.len(), 1);
}
