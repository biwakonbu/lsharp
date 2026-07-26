use wasm_encoder::CodeSection;

use super::{CollectorGlobals, gc_collect::emit_gc_collect_func};

#[test]
fn gc_collect_module_emits_function_body() {
    let mut codes = CodeSection::new();
    emit_gc_collect_func(
        &mut codes,
        CollectorGlobals {
            heap_ptr_global_idx: 0,
            heap_start_global_idx: 1,
            root_stack_top_global_idx: 2,
            root_stack_base_global_idx: 3,
            object_count_global_idx: 4,
            free_list_count_global_idx: 5,
            free_list_base_global_idx: 6,
            free_list_capacity_global_idx: 7,
            object_table_base_global_idx: 8,
            gc_collection_count_global_idx: 9,
            gc_freed_count_global_idx: 10,
            free_class_heads_base_global_idx: 11,
        },
    );

    assert_eq!(codes.len(), 1);
}
