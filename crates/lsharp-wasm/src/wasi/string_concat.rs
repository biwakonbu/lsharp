use wasm_encoder::{CodeSection, ValType};

use super::emit_tagged_pointer_from_i32_local;

/// __string_concat: 2 つの String オブジェクト (ヒープ上) を結合
/// String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
pub(super) fn emit_string_concat_func(codes: &mut CodeSection, alloc_func_idx: u32) {
    use wasm_encoder::Instruction as W;

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // local 2: addr1
        (1, ValType::I32), // local 3: len1
        (1, ValType::I32), // local 4: addr2
        (1, ValType::I32), // local 5: len2
        (1, ValType::I32), // local 6: total_len
        (1, ValType::I32), // local 7: new_obj (新しい String オブジェクトのアドレス)
    ]);

    // addr1 = s1 as i32 (String オブジェクトのアドレス)
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(2));
    // len1 = i32.load(addr1 + 4)
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(3));
    // addr2 = s2 as i32
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(4));
    // len2 = i32.load(addr2 + 4)
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(5));
    // total_len = len1 + len2
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(6));
    // new_obj = __alloc(8 + total_len) -- tag(4) + len(4) + bytes
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(7));
    // tag = 1
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    // len = total_len
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    // memory.copy(new_obj + 8, addr1 + 8, len1)
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    // memory.copy(new_obj + 8 + len1, addr2 + 8, len2)
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    // return tagged String handle
    emit_tagged_pointer_from_i32_local(&mut f, 7);
    f.instruction(&W::End);
    codes.function(&f);
}
