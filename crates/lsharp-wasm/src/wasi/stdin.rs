use wasm_encoder::{CodeSection, MemArg, ValType};

use super::emit_tagged_pointer_from_i32_local;

/// `__read_stdin`: stdin(fd=0) を 4KiB chunk で EOF まで繰り返し読み、String object を返す。
pub(super) fn emit_read_stdin_func(
    codes: &mut CodeSection,
    alloc_func_idx: u32,
    string_concat_idx: u32,
    fd_read_idx: u32,
) {
    use wasm_encoder::Instruction as W;

    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

    // locals: 0=result_addr(i32), 1=chunk_addr(i32), 2=nread(i32)
    let mut f = wasm_encoder::Function::new(vec![(3, ValType::I32)]);

    // 空文字列を初期値にする
    f.instruction(&W::I64Const(8));
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(0));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(mem32(4)));

    // 読み込み chunk は再利用する
    f.instruction(&W::I64Const(4104));
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(mem32(4)));

    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));

    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::I32Const(352));
    f.instruction(&W::I32Const(4096));
    f.instruction(&W::I32Store(mem32(4)));

    f.instruction(&W::I32Const(360));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(mem32(0)));

    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Const(352));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(360));
    f.instruction(&W::Call(fd_read_idx));
    f.instruction(&W::Drop);

    f.instruction(&W::I32Const(360));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(2));

    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::BrIf(1));

    f.instruction(&W::LocalGet(1));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Store(mem32(4)));

    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(string_concat_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(0));

    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    emit_tagged_pointer_from_i32_local(&mut f, 0);
    f.instruction(&W::End);
    codes.function(&f);
}
