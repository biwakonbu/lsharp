use wasm_encoder::{CodeSection, Function, Instruction as W, ValType};

use super::{CollectorGlobals, GcMarkHelperLocals, gc_mark::emit_gc_mark_candidate};

#[test]
fn gc_mark_module_emits_mark_candidate_body() {
    let mut function = Function::new(vec![
        (1, ValType::I32),
        (1, ValType::I64),
        (3, ValType::I32),
        (1, ValType::I64),
    ]);
    emit_gc_mark_candidate(
        &mut function,
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
        GcMarkHelperLocals {
            old_count_local: 0,
            candidate_value_local: 1,
            candidate_addr_local: 2,
            search_idx_local: 3,
            search_entry_ptr_local: 4,
            temp_i64_local: 5,
        },
    );
    function.instruction(&W::End);

    let mut codes = CodeSection::new();
    codes.function(&function);

    assert_eq!(codes.len(), 1);
}
