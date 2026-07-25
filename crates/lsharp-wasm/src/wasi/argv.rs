use wasm_encoder::{CodeSection, ValType};

use super::emit_tagged_pointer_from_i64_local;

/// `__command_line_args`: コマンドライン引数の数を返す。
pub(super) fn emit_command_line_args_func(codes: &mut CodeSection, args_sizes_get_idx: u32) {
    use wasm_encoder::Instruction as W;

    // locals: なし (スクラッチ領域を使用)
    let mut f = wasm_encoder::Function::new(vec![]);

    // args_sizes_get(argc_ptr=280, argv_buf_size_ptr=284)
    f.instruction(&W::I32Const(280)); // argc ptr
    f.instruction(&W::I32Const(284)); // argv_buf_size ptr
    f.instruction(&W::Call(args_sizes_get_idx));
    f.instruction(&W::Drop); // errno

    // argc を読み取って返す
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::I64ExtendI32U);

    f.instruction(&W::End);
    codes.function(&f);
}

/// `__command_line_arg`: 指定 index のコマンドライン引数を String オブジェクトで返す。
pub(super) fn emit_command_line_arg_func(
    codes: &mut CodeSection,
    alloc_func_idx: u32,
    args_get_idx: u32,
    args_sizes_get_idx: u32,
) {
    use wasm_encoder::Instruction as W;

    // locals:
    // 1=index_i32 2=argc 3=argv_buf_size 4=argv_ptr 5=argv_buf
    // 6=arg_ptr 7=scan_ptr 8=arg_len 9=str_ptr 10=i
    let mut f = wasm_encoder::Function::new(vec![
        (8, ValType::I32),
        (1, ValType::I64),
        (1, ValType::I32),
    ]);

    // index_i32 = i32.wrap_i64(index)
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(1));

    // args_sizes_get(argc_ptr=280, argv_buf_size_ptr=284)
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Const(284));
    f.instruction(&W::Call(args_sizes_get_idx));
    f.instruction(&W::Drop);

    // argc / argv_buf_size
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(2));
    f.instruction(&W::I32Const(284));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(3));

    // index < 0 -> empty string
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32LtS);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I64Const(8));
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::LocalTee(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    emit_tagged_pointer_from_i64_local(&mut f, 9);
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // index >= argc -> empty string
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32GeS);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I64Const(8));
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::LocalTee(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    emit_tagged_pointer_from_i64_local(&mut f, 9);
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // argv_ptr = __alloc(argc * 4)
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Mul);
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(4));

    // argv_buf = __alloc(argv_buf_size)
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(5));

    // args_get(argv_ptr, argv_buf)
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::Call(args_get_idx));
    f.instruction(&W::Drop);

    // arg_ptr = i32.load(argv_ptr + index * 4)
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Mul);
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(6));

    // scan_ptr = arg_ptr, arg_len = 0
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(8));

    // nul 終端まで長さを数える
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Load8U(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(8));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    // str_ptr = __alloc(8 + arg_len)
    f.instruction(&W::I32Const(8));
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::LocalTee(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));

    // i = 0
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(10));

    // bytes を String object に copy
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(10));
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(10));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::LocalGet(10));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Load8U(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    f.instruction(&W::I32Store8(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    f.instruction(&W::LocalGet(10));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(10));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    emit_tagged_pointer_from_i64_local(&mut f, 9);
    f.instruction(&W::End);
    codes.function(&f);
}
