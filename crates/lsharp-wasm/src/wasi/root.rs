use wasm_encoder::{CodeSection, ValType};

pub(super) fn emit_root_push_func(
    codes: &mut CodeSection,
    heap_ptr_global_idx: u32,
    root_stack_top_global_idx: u32,
    root_stack_base_global_idx: u32,
    root_stack_capacity_global_idx: u32,
) {
    use wasm_encoder::{Instruction as W, MemArg};

    const TOP_LOCAL: u32 = 1;
    const OLD_BASE_LOCAL: u32 = 2;
    const OLD_CAPACITY_LOCAL: u32 = 3;
    const NEW_CAPACITY_LOCAL: u32 = 4;
    const ROOT_TABLE_BYTES_LOCAL: u32 = 5;
    const MEMORY_END_LOCAL: u32 = 6;
    const NEW_BASE_LOCAL: u32 = 7;
    const NEW_END_LOCAL: u32 = 8;
    const GROW_PAGES_LOCAL: u32 = 9;
    const GROW_RESULT_LOCAL: u32 = 10;
    const SLOT_ADDR_LOCAL: u32 = 11;

    let mem64 = |offset: u64| MemArg {
        offset,
        align: 3,
        memory_index: 0,
    };

    let mut f = wasm_encoder::Function::new(vec![(11, ValType::I32)]);
    f.instruction(&W::GlobalGet(root_stack_top_global_idx));
    f.instruction(&W::LocalSet(TOP_LOCAL));
    f.instruction(&W::LocalGet(TOP_LOCAL));
    f.instruction(&W::GlobalGet(root_stack_capacity_global_idx));
    f.instruction(&W::I32GeU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(root_stack_base_global_idx));
    f.instruction(&W::LocalSet(OLD_BASE_LOCAL));
    f.instruction(&W::GlobalGet(root_stack_capacity_global_idx));
    f.instruction(&W::LocalSet(OLD_CAPACITY_LOCAL));
    f.instruction(&W::LocalGet(OLD_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(2));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(NEW_CAPACITY_LOCAL));
    f.instruction(&W::LocalGet(NEW_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(ROOT_TABLE_BYTES_LOCAL));
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
    f.instruction(&W::LocalGet(ROOT_TABLE_BYTES_LOCAL));
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
    f.instruction(&W::LocalGet(OLD_BASE_LOCAL));
    f.instruction(&W::LocalGet(OLD_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Mul);
    f.instruction(&W::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&W::LocalGet(NEW_BASE_LOCAL));
    f.instruction(&W::GlobalSet(root_stack_base_global_idx));
    f.instruction(&W::LocalGet(NEW_CAPACITY_LOCAL));
    f.instruction(&W::GlobalSet(root_stack_capacity_global_idx));
    f.instruction(&W::LocalGet(NEW_END_LOCAL));
    f.instruction(&W::GlobalSet(heap_ptr_global_idx));
    f.instruction(&W::End);
    f.instruction(&W::GlobalGet(root_stack_base_global_idx));
    f.instruction(&W::LocalGet(TOP_LOCAL));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(SLOT_ADDR_LOCAL));
    f.instruction(&W::LocalGet(SLOT_ADDR_LOCAL));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64Store(mem64(0)));
    f.instruction(&W::LocalGet(TOP_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(root_stack_top_global_idx));
    f.instruction(&W::LocalGet(TOP_LOCAL));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::End);
    codes.function(&f);
}

pub(super) fn emit_root_pop_func(
    codes: &mut CodeSection,
    root_stack_top_global_idx: u32,
    root_stack_base_global_idx: u32,
) {
    use wasm_encoder::{Instruction as W, MemArg};

    let mem64 = |offset: u64| MemArg {
        offset,
        align: 3,
        memory_index: 0,
    };

    let mut f = wasm_encoder::Function::new(vec![(1, ValType::I32), (1, ValType::I32)]);
    f.instruction(&W::GlobalGet(root_stack_top_global_idx));
    f.instruction(&W::LocalSet(0));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Result(ValType::I64)));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::Else);
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::LocalSet(0));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::GlobalSet(root_stack_top_global_idx));
    f.instruction(&W::GlobalGet(root_stack_base_global_idx));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I64Load(mem64(0)));
    f.instruction(&W::End);
    f.instruction(&W::End);
    codes.function(&f);
}

pub(super) fn emit_root_set_func(
    codes: &mut CodeSection,
    root_stack_top_global_idx: u32,
    root_stack_base_global_idx: u32,
    failure_slot_global_idx: u32,
    failure_top_global_idx: u32,
    failure_count_global_idx: u32,
) {
    use wasm_encoder::{Instruction as W, MemArg};

    let mem64 = |offset: u64| MemArg {
        offset,
        align: 3,
        memory_index: 0,
    };

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32),
        (1, ValType::I32),
        (1, ValType::I32),
    ]);
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(2));
    f.instruction(&W::GlobalGet(root_stack_top_global_idx));
    f.instruction(&W::LocalSet(3));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::GlobalSet(failure_slot_global_idx));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::GlobalSet(failure_top_global_idx));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32GeU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(failure_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(failure_count_global_idx));
    f.instruction(&W::Unreachable);
    f.instruction(&W::End);
    f.instruction(&W::GlobalGet(root_stack_base_global_idx));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I64Store(mem64(0)));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::End);
    codes.function(&f);
}
