//! WASI 対応の Wasm コード生成
//!
//! wasmtime で直接実行可能な Wasm バイナリを生成する。
//! print 関数を WASI の fd_write で実装し、_start エントリポイントを生成。

use lsharp_ir::{Instruction, IrType, Module};
use wasm_encoder::{
    CodeSection, DataSection, EntityType, ExportKind, ExportSection, FunctionSection,
    ImportSection, MemorySection, MemoryType, TypeSection, ValType,
};

use crate::codegen::CodegenError;

/// メモリレイアウト定数
const NEWLINE_ADDR: i32 = 0;     // '\n' の格納位置
const IOV_ADDR: i32 = 16;        // iovec 構造体 (8 bytes: base + len)
const NWRITTEN_ADDR: i32 = 24;   // nwritten (4 bytes)
const _BUF_START: i32 = 256;     // 数値変換バッファ開始
const BUF_END: i32 = 276;        // 数値変換バッファ末尾 (21桁分: i64の最大桁数+符号)

/// WASI モードで Wasm バイナリを生成
pub fn emit_wasm_wasi(module: &Module) -> Result<Vec<u8>, CodegenError> {
    let mut wasm_module = wasm_encoder::Module::new();

    // 関数インデックス:
    // 0: fd_write (import)
    // 1: __print_i64 (内部ヘルパー)
    // 2..2+N-1: ユーザー関数
    // 2+N: _start
    let print_helper_idx: u32 = 1;
    let user_func_base: u32 = 2;
    let start_func_idx: u32 = user_func_base + module.functions.len() as u32;

    // === Type Section ===
    let mut types = TypeSection::new();

    // type 0: fd_write (i32, i32, i32, i32) -> i32
    types.ty().function(vec![ValType::I32; 4], vec![ValType::I32]);

    // type 1: __print_i64 (i64) -> ()
    types.ty().function(vec![ValType::I64], vec![]);

    // ユーザー関数の型
    let mut user_type_indices = Vec::new();
    for func in &module.functions {
        let type_idx = types.len();
        let params: Vec<ValType> = func.params.iter().map(|t| ir_to_wasm(*t)).collect();
        let results = vec![ir_to_wasm(func.result)];
        types.ty().function(params, results);
        user_type_indices.push(type_idx);
    }

    // type for _start: () -> ()
    let start_type_idx = types.len();
    types.ty().function(vec![], vec![]);

    wasm_module.section(&types);

    // === Import Section ===
    let mut imports = ImportSection::new();
    imports.import("wasi_snapshot_preview1", "fd_write", EntityType::Function(0));
    wasm_module.section(&imports);

    // === Function Section ===
    let mut functions = FunctionSection::new();
    functions.function(1); // __print_i64: type 1
    for &type_idx in &user_type_indices {
        functions.function(type_idx);
    }
    functions.function(start_type_idx); // _start
    wasm_module.section(&functions);

    // === Memory Section ===
    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    wasm_module.section(&memories);

    // === Export Section ===
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("_start", ExportKind::Func, start_func_idx);
    wasm_module.section(&exports);

    // === Code Section ===
    let mut codes = CodeSection::new();

    // __print_i64
    emit_print_i64_func(&mut codes);

    // ユーザー関数
    for func in &module.functions {
        let mut f = wasm_encoder::Function::new(
            func.locals
                .iter()
                .map(|t| (1, ir_to_wasm(*t)))
                .collect::<Vec<_>>(),
        );
        emit_instructions_wasi(&mut f, &func.body, print_helper_idx, user_func_base)?;
        f.instruction(&wasm_encoder::Instruction::End);
        codes.function(&f);
    }

    // _start
    {
        let mut f = wasm_encoder::Function::new(vec![]);
        if let Some(main_idx) = module.functions.iter().position(|f| f.name == "main") {
            f.instruction(&wasm_encoder::Instruction::Call(user_func_base + main_idx as u32));
            f.instruction(&wasm_encoder::Instruction::Drop);
        }
        f.instruction(&wasm_encoder::Instruction::End);
        codes.function(&f);
    }

    wasm_module.section(&codes);

    // === Data Section ===
    let mut data = DataSection::new();
    data.active(0, &wasm_encoder::ConstExpr::i32_const(NEWLINE_ADDR), b"\n".iter().copied());
    wasm_module.section(&data);

    Ok(wasm_module.finish())
}

/// __print_i64: i64 の値を10進文字列に変換して stdout に出力
///
/// アルゴリズム:
/// 1. BUF_END から左に向かって桁を書く（右詰め）
/// 2. 書き終わったら iov を設定して fd_write
/// 3. 改行を出力
fn emit_print_i64_func(codes: &mut CodeSection) {
    use wasm_encoder::Instruction as W;
    use wasm_encoder::MemArg;

    let mem = |offset: u64| MemArg { offset, align: 0, memory_index: 0 };
    let mem32 = |offset: u64| MemArg { offset, align: 2, memory_index: 0 };

    // param 0: value (i64)
    // local 1: pos (i32) — 現在の書き込み位置
    // local 2: is_neg (i32)
    // local 3: abs_val (i64)
    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // local 1: pos
        (1, ValType::I32), // local 2: is_neg
        (1, ValType::I64), // local 3: abs_val
    ]);

    // pos = BUF_END
    f.instruction(&W::I32Const(BUF_END));
    f.instruction(&W::LocalSet(1));

    // abs_val = value
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::LocalSet(3));

    // is_neg = 0
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(2));

    // if (value < 0) { is_neg = 1; abs_val = -value; }
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::I64LtS);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    {
        f.instruction(&W::I32Const(1));
        f.instruction(&W::LocalSet(2));
        f.instruction(&W::I64Const(0));
        f.instruction(&W::LocalGet(0));
        f.instruction(&W::I64Sub);
        f.instruction(&W::LocalSet(3));
    }
    f.instruction(&W::End);

    // 特殊ケース: value == 0
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I64Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    {
        // pos -= 1
        f.instruction(&W::LocalGet(1));
        f.instruction(&W::I32Const(1));
        f.instruction(&W::I32Sub);
        f.instruction(&W::LocalSet(1));
        // mem[pos] = '0'
        f.instruction(&W::LocalGet(1));
        f.instruction(&W::I32Const(48));
        f.instruction(&W::I32Store8(mem(0)));
    }
    f.instruction(&W::Else);
    {
        // ループ: abs_val > 0 の間、桁を書く
        f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
        f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
        {
            // if (abs_val == 0) break
            f.instruction(&W::LocalGet(3));
            f.instruction(&W::I64Eqz);
            f.instruction(&W::BrIf(1));

            // pos -= 1
            f.instruction(&W::LocalGet(1));
            f.instruction(&W::I32Const(1));
            f.instruction(&W::I32Sub);
            f.instruction(&W::LocalSet(1));

            // digit = (abs_val % 10) + '0'
            // mem[pos] = digit
            f.instruction(&W::LocalGet(1)); // addr for store
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

            f.instruction(&W::Br(0)); // continue loop
        }
        f.instruction(&W::End); // end loop
        f.instruction(&W::End); // end block
    }
    f.instruction(&W::End); // end if-else

    // 負号
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    {
        f.instruction(&W::LocalGet(1));
        f.instruction(&W::I32Const(1));
        f.instruction(&W::I32Sub);
        f.instruction(&W::LocalSet(1));
        f.instruction(&W::LocalGet(1));
        f.instruction(&W::I32Const(45)); // '-'
        f.instruction(&W::I32Store8(mem(0)));
    }
    f.instruction(&W::End);

    // === fd_write: 数値出力 ===
    // iov_base = pos
    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Store(mem32(0)));
    // iov_len = BUF_END - pos
    f.instruction(&W::I32Const(IOV_ADDR + 4));
    f.instruction(&W::I32Const(BUF_END));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::I32Store(mem32(0)));

    // fd_write(1, IOV_ADDR, 1, NWRITTEN_ADDR)
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(NWRITTEN_ADDR));
    f.instruction(&W::Call(0));
    f.instruction(&W::Drop);

    // === fd_write: 改行出力 ===
    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::I32Const(NEWLINE_ADDR));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::I32Const(IOV_ADDR + 4));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(mem32(0)));

    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(NWRITTEN_ADDR));
    f.instruction(&W::Call(0));
    f.instruction(&W::Drop);

    f.instruction(&W::End);
    codes.function(&f);
}

/// IR 命令を WASI 用にリマップして出力
fn emit_instructions_wasi(
    func: &mut wasm_encoder::Function,
    instructions: &[Instruction],
    print_helper_idx: u32,
    user_func_base: u32,
) -> Result<(), CodegenError> {
    use wasm_encoder::Instruction as W;

    for instr in instructions {
        match instr {
            Instruction::Call(i) => {
                if *i == 0 {
                    func.instruction(&W::Call(print_helper_idx));
                } else {
                    func.instruction(&W::Call(user_func_base + (*i - 1)));
                }
            }
            Instruction::I64Const(n) => { func.instruction(&W::I64Const(*n)); }
            Instruction::F64Const(n) => { func.instruction(&W::F64Const(*n)); }
            Instruction::I32Const(n) => { func.instruction(&W::I32Const(*n)); }
            Instruction::LocalGet(i) => { func.instruction(&W::LocalGet(*i)); }
            Instruction::LocalSet(i) => { func.instruction(&W::LocalSet(*i)); }
            Instruction::LocalTee(i) => { func.instruction(&W::LocalTee(*i)); }
            Instruction::I64Add => { func.instruction(&W::I64Add); }
            Instruction::I64Sub => { func.instruction(&W::I64Sub); }
            Instruction::I64Mul => { func.instruction(&W::I64Mul); }
            Instruction::I64Div => { func.instruction(&W::I64DivS); }
            Instruction::I64Rem => { func.instruction(&W::I64RemS); }
            Instruction::F64Add => { func.instruction(&W::F64Add); }
            Instruction::F64Sub => { func.instruction(&W::F64Sub); }
            Instruction::F64Mul => { func.instruction(&W::F64Mul); }
            Instruction::F64Div => { func.instruction(&W::F64Div); }
            Instruction::I64Eq => { func.instruction(&W::I64Eq); }
            Instruction::I64Ne => { func.instruction(&W::I64Ne); }
            Instruction::I64LtS => { func.instruction(&W::I64LtS); }
            Instruction::I64GtS => { func.instruction(&W::I64GtS); }
            Instruction::I64LeS => { func.instruction(&W::I64LeS); }
            Instruction::I64GeS => { func.instruction(&W::I64GeS); }
            Instruction::I32Eqz => { func.instruction(&W::I32Eqz); }
            Instruction::I32And => { func.instruction(&W::I32And); }
            Instruction::I32Or => { func.instruction(&W::I32Or); }
            Instruction::I64ExtendI32S => { func.instruction(&W::I64ExtendI32S); }
            Instruction::I32WrapI64 => { func.instruction(&W::I32WrapI64); }
            Instruction::If(ty) => {
                func.instruction(&W::If(wasm_encoder::BlockType::Result(ir_to_wasm(*ty))));
            }
            Instruction::Else => { func.instruction(&W::Else); }
            Instruction::End => { func.instruction(&W::End); }
            Instruction::Block(ty) => {
                func.instruction(&W::Block(wasm_encoder::BlockType::Result(ir_to_wasm(*ty))));
            }
            Instruction::Loop(ty) => {
                func.instruction(&W::Loop(wasm_encoder::BlockType::Result(ir_to_wasm(*ty))));
            }
            Instruction::Br(i) => { func.instruction(&W::Br(*i)); }
            Instruction::BrIf(i) => { func.instruction(&W::BrIf(*i)); }
            Instruction::Return => { func.instruction(&W::Return); }
            Instruction::Unreachable => { func.instruction(&W::Unreachable); }
            Instruction::CallImport(i) => { func.instruction(&W::Call(*i)); }
            Instruction::Drop => { func.instruction(&W::Drop); }
        };
    }

    Ok(())
}

fn ir_to_wasm(ty: IrType) -> ValType {
    match ty {
        IrType::I64 => ValType::I64,
        IrType::F64 => ValType::F64,
        IrType::I32 => ValType::I32,
    }
}
