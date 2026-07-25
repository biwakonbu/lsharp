// === ヒープオブジェクトタグ定数 ===

/// ヒープオブジェクトタグ: 文字列
pub const HEAP_TAG_STRING: i32 = 1;
/// ヒープオブジェクトタグ: レコード
pub const HEAP_TAG_RECORD: i32 = 2;
/// ヒープオブジェクトタグ: ADT
pub const HEAP_TAG_ADT: i32 = 3;
/// ヒープオブジェクトタグ: クロージャ
pub const HEAP_TAG_CLOSURE: i32 = 4;
/// ヒープオブジェクトタグ: ベクタ
pub const HEAP_TAG_VECTOR: i32 = 5;
/// ヒープオブジェクトタグ: ハッシュマップ
pub const HEAP_TAG_HASHMAP: i32 = 6;
/// ヒープオブジェクトタグ: Ref (可変参照)
pub const HEAP_TAG_REF: i32 = 7;

/// i32 アドレスをタグ付きポインタ (i64) に変換する IR 命令列を生成
/// 最上位ビットを 1 にセット: addr | (1 << 63)
/// スタック: [addr: i32] -> [tagged_ptr: i64]
pub(crate) fn emit_tag_pointer(body: &mut Vec<crate::Instruction>, _addr_local: u32) {
    use crate::Instruction;
    // スタックトップの i32 アドレスを i64 に拡張してタグ付け
    body.push(Instruction::I64ExtendI32U);
    body.push(Instruction::I64Const(1i64 << 63));
    body.push(Instruction::I64Add); // OR の代わりに ADD (最上位ビットが 0 なので等価)
}

/// タグ付きポインタ (i64) から i32 アドレスを取り出す IR 命令列を生成
/// 下位 32 ビットを取得: ptr as i32
/// スタック: [tagged_ptr: i64] -> [addr: i32]
pub(crate) fn emit_untag_pointer(body: &mut Vec<crate::Instruction>) {
    use crate::Instruction;
    body.push(Instruction::I32WrapI64);
}

/// ヒープオブジェクトヘッダ [tag: i32, size: i32] を書き込む IR 命令列を生成
/// スタック: [addr: i32] -> [] (アドレスは消費される、呼び出し側で保存が必要)
/// addr+0 に tag、addr+4 に size を書き込む
#[allow(dead_code)]
pub(crate) fn emit_write_heap_header(body: &mut Vec<crate::Instruction>, tag: i32, size: i32) {
    use crate::Instruction;
    // I32Store はスタックから [addr, value] を消費する
    // tag を書き込み: mem[addr+0] = tag
    body.push(Instruction::I32Const(tag));
    body.push(Instruction::I32Store { offset: 0 });
    // size を書き込み: mem[addr+4] = size
    // 注意: addr は I32Store で消費済み。呼び出し側が LocalTee/LocalGet でアドレスを再供給する
    body.push(Instruction::I32Const(size));
    body.push(Instruction::I32Store { offset: 4 });
}
