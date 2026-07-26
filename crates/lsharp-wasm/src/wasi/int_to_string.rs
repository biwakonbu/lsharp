use wasm_encoder::{CodeSection, ValType};

use super::{BUF_END, emit_tagged_pointer_from_i64_local};

/// __int_to_string: i64 の値を10進文字列に変換してヒープに格納し、パック文字列を返す
/// __print_i64 と同じ数値→文字列変換ロジックだが、stdout ではなくヒープに書き込む
pub(super) fn emit_int_to_string_func(codes: &mut CodeSection, alloc_func_idx: u32) {
    use wasm_encoder::Instruction as W;
    use wasm_encoder::MemArg;

    let mem = |offset: u64| MemArg {
        offset,
        align: 0,
        memory_index: 0,
    };

    // param 0: n (i64)
    // local 1: buf_end (i32) - スクラッチバッファ末尾
    // local 2: is_neg (i32)
    // local 3: abs_val (i64)
    // local 4: str_len (i32)
    // local 5: new_addr (i64) - __alloc の戻り値
    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // local 1: buf_end
        (1, ValType::I32), // local 2: is_neg
        (1, ValType::I64), // local 3: abs_val
        (1, ValType::I32), // local 4: str_len
        (1, ValType::I64), // local 5: new_addr
    ]);

    // スクラッチバッファとして BUF_END (276) 付近を使用
    // 数値変換は末尾から先頭に向かって書き込む
    f.instruction(&W::I32Const(BUF_END));
    f.instruction(&W::LocalSet(1));

    // is_neg = 0
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(2));

    // abs_val = n
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::LocalSet(3));

    // n < 0 ?
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::I64LtS);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    // is_neg = 1
    f.instruction(&W::I32Const(1));
    f.instruction(&W::LocalSet(2));
    // abs_val = 0 - n
    f.instruction(&W::I64Const(0));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64Sub);
    f.instruction(&W::LocalSet(3));
    f.instruction(&W::End);

    // abs_val == 0 の場合
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I64Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(48)); // '0'
    f.instruction(&W::I32Store8(mem(0)));
    f.instruction(&W::Else);
    // ループ: 各桁を変換
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I64Eqz);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::LocalGet(1));
    // digit = abs_val % 10 + '0'
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I64Const(10));
    f.instruction(&W::I64RemU);
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(48));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Store8(mem(0)));
    // abs_val /= 10
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I64Const(10));
    f.instruction(&W::I64DivU);
    f.instruction(&W::LocalSet(3));
    f.instruction(&W::Br(0));
    f.instruction(&W::End); // end loop
    f.instruction(&W::End); // end block
    f.instruction(&W::End); // end if-else

    // 負符号を追加
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(45)); // '-'
    f.instruction(&W::I32Store8(mem(0)));
    f.instruction(&W::End);

    // str_len = BUF_END - buf_end
    f.instruction(&W::I32Const(BUF_END));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::LocalSet(4));

    // new_obj = __alloc(8 + str_len) -- String オブジェクト [tag=1, len, bytes]
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::LocalSet(5));

    // tag = 1
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    // len
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Store(MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    // memory.copy(new_obj + 8, buf_end, str_len)
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });

    // タグ付き String handle を返す
    emit_tagged_pointer_from_i64_local(&mut f, 5);

    f.instruction(&W::End);
    codes.function(&f);
}
