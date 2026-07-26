use wasm_encoder::{Function, Instruction as W, MemArg};

use super::{CollectorGlobals, GC_MARK_PENDING, GcMarkHelperLocals, TAGGED_POINTER_MASK};

pub(super) fn emit_gc_mark_candidate(
    function: &mut Function,
    globals: CollectorGlobals,
    locals: GcMarkHelperLocals,
) {
    let CollectorGlobals {
        heap_ptr_global_idx,
        heap_start_global_idx,
        ..
    } = globals;
    let object_table_base_global_idx = globals.object_table_base_global_idx;
    let GcMarkHelperLocals {
        old_count_local,
        candidate_value_local,
        candidate_addr_local,
        search_idx_local,
        search_entry_ptr_local,
        temp_i64_local,
    } = locals;

    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

    // raw address または tagged handle からヒープ先頭アドレスを抽出する。
    function.instruction(&W::I32Const(0));
    function.instruction(&W::LocalSet(candidate_addr_local));

    function.instruction(&W::LocalGet(candidate_value_local));
    function.instruction(&W::GlobalGet(heap_start_global_idx));
    function.instruction(&W::I64ExtendI32U);
    function.instruction(&W::I64GeS);
    function.instruction(&W::If(wasm_encoder::BlockType::Empty));
    function.instruction(&W::LocalGet(candidate_value_local));
    function.instruction(&W::GlobalGet(heap_ptr_global_idx));
    function.instruction(&W::I64ExtendI32U);
    function.instruction(&W::I64LtS);
    function.instruction(&W::If(wasm_encoder::BlockType::Empty));
    function.instruction(&W::LocalGet(candidate_value_local));
    function.instruction(&W::I32WrapI64);
    function.instruction(&W::LocalSet(candidate_addr_local));
    function.instruction(&W::End);
    function.instruction(&W::End);

    function.instruction(&W::LocalGet(candidate_addr_local));
    function.instruction(&W::I32Eqz);
    function.instruction(&W::If(wasm_encoder::BlockType::Empty));
    function.instruction(&W::LocalGet(candidate_value_local));
    function.instruction(&W::I64Const(TAGGED_POINTER_MASK));
    function.instruction(&W::I64GeU);
    function.instruction(&W::If(wasm_encoder::BlockType::Empty));
    function.instruction(&W::LocalGet(candidate_value_local));
    function.instruction(&W::I64Const(TAGGED_POINTER_MASK));
    function.instruction(&W::I64Sub);
    function.instruction(&W::LocalSet(temp_i64_local));
    function.instruction(&W::LocalGet(temp_i64_local));
    function.instruction(&W::GlobalGet(heap_start_global_idx));
    function.instruction(&W::I64ExtendI32U);
    function.instruction(&W::I64GeS);
    function.instruction(&W::If(wasm_encoder::BlockType::Empty));
    function.instruction(&W::LocalGet(temp_i64_local));
    function.instruction(&W::GlobalGet(heap_ptr_global_idx));
    function.instruction(&W::I64ExtendI32U);
    function.instruction(&W::I64LtS);
    function.instruction(&W::If(wasm_encoder::BlockType::Empty));
    function.instruction(&W::LocalGet(temp_i64_local));
    function.instruction(&W::I32WrapI64);
    function.instruction(&W::LocalSet(candidate_addr_local));
    function.instruction(&W::End);
    function.instruction(&W::End);
    function.instruction(&W::End);
    function.instruction(&W::End);

    // object table 上の matching entry を探し、未マークなら pending にする。
    function.instruction(&W::LocalGet(candidate_addr_local));
    function.instruction(&W::If(wasm_encoder::BlockType::Empty));
    function.instruction(&W::I32Const(0));
    function.instruction(&W::LocalSet(search_idx_local));
    function.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    function.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    function.instruction(&W::LocalGet(search_idx_local));
    function.instruction(&W::LocalGet(old_count_local));
    function.instruction(&W::I32GeU);
    function.instruction(&W::BrIf(1));

    function.instruction(&W::GlobalGet(object_table_base_global_idx));
    function.instruction(&W::LocalGet(search_idx_local));
    function.instruction(&W::I32Const(4));
    function.instruction(&W::I32Shl);
    function.instruction(&W::I32Add);
    function.instruction(&W::LocalSet(search_entry_ptr_local));

    function.instruction(&W::LocalGet(search_entry_ptr_local));
    function.instruction(&W::I32Load(mem32(0)));
    function.instruction(&W::LocalGet(candidate_addr_local));
    function.instruction(&W::I32Eq);
    function.instruction(&W::If(wasm_encoder::BlockType::Empty));
    function.instruction(&W::LocalGet(search_entry_ptr_local));
    function.instruction(&W::I32Load(mem32(8)));
    function.instruction(&W::I32Eqz);
    function.instruction(&W::If(wasm_encoder::BlockType::Empty));
    function.instruction(&W::LocalGet(search_entry_ptr_local));
    function.instruction(&W::I32Const(GC_MARK_PENDING));
    function.instruction(&W::I32Store(mem32(8)));
    function.instruction(&W::End);
    function.instruction(&W::Br(2));
    function.instruction(&W::End);

    function.instruction(&W::LocalGet(search_idx_local));
    function.instruction(&W::I32Const(1));
    function.instruction(&W::I32Add);
    function.instruction(&W::LocalSet(search_idx_local));
    function.instruction(&W::Br(0));
    function.instruction(&W::End);
    function.instruction(&W::End);
    function.instruction(&W::End);
}
