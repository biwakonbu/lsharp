use std::fmt;

/// IR の型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IrType {
    I64,
    F64,
    I32,
    /// GC 参照型 (WasmGC struct/array への参照)
    Ref(u32),
    /// 関数参照型 (funcref)
    FuncRef,
    /// 特定の関数型を指す non-nullable ではない concrete funcref。
    ///
    /// 値は IR の型セクション上の関数型インデックスを保持する。WasmGC
    /// emitter は synthetic import の有無に応じて最終的な型インデックスへ
    /// rebasing する。
    TypedFuncRef(u32),
}

impl fmt::Display for IrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrType::I64 => write!(f, "i64"),
            IrType::F64 => write!(f, "f64"),
            IrType::I32 => write!(f, "i32"),
            IrType::Ref(idx) => write!(f, "ref({idx})"),
            IrType::FuncRef => write!(f, "funcref"),
            IrType::TypedFuncRef(idx) => write!(f, "funcref({idx})"),
        }
    }
}

/// IR 命令
#[derive(Debug, Clone)]
pub enum Instruction {
    // 定数
    I64Const(i64),
    F64Const(f64),
    I32Const(i32),

    // ローカル変数操作
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),

    // 整数演算
    I64Add,
    I64Sub,
    I64Mul,
    I64Div,
    I64Rem,

    // 浮動小数点演算
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,

    // 比較演算 (結果は i32)
    I64Eq,
    I64Ne,
    I64LtS,
    I64GtS,
    I64LeS,
    I64GeS,

    // 論理演算 (i32)
    I32Eqz,
    I32And,
    I32Or,

    // 型変換
    I64ExtendI32S,
    I32WrapI64,

    // 制御フロー
    Call(u32),  // 関数インデックス
    If(IrType), // if-then-else 開始（結果型付き）
    Else,
    End,
    Block(IrType), // ブロック開始 (結果型あり)
    Loop(IrType),  // ループ開始 (結果型あり)
    BlockEmpty,    // ブロック開始 (結果型なし)
    LoopEmpty,     // ループ開始 (結果型なし)
    IfEmpty,       // if-then-else 開始 (結果型なし)
    Br(u32),       // 分岐
    BrIf(u32),     // 条件分岐
    Return,
    Unreachable,

    // ホスト関数呼び出し
    CallImport(u32), // import された関数のインデックス

    // バックエンド専用ランタイム命令
    // 既存の import index を消費せず、Vector の下位 8 bit を raw bytes として書き込む。
    WriteFileBytes,

    // スタック操作
    Drop,

    // GC 命令 (WasmGC)
    StructNew(u32),          // struct.new type_idx
    StructGet(u32, u32),     // struct.get type_idx field_idx
    StructSet(u32, u32),     // struct.set type_idx field_idx
    RefCast(u32),            // ref.cast type_idx (ダウンキャスト)
    RefNull(u32),            // ref.null concrete type_idx
    ArrayNewFixed(u32, u32), // array.new_fixed type_idx length
    ArrayNewDefault(u32),    // array.new_default type_idx (dynamic length)
    ArrayGet(u32),           // array.get type_idx
    ArraySet(u32),           // array.set type_idx
    ArrayLen(u32),           // array.len type_idx (validation metadata)

    // 関数参照 (vtable/辞書パスイング)
    RefFunc(u32), // ref.func func_idx
    CallRef(u32), // call_ref type_idx (funcref 経由の間接呼び出し)

    // グローバル変数
    GlobalGet(u32), // global.get idx
    GlobalSet(u32), // global.set idx

    // メモリ操作
    I32Load {
        offset: u32,
    },
    I32Store {
        offset: u32,
    },
    I32Load8U {
        offset: u32,
    },
    I32Store8 {
        offset: u32,
    },
    I64Load {
        offset: u32,
    },
    I64Store {
        offset: u32,
    },

    // 型変換（符号なし拡張）
    I64ExtendI32U,

    // i32 算術演算
    I32Add,
    I32Sub,
    I32Mul,

    // i32 比較（符号なし）
    I32GtU,
    I32GeU,

    // ビット操作
    I32Shl,
    I32ShrU,
    I64Shl,
    I64ShrU,
    I64And,
    I64Or,
    I64Xor,

    // メモリ管理
    MemoryGrow,
    MemorySize,
    MemoryCopy,
    MemoryFill,

    // 間接呼び出し (クロージャ用)
    /// call_indirect: テーブルインデックスと型インデックスで間接呼び出し
    /// type_idx はリフト関数の型インデックスを指す
    CallIndirect(u32),

    /// 関数インデックスを i32 値としてスタックに積む
    /// Call(idx) と同じインデックス空間。codegen でリマップされる。
    FuncIdx(u32),

    /// 文字列定数: string_data のインデックスを指す
    /// codegen でヒープ上に String オブジェクト (tag=1, len, bytes) を確保し、アドレスを返す
    StringConst(u32),
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::I64Const(n) => write!(f, "i64.const {n}"),
            Instruction::F64Const(n) => write!(f, "f64.const {n}"),
            Instruction::I32Const(n) => write!(f, "i32.const {n}"),
            Instruction::LocalGet(i) => write!(f, "local.get {i}"),
            Instruction::LocalSet(i) => write!(f, "local.set {i}"),
            Instruction::LocalTee(i) => write!(f, "local.tee {i}"),
            Instruction::I64Add => write!(f, "i64.add"),
            Instruction::I64Sub => write!(f, "i64.sub"),
            Instruction::I64Mul => write!(f, "i64.mul"),
            Instruction::I64Div => write!(f, "i64.div_s"),
            Instruction::I64Rem => write!(f, "i64.rem_s"),
            Instruction::F64Add => write!(f, "f64.add"),
            Instruction::F64Sub => write!(f, "f64.sub"),
            Instruction::F64Mul => write!(f, "f64.mul"),
            Instruction::F64Div => write!(f, "f64.div"),
            Instruction::I64Eq => write!(f, "i64.eq"),
            Instruction::I64Ne => write!(f, "i64.ne"),
            Instruction::I64LtS => write!(f, "i64.lt_s"),
            Instruction::I64GtS => write!(f, "i64.gt_s"),
            Instruction::I64LeS => write!(f, "i64.le_s"),
            Instruction::I64GeS => write!(f, "i64.ge_s"),
            Instruction::I32Eqz => write!(f, "i32.eqz"),
            Instruction::I32And => write!(f, "i32.and"),
            Instruction::I32Or => write!(f, "i32.or"),
            Instruction::I64ExtendI32S => write!(f, "i64.extend_i32_s"),
            Instruction::I32WrapI64 => write!(f, "i32.wrap_i64"),
            Instruction::Call(i) => write!(f, "call {i}"),
            Instruction::If(ty) => write!(f, "if ({ty})"),
            Instruction::Else => write!(f, "else"),
            Instruction::End => write!(f, "end"),
            Instruction::Block(ty) => write!(f, "block ({ty})"),
            Instruction::Loop(ty) => write!(f, "loop ({ty})"),
            Instruction::BlockEmpty => write!(f, "block"),
            Instruction::LoopEmpty => write!(f, "loop"),
            Instruction::IfEmpty => write!(f, "if"),
            Instruction::Br(i) => write!(f, "br {i}"),
            Instruction::BrIf(i) => write!(f, "br_if {i}"),
            Instruction::Return => write!(f, "return"),
            Instruction::Unreachable => write!(f, "unreachable"),
            Instruction::CallImport(i) => write!(f, "call_import {i}"),
            Instruction::WriteFileBytes => write!(f, "write_file_bytes"),
            Instruction::Drop => write!(f, "drop"),
            Instruction::StructNew(idx) => write!(f, "struct.new {idx}"),
            Instruction::StructGet(type_idx, field_idx) => {
                write!(f, "struct.get {type_idx} {field_idx}")
            }
            Instruction::StructSet(type_idx, field_idx) => {
                write!(f, "struct.set {type_idx} {field_idx}")
            }
            Instruction::RefCast(idx) => write!(f, "ref.cast {idx}"),
            Instruction::RefNull(idx) => write!(f, "ref.null {idx}"),
            Instruction::ArrayNewFixed(type_idx, length) => {
                write!(f, "array.new_fixed {type_idx} {length}")
            }
            Instruction::ArrayNewDefault(type_idx) => write!(f, "array.new_default {type_idx}"),
            Instruction::ArrayGet(type_idx) => write!(f, "array.get {type_idx}"),
            Instruction::ArraySet(type_idx) => write!(f, "array.set {type_idx}"),
            Instruction::ArrayLen(type_idx) => write!(f, "array.len {type_idx}"),
            Instruction::RefFunc(idx) => write!(f, "ref.func {idx}"),
            Instruction::CallRef(idx) => write!(f, "call_ref {idx}"),
            Instruction::GlobalGet(idx) => write!(f, "global.get {idx}"),
            Instruction::GlobalSet(idx) => write!(f, "global.set {idx}"),
            // メモリ操作
            Instruction::I32Load { offset } => write!(f, "i32.load offset={offset}"),
            Instruction::I32Store { offset } => write!(f, "i32.store offset={offset}"),
            Instruction::I32Load8U { offset } => write!(f, "i32.load8_u offset={offset}"),
            Instruction::I32Store8 { offset } => write!(f, "i32.store8 offset={offset}"),
            Instruction::I64Load { offset } => write!(f, "i64.load offset={offset}"),
            Instruction::I64Store { offset } => write!(f, "i64.store offset={offset}"),
            // 型変換
            Instruction::I64ExtendI32U => write!(f, "i64.extend_i32_u"),
            // i32 算術演算
            Instruction::I32Add => write!(f, "i32.add"),
            Instruction::I32Sub => write!(f, "i32.sub"),
            Instruction::I32Mul => write!(f, "i32.mul"),
            // i32 比較
            Instruction::I32GtU => write!(f, "i32.gt_u"),
            Instruction::I32GeU => write!(f, "i32.ge_u"),
            // ビット操作
            Instruction::I32Shl => write!(f, "i32.shl"),
            Instruction::I32ShrU => write!(f, "i32.shr_u"),
            Instruction::I64Shl => write!(f, "i64.shl"),
            Instruction::I64ShrU => write!(f, "i64.shr_u"),
            Instruction::I64And => write!(f, "i64.and"),
            Instruction::I64Or => write!(f, "i64.or"),
            Instruction::I64Xor => write!(f, "i64.xor"),
            // メモリ管理
            Instruction::MemoryGrow => write!(f, "memory.grow"),
            Instruction::MemorySize => write!(f, "memory.size"),
            Instruction::MemoryCopy => write!(f, "memory.copy"),
            Instruction::MemoryFill => write!(f, "memory.fill"),
            Instruction::CallIndirect(type_idx) => write!(f, "call_indirect {type_idx}"),
            Instruction::FuncIdx(idx) => write!(f, "func_idx {idx}"),
            Instruction::StringConst(idx) => write!(f, "string_const {idx}"),
        }
    }
}
