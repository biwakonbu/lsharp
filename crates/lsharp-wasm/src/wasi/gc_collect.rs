use super::*;

pub(super) fn emit_gc_collect_func(codes: &mut CodeSection, globals: CollectorGlobals) {
    use wasm_encoder::{Instruction as W, MemArg};

    const OLD_COUNT_LOCAL: u32 = 0;
    const READ_IDX_LOCAL: u32 = 1;
    const WRITE_IDX_LOCAL: u32 = 2;
    const ENTRY_PTR_LOCAL: u32 = 3;
    const OBJ_ADDR_LOCAL: u32 = 4;
    const OBJ_SIZE_LOCAL: u32 = 5;
    const MARK_STATE_LOCAL: u32 = 6;
    const ROOT_IDX_LOCAL: u32 = 7;
    const SLOT_ADDR_LOCAL: u32 = 8;
    const FREED_THIS_CYCLE_LOCAL: u32 = 9;
    const MARK_PROGRESS_LOCAL: u32 = 10;
    const CHILD_IDX_LOCAL: u32 = 11;
    const CHILD_LIMIT_LOCAL: u32 = 12;
    const CHILD_ENTRY_ADDR_LOCAL: u32 = 13;
    const TAG_LOCAL: u32 = 14;
    const TEMP_I32_LOCAL: u32 = 15;
    const CANDIDATE_ADDR_LOCAL: u32 = 16;
    const SEARCH_IDX_LOCAL: u32 = 17;
    const SEARCH_ENTRY_PTR_LOCAL: u32 = 18;
    const SLOT_VALUE_LOCAL: u32 = 19;
    const TEMP_I64_LOCAL: u32 = 20;
    const CHILD_VALUE_LOCAL: u32 = 21;
    const FREE_LIST_BASE_LOCAL: u32 = 22;
    const FREE_LIST_CAPACITY_LOCAL: u32 = 23;
    const FREE_LIST_NEW_CAPACITY_LOCAL: u32 = 24;
    const FREE_LIST_BYTES_LOCAL: u32 = 25;
    const MEMORY_END_LOCAL: u32 = 26;
    const NEW_BASE_LOCAL: u32 = 27;
    const NEW_END_LOCAL: u32 = 28;
    const GROW_PAGES_LOCAL: u32 = 29;
    const GROW_RESULT_LOCAL: u32 = 30;
    const OBJ_CAPACITY_LOCAL: u32 = 31;
    const CLASS_INDEX_LOCAL: u32 = 32;
    const NEXT_FREE_LOCAL: u32 = 33;

    let CollectorGlobals {
        heap_ptr_global_idx,
        heap_start_global_idx: _,
        root_stack_top_global_idx,
        root_stack_base_global_idx,
        object_count_global_idx,
        free_list_count_global_idx,
        free_list_base_global_idx,
        free_list_capacity_global_idx,
        object_table_base_global_idx,
        gc_collection_count_global_idx,
        gc_freed_count_global_idx,
        free_class_heads_base_global_idx,
    } = globals;
    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };
    let mem64 = |offset: u64| MemArg {
        offset,
        align: 3,
        memory_index: 0,
    };

    let mut f = wasm_encoder::Function::new(vec![
        (19, ValType::I32),
        (3, ValType::I64),
        (9, ValType::I32),
        (3, ValType::I32),
    ]);

    // mark bit をクリアしてから root stack を seed に fixed-point で trace する。
    f.instruction(&W::GlobalGet(object_count_global_idx));
    f.instruction(&W::LocalSet(OLD_COUNT_LOCAL));

    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(READ_IDX_LOCAL));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::LocalGet(OLD_COUNT_LOCAL));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::GlobalGet(object_table_base_global_idx));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Const(GC_MARK_UNMARKED));
    f.instruction(&W::I32Store(mem32(8)));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(READ_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(ROOT_IDX_LOCAL));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(ROOT_IDX_LOCAL));
    f.instruction(&W::GlobalGet(root_stack_top_global_idx));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::GlobalGet(root_stack_base_global_idx));
    f.instruction(&W::LocalGet(ROOT_IDX_LOCAL));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(SLOT_ADDR_LOCAL));
    f.instruction(&W::LocalGet(SLOT_ADDR_LOCAL));
    f.instruction(&W::I64Load(mem64(0)));
    f.instruction(&W::LocalSet(SLOT_VALUE_LOCAL));
    gc_mark::emit_gc_mark_candidate(
        &mut f,
        globals,
        GcMarkHelperLocals {
            old_count_local: OLD_COUNT_LOCAL,
            candidate_value_local: SLOT_VALUE_LOCAL,
            candidate_addr_local: CANDIDATE_ADDR_LOCAL,
            search_idx_local: SEARCH_IDX_LOCAL,
            search_entry_ptr_local: SEARCH_ENTRY_PTR_LOCAL,
            temp_i64_local: TEMP_I64_LOCAL,
        },
    );
    f.instruction(&W::LocalGet(ROOT_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ROOT_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(MARK_PROGRESS_LOCAL));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(READ_IDX_LOCAL));

    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::LocalGet(OLD_COUNT_LOCAL));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));

    f.instruction(&W::GlobalGet(object_table_base_global_idx));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(8)));
    f.instruction(&W::LocalSet(MARK_STATE_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(OBJ_SIZE_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(12)));
    f.instruction(&W::LocalSet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::LocalGet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_SIZE_LOCAL));
    f.instruction(&W::LocalSet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(MARK_STATE_LOCAL));
    f.instruction(&W::I32Const(GC_MARK_PENDING));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::LocalSet(MARK_PROGRESS_LOCAL));

    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(OBJ_ADDR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(OBJ_SIZE_LOCAL));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(TAG_LOCAL));

    f.instruction(&W::LocalGet(TAG_LOCAL));
    f.instruction(&W::I32Const(HEAP_TAG_REF));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::I64Load(mem64(8)));
    f.instruction(&W::LocalSet(CHILD_VALUE_LOCAL));
    gc_mark::emit_gc_mark_candidate(
        &mut f,
        globals,
        GcMarkHelperLocals {
            old_count_local: OLD_COUNT_LOCAL,
            candidate_value_local: CHILD_VALUE_LOCAL,
            candidate_addr_local: CANDIDATE_ADDR_LOCAL,
            search_idx_local: SEARCH_IDX_LOCAL,
            search_entry_ptr_local: SEARCH_ENTRY_PTR_LOCAL,
            temp_i64_local: TEMP_I64_LOCAL,
        },
    );
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(TAG_LOCAL));
    f.instruction(&W::I32Const(HEAP_TAG_VECTOR));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::I32Load(mem32(8)));
    f.instruction(&W::LocalSet(CHILD_LIMIT_LOCAL));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(CHILD_IDX_LOCAL));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::LocalGet(CHILD_LIMIT_LOCAL));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(16));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::LocalGet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::I64Load(mem64(0)));
    f.instruction(&W::LocalSet(CHILD_VALUE_LOCAL));
    gc_mark::emit_gc_mark_candidate(
        &mut f,
        globals,
        GcMarkHelperLocals {
            old_count_local: OLD_COUNT_LOCAL,
            candidate_value_local: CHILD_VALUE_LOCAL,
            candidate_addr_local: CANDIDATE_ADDR_LOCAL,
            search_idx_local: SEARCH_IDX_LOCAL,
            search_entry_ptr_local: SEARCH_ENTRY_PTR_LOCAL,
            temp_i64_local: TEMP_I64_LOCAL,
        },
    );
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(CHILD_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(TAG_LOCAL));
    f.instruction(&W::I32Const(HEAP_TAG_HASHMAP));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(CHILD_LIMIT_LOCAL));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(CHILD_IDX_LOCAL));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::LocalGet(CHILD_LIMIT_LOCAL));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(16));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::LocalGet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::I64Load(mem64(0)));
    f.instruction(&W::LocalSet(CHILD_VALUE_LOCAL));
    f.instruction(&W::LocalGet(CHILD_VALUE_LOCAL));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::I64Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Else);
    f.instruction(&W::LocalGet(CHILD_VALUE_LOCAL));
    f.instruction(&W::I64Const(-1));
    f.instruction(&W::I64Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Else);
    gc_mark::emit_gc_mark_candidate(
        &mut f,
        globals,
        GcMarkHelperLocals {
            old_count_local: OLD_COUNT_LOCAL,
            candidate_value_local: CHILD_VALUE_LOCAL,
            candidate_addr_local: CANDIDATE_ADDR_LOCAL,
            search_idx_local: SEARCH_IDX_LOCAL,
            search_entry_ptr_local: SEARCH_ENTRY_PTR_LOCAL,
            temp_i64_local: TEMP_I64_LOCAL,
        },
    );
    f.instruction(&W::LocalGet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::I64Load(mem64(8)));
    f.instruction(&W::LocalSet(CHILD_VALUE_LOCAL));
    gc_mark::emit_gc_mark_candidate(
        &mut f,
        globals,
        GcMarkHelperLocals {
            old_count_local: OLD_COUNT_LOCAL,
            candidate_value_local: CHILD_VALUE_LOCAL,
            candidate_addr_local: CANDIDATE_ADDR_LOCAL,
            search_idx_local: SEARCH_IDX_LOCAL,
            search_entry_ptr_local: SEARCH_ENTRY_PTR_LOCAL,
            temp_i64_local: TEMP_I64_LOCAL,
        },
    );
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(CHILD_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(TAG_LOCAL));
    f.instruction(&W::I32Const(HEAP_TAG_CLOSURE));
    f.instruction(&W::I32Eq);
    f.instruction(&W::LocalGet(TAG_LOCAL));
    f.instruction(&W::I32Const(HEAP_TAG_RECORD));
    f.instruction(&W::I32Eq);
    f.instruction(&W::I32Or);
    f.instruction(&W::LocalGet(TAG_LOCAL));
    f.instruction(&W::I32Const(HEAP_TAG_ADT));
    f.instruction(&W::I32Eq);
    f.instruction(&W::I32Or);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_SIZE_LOCAL));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32GtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_SIZE_LOCAL));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Sub);
    f.instruction(&W::LocalSet(TEMP_I32_LOCAL));
    f.instruction(&W::LocalGet(TEMP_I32_LOCAL));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32ShrU);
    f.instruction(&W::LocalSet(CHILD_LIMIT_LOCAL));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(CHILD_IDX_LOCAL));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::LocalGet(CHILD_LIMIT_LOCAL));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::LocalGet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::I64Load(mem64(0)));
    f.instruction(&W::LocalSet(CHILD_VALUE_LOCAL));
    gc_mark::emit_gc_mark_candidate(
        &mut f,
        globals,
        GcMarkHelperLocals {
            old_count_local: OLD_COUNT_LOCAL,
            candidate_value_local: CHILD_VALUE_LOCAL,
            candidate_addr_local: CANDIDATE_ADDR_LOCAL,
            search_idx_local: SEARCH_IDX_LOCAL,
            search_entry_ptr_local: SEARCH_ENTRY_PTR_LOCAL,
            temp_i64_local: TEMP_I64_LOCAL,
        },
    );
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(CHILD_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Const(GC_MARK_SCANNED));
    f.instruction(&W::I32Store(mem32(8)));
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(READ_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(MARK_PROGRESS_LOCAL));
    f.instruction(&W::BrIf(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(WRITE_IDX_LOCAL));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(FREED_THIS_CYCLE_LOCAL));

    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::LocalGet(OLD_COUNT_LOCAL));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));

    f.instruction(&W::GlobalGet(object_table_base_global_idx));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(OBJ_ADDR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(OBJ_SIZE_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(8)));
    f.instruction(&W::LocalSet(MARK_STATE_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(12)));
    f.instruction(&W::LocalSet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::LocalGet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_SIZE_LOCAL));
    f.instruction(&W::LocalSet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(MARK_STATE_LOCAL));
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(object_table_base_global_idx));
    f.instruction(&W::LocalGet(WRITE_IDX_LOCAL));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(OBJ_SIZE_LOCAL));
    f.instruction(&W::I32Store(mem32(4)));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Const(GC_MARK_UNMARKED));
    f.instruction(&W::I32Store(mem32(8)));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::I32Store(mem32(12)));
    f.instruction(&W::LocalGet(WRITE_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(WRITE_IDX_LOCAL));
    f.instruction(&W::Else);
    free_list::emit_free_class_index(&mut f, OBJ_CAPACITY_LOCAL, CLASS_INDEX_LOCAL);
    free_list::emit_free_class_push(
        &mut f,
        CLASS_INDEX_LOCAL,
        OBJ_ADDR_LOCAL,
        OBJ_CAPACITY_LOCAL,
        NEXT_FREE_LOCAL,
        free_class_heads_base_global_idx,
    );
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(free_list_count_global_idx));
    f.instruction(&W::LocalGet(FREED_THIS_CYCLE_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(FREED_THIS_CYCLE_LOCAL));
    // 旧 free-list table の grow/append 経路は後方互換用に残すが、
    // サイズクラス node を登録した後は実行しない。
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Br(0));
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::GlobalGet(free_list_capacity_global_idx));
    f.instruction(&W::I32GeU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(free_list_base_global_idx));
    f.instruction(&W::LocalSet(FREE_LIST_BASE_LOCAL));
    f.instruction(&W::GlobalGet(free_list_capacity_global_idx));
    f.instruction(&W::LocalSet(FREE_LIST_CAPACITY_LOCAL));
    f.instruction(&W::LocalGet(FREE_LIST_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(2));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(FREE_LIST_NEW_CAPACITY_LOCAL));
    f.instruction(&W::LocalGet(FREE_LIST_NEW_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(GC_FREE_LIST_SLOT_BYTES));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(FREE_LIST_BYTES_LOCAL));
    f.instruction(&W::MemorySize(0));
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(MEMORY_END_LOCAL));
    f.instruction(&W::GlobalGet(heap_ptr_global_idx));
    f.instruction(&W::LocalSet(NEW_BASE_LOCAL));
    f.instruction(&W::LocalGet(NEW_BASE_LOCAL));
    f.instruction(&W::LocalGet(MEMORY_END_LOCAL));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(MEMORY_END_LOCAL));
    f.instruction(&W::LocalSet(NEW_BASE_LOCAL));
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(NEW_BASE_LOCAL));
    f.instruction(&W::I32Const(7));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(-8));
    f.instruction(&W::I32And);
    f.instruction(&W::LocalSet(NEW_BASE_LOCAL));
    f.instruction(&W::LocalGet(NEW_BASE_LOCAL));
    f.instruction(&W::LocalGet(FREE_LIST_BYTES_LOCAL));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(NEW_END_LOCAL));
    f.instruction(&W::LocalGet(NEW_END_LOCAL));
    f.instruction(&W::LocalGet(MEMORY_END_LOCAL));
    f.instruction(&W::I32GtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(NEW_END_LOCAL));
    f.instruction(&W::LocalGet(MEMORY_END_LOCAL));
    f.instruction(&W::I32Sub);
    f.instruction(&W::I32Const(65535));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32DivU);
    f.instruction(&W::LocalSet(GROW_PAGES_LOCAL));
    f.instruction(&W::Else);
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(GROW_PAGES_LOCAL));
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(GROW_PAGES_LOCAL));
    f.instruction(&W::MemoryGrow(0));
    f.instruction(&W::LocalSet(GROW_RESULT_LOCAL));
    f.instruction(&W::LocalGet(GROW_RESULT_LOCAL));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Unreachable);
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(NEW_BASE_LOCAL));
    f.instruction(&W::LocalGet(FREE_LIST_BASE_LOCAL));
    f.instruction(&W::LocalGet(FREE_LIST_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(GC_FREE_LIST_SLOT_BYTES));
    f.instruction(&W::I32Mul);
    f.instruction(&W::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&W::LocalGet(NEW_BASE_LOCAL));
    f.instruction(&W::GlobalSet(free_list_base_global_idx));
    f.instruction(&W::LocalGet(FREE_LIST_NEW_CAPACITY_LOCAL));
    f.instruction(&W::GlobalSet(free_list_capacity_global_idx));
    f.instruction(&W::LocalGet(NEW_END_LOCAL));
    f.instruction(&W::GlobalSet(heap_ptr_global_idx));
    f.instruction(&W::End);
    f.instruction(&W::GlobalGet(free_list_base_global_idx));
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(OBJ_SIZE_LOCAL));
    f.instruction(&W::I32Store(mem32(4)));
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(free_list_count_global_idx));
    f.instruction(&W::LocalGet(FREED_THIS_CYCLE_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(FREED_THIS_CYCLE_LOCAL));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(READ_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(WRITE_IDX_LOCAL));
    f.instruction(&W::GlobalSet(object_count_global_idx));
    f.instruction(&W::GlobalGet(gc_collection_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(gc_collection_count_global_idx));
    f.instruction(&W::GlobalGet(gc_freed_count_global_idx));
    f.instruction(&W::LocalGet(FREED_THIS_CYCLE_LOCAL));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(gc_freed_count_global_idx));
    f.instruction(&W::LocalGet(FREED_THIS_CYCLE_LOCAL));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::End);
    codes.function(&f);
}
