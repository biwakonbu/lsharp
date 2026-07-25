use wasm_encoder::{CodeSection, ValType};

/// `__fnv1a_hash`: String オブジェクト (ヒープ上) の FNV-1a ハッシュ値を計算する。
/// String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
/// パラメータ: local 0 = str_obj (i64: String オブジェクトのアドレス)
/// 戻り値: ハッシュ値 (i64)、0 と -1 を避けるため +2 する
pub(super) fn emit_fnv1a_hash_func(codes: &mut CodeSection) {
    use wasm_encoder::Instruction as W;

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // local 1: offset (bytes の開始アドレス = addr + 8)
        (1, ValType::I32), // local 2: len (文字列の長さ)
        (1, ValType::I32), // local 3: i (ループカウンタ)
        (1, ValType::I32), // local 4: hash (ハッシュ値、i32 で計算)
        (1, ValType::I32), // local 5: byte (読み込んだバイト)
    ]);

    // offset = addr + 8 (bytes の開始位置)
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(1));

    // len = i32.load(addr + 4)
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(2));

    // i = 0
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(3));

    // hash = 2166136261 (FNV offset basis)
    f.instruction(&W::I32Const(2166136261u32 as i32));
    f.instruction(&W::LocalSet(4));

    // ループ: i < len の間
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));

    // if i >= len → break
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));

    // byte = mem[offset + i]
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Load8U(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(5));

    // hash = hash XOR byte
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Xor);
    f.instruction(&W::LocalSet(4));

    // hash = hash * 16777619 (FNV prime)
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(16777619));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(4));

    // i++
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(3));

    // continue loop
    f.instruction(&W::Br(0));
    f.instruction(&W::End); // end loop
    f.instruction(&W::End); // end block

    // ハッシュ値を 0 と -1 (トゥームストーン) と衝突しないように調整
    // hash == 0 || hash == -1 の場合は hash = hash + 2
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Eq);
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::I32Eq);
    f.instruction(&W::I32Or);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(2));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::End);

    // i64 に拡張して返す
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I64ExtendI32U);

    f.instruction(&W::End);
    codes.function(&f);
}
