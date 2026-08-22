use super::*;

/// __alloc: サイズクラス別 free-list と oversize fallback を持つ allocator
pub(super) fn emit_alloc_func(codes: &mut CodeSection, globals: AllocatorGlobals) {
    use wasm_encoder::{Instruction as W, MemArg};

    let AllocatorGlobals {
        heap_ptr_global_idx,
        alloc_count_global_idx,
        object_count_global_idx,
        free_list_count_global_idx,
        object_table_base_global_idx,
        object_table_capacity_global_idx,
        free_class_heads_base_global_idx,
        free_list_scan_steps_global_idx,
    } = globals;
    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

    let mut f = wasm_encoder::Function::new(vec![(24, ValType::I32)]);

    // local1 = aligned size
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(7));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(-8));
    f.instruction(&W::I32And);
    f.instruction(&W::LocalSet(1));
    // free-list node の next/capacity を置ける最小 block を保証する。
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::End);

    // local8 = allocated address (0 means not found in free-list)
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(8));

    // local21 = class, local22 = physical capacity, local24 = linked-list next
    free_list::emit_free_class_index(&mut f, 1, 21);
    free_list::emit_small_free_class_pop(
        &mut f,
        21,
        8,
        22,
        24,
        free_class_heads_base_global_idx,
        free_list_count_global_idx,
    );

    // oversize class は block size を確認する first-fit fallback とする。
    f.instruction(&W::LocalGet(21));
    f.instruction(&W::I32Const(GC_FREE_CLASS_COUNT - 1));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(
        free_class_heads_base_global_idx + (GC_FREE_CLASS_COUNT as u32 - 1),
    ));
    f.instruction(&W::LocalSet(5));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(6));
    // local4 = oversize search hit flag (0 = miss, 1 = reused)
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(22));
    f.instruction(&W::GlobalGet(free_list_scan_steps_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(free_list_scan_steps_global_idx));
    f.instruction(&W::LocalGet(22));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalSet(6));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(5));
    f.instruction(&W::Br(0));
    f.instruction(&W::Else);
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(24));
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(24));
    f.instruction(&W::GlobalSet(
        free_class_heads_base_global_idx + (GC_FREE_CLASS_COUNT as u32 - 1),
    ));
    f.instruction(&W::Else);
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::LocalGet(24));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalSet(8));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::Br(2));
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::GlobalSet(free_list_count_global_idx));
    f.instruction(&W::End);
    f.instruction(&W::End);

    // free-list miss (または oversize miss) だけ class 境界まで予約する。
    // 再利用時は free-list node に保存した実容量をそのまま使う。
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    free_list::emit_free_class_capacity(&mut f, 1, 22);
    f.instruction(&W::End);

    // legacy free-list first-fit search はここにあったが、size-class heads の導入で
    // 無条件 Br(0) により丸ごと到達不能になっていた。到達不能なまま残すと中の誤り
    // (Br(1) であるべき箇所が Br(0) だった) に実行で気付けないので削除した。
    // 判断は ISSUES.md の I-35、guard は allocator_body_has_no_unreachable_block_prologue。

    // free-list miss -> bump allocate
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(heap_ptr_global_idx));
    f.instruction(&W::LocalSet(2));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalGet(22));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(3));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::MemorySize(0));
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32Mul);
    f.instruction(&W::I32GtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::MemorySize(0));
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32Mul);
    f.instruction(&W::I32Sub);
    f.instruction(&W::I32Const(65535));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32DivU);
    f.instruction(&W::MemoryGrow(0));
    f.instruction(&W::LocalSet(20));
    f.instruction(&W::LocalGet(20));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Unreachable);
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::GlobalSet(heap_ptr_global_idx));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalSet(8));
    f.instruction(&W::End);

    // object table が満杯になったら Wasm memory の末尾へ倍増コピーする。
    // metadata を heap payload と同じ固定領域に置かず、既存 object address を動かさない。
    f.instruction(&W::GlobalGet(object_count_global_idx));
    f.instruction(&W::GlobalGet(object_table_capacity_global_idx));
    f.instruction(&W::I32GeU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(object_table_base_global_idx));
    f.instruction(&W::LocalSet(12));
    f.instruction(&W::GlobalGet(object_table_capacity_global_idx));
    f.instruction(&W::LocalSet(13));
    f.instruction(&W::LocalGet(13));
    f.instruction(&W::I32Const(2));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(14));
    f.instruction(&W::LocalGet(14));
    f.instruction(&W::I32Const(GC_OBJECT_SLOT_BYTES));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(17));
    f.instruction(&W::MemorySize(0));
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(15));
    f.instruction(&W::GlobalGet(heap_ptr_global_idx));
    f.instruction(&W::LocalSet(16));
    f.instruction(&W::LocalGet(16));
    f.instruction(&W::LocalGet(15));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(15));
    f.instruction(&W::LocalSet(16));
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(16));
    f.instruction(&W::I32Const(7));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(-8));
    f.instruction(&W::I32And);
    f.instruction(&W::LocalSet(16));
    f.instruction(&W::LocalGet(16));
    f.instruction(&W::LocalGet(17));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(18));
    f.instruction(&W::LocalGet(18));
    f.instruction(&W::LocalGet(15));
    f.instruction(&W::I32GtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(18));
    f.instruction(&W::LocalGet(15));
    f.instruction(&W::I32Sub);
    f.instruction(&W::I32Const(65535));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32DivU);
    f.instruction(&W::LocalSet(19));
    f.instruction(&W::Else);
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(19));
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(19));
    f.instruction(&W::MemoryGrow(0));
    f.instruction(&W::LocalSet(20));
    f.instruction(&W::LocalGet(20));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Unreachable);
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(16));
    f.instruction(&W::LocalGet(12));
    f.instruction(&W::LocalGet(13));
    f.instruction(&W::I32Const(GC_OBJECT_SLOT_BYTES));
    f.instruction(&W::I32Mul);
    f.instruction(&W::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&W::LocalGet(16));
    f.instruction(&W::GlobalSet(object_table_base_global_idx));
    f.instruction(&W::LocalGet(14));
    f.instruction(&W::GlobalSet(object_table_capacity_global_idx));
    f.instruction(&W::LocalGet(18));
    f.instruction(&W::GlobalSet(heap_ptr_global_idx));
    f.instruction(&W::End);

    // live object metadata を記録
    f.instruction(&W::GlobalGet(object_count_global_idx));
    f.instruction(&W::GlobalGet(object_table_capacity_global_idx));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(object_table_base_global_idx));
    f.instruction(&W::GlobalGet(object_count_global_idx));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(5));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Store(mem32(4)));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(GC_MARK_UNMARKED));
    f.instruction(&W::I32Store(mem32(8)));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(22));
    f.instruction(&W::I32Store(mem32(12)));
    f.instruction(&W::GlobalGet(object_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(object_count_global_idx));
    f.instruction(&W::End);

    f.instruction(&W::GlobalGet(alloc_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(alloc_count_global_idx));
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::End);
    codes.function(&f);
}
