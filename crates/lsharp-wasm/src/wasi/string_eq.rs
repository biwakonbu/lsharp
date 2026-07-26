use wasm_encoder::{CodeSection, ValType};

/// __string_eq: 2 つの String オブジェクト (ヒープ上) を比較
/// String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
pub(super) fn emit_string_eq_func(codes: &mut CodeSection) {
    use wasm_encoder::Instruction as W;

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // local 2: addr1
        (1, ValType::I32), // local 3: len1
        (1, ValType::I32), // local 4: addr2
        (1, ValType::I32), // local 5: len2
        (1, ValType::I32), // local 6: i
    ]);

    // addr1 = s1 as i32
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

    // 長さ比較
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // i = 0
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(6));

    // バイト比較ループ (bytes は addr + 8 から)
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    // mem[addr1 + 8 + i]
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Load8U(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    // mem[addr2 + 8 + i]
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Load8U(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::Return);
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(6));
    f.instruction(&W::Br(0));
    f.instruction(&W::End); // end loop
    f.instruction(&W::End); // end block

    f.instruction(&W::I64Const(1));
    f.instruction(&W::End);
    codes.function(&f);
}
