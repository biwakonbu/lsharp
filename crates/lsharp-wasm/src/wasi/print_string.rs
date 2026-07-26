use wasm_encoder::{CodeSection, ValType};

use super::{IOV_ADDR, NWRITTEN_ADDR};

/// __print_string: ヒープ上 String オブジェクトを stdout に出力 (改行なし)
/// String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
pub(super) fn emit_print_string_func(codes: &mut CodeSection) {
    use wasm_encoder::Instruction as W;
    use wasm_encoder::MemArg;

    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // local 1: addr (String オブジェクトのアドレス)
        (1, ValType::I32), // local 2: len
    ]);

    // addr = s as i32 (String オブジェクトのアドレス)
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(1));
    // len = i32.load(addr + 4)
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(2));

    // len == 0 なら何もしない
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // iov[0].buf = addr + 8 (bytes の開始位置)
    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Store(mem32(0)));
    // iov[0].len = len
    f.instruction(&W::I32Const(IOV_ADDR + 4));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Store(mem32(0)));
    // fd_write(1, iov, 1, nwritten)
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(NWRITTEN_ADDR));
    f.instruction(&W::Call(0)); // fd_write
    f.instruction(&W::Drop);

    f.instruction(&W::End);
    codes.function(&f);
}
