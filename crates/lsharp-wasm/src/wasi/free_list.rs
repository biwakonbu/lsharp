use wasm_encoder::{Function, Instruction as W, MemArg};

use super::{GC_FREE_CLASS_COUNT, GC_FREE_CLASS_LIMITS};

pub(super) fn emit_free_class_index(function: &mut Function, size_local: u32, class_local: u32) {
    // まず oversize class を選び、下限を満たす小さい class で上書きする。
    function.instruction(&W::I32Const(GC_FREE_CLASS_COUNT - 1));
    function.instruction(&W::LocalSet(class_local));
    for (idx, limit) in GC_FREE_CLASS_LIMITS.iter().enumerate().rev() {
        function.instruction(&W::LocalGet(size_local));
        function.instruction(&W::I32Const(*limit));
        function.instruction(&W::I32LeU);
        function.instruction(&W::If(wasm_encoder::BlockType::Empty));
        function.instruction(&W::I32Const(idx as i32));
        function.instruction(&W::LocalSet(class_local));
        function.instruction(&W::End);
    }
}

pub(super) fn emit_free_class_capacity(
    function: &mut Function,
    size_local: u32,
    capacity_local: u32,
) {
    // bump allocation は従来の linear-memory ABI を保ち、要求された aligned size
    // だけを進める。サイズ class は free-list の探索分岐にだけ使い、既存の
    // heap_ptr/telemetry の差分を発生させない。
    function.instruction(&W::LocalGet(size_local));
    function.instruction(&W::LocalSet(capacity_local));
}

pub(super) fn emit_small_free_class_pop(
    function: &mut Function,
    class_local: u32,
    addr_local: u32,
    capacity_local: u32,
    next_local: u32,
    free_class_heads_base_global_idx: u32,
    free_list_count_global_idx: u32,
) {
    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

    // class 7 は oversize fallback の線形探索に残す。
    for idx in 0..(GC_FREE_CLASS_COUNT - 1) {
        function.instruction(&W::LocalGet(class_local));
        function.instruction(&W::I32Const(idx));
        function.instruction(&W::I32Eq);
        function.instruction(&W::If(wasm_encoder::BlockType::Empty));
        function.instruction(&W::GlobalGet(free_class_heads_base_global_idx + idx as u32));
        function.instruction(&W::LocalSet(next_local));
        function.instruction(&W::LocalGet(next_local));
        function.instruction(&W::I32Eqz);
        function.instruction(&W::If(wasm_encoder::BlockType::Empty));
        function.instruction(&W::Else);
        function.instruction(&W::LocalGet(next_local));
        function.instruction(&W::LocalSet(addr_local));
        function.instruction(&W::LocalGet(next_local));
        function.instruction(&W::I32Load(mem32(4)));
        function.instruction(&W::LocalSet(capacity_local));
        function.instruction(&W::LocalGet(next_local));
        function.instruction(&W::I32Load(mem32(0)));
        function.instruction(&W::GlobalSet(free_class_heads_base_global_idx + idx as u32));
        function.instruction(&W::GlobalGet(free_list_count_global_idx));
        function.instruction(&W::I32Const(1));
        function.instruction(&W::I32Sub);
        function.instruction(&W::GlobalSet(free_list_count_global_idx));
        function.instruction(&W::End);
        function.instruction(&W::End);
    }
}

pub(super) fn emit_free_class_push(
    function: &mut Function,
    class_local: u32,
    addr_local: u32,
    capacity_local: u32,
    next_local: u32,
    free_class_heads_base_global_idx: u32,
) {
    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

    for idx in 0..GC_FREE_CLASS_COUNT {
        function.instruction(&W::LocalGet(class_local));
        function.instruction(&W::I32Const(idx));
        function.instruction(&W::I32Eq);
        function.instruction(&W::If(wasm_encoder::BlockType::Empty));
        function.instruction(&W::GlobalGet(free_class_heads_base_global_idx + idx as u32));
        function.instruction(&W::LocalSet(next_local));
        // 既に解放された object の先頭 8 bytes を free-list node として使う。
        function.instruction(&W::LocalGet(addr_local));
        function.instruction(&W::LocalGet(next_local));
        function.instruction(&W::I32Store(mem32(0)));
        function.instruction(&W::LocalGet(addr_local));
        function.instruction(&W::LocalGet(capacity_local));
        function.instruction(&W::I32Store(mem32(4)));
        function.instruction(&W::LocalGet(addr_local));
        function.instruction(&W::GlobalSet(free_class_heads_base_global_idx + idx as u32));
        function.instruction(&W::End);
    }
}
