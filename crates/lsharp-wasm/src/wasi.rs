//! WASI 対応の Wasm コード生成
//!
//! wasmtime で直接実行可能な Wasm バイナリを生成する。
//! print 関数を WASI の fd_write で実装し、_start エントリポイントを生成。

use lsharp_ir::{GcTypeKind, Instruction, Module};
use std::collections::HashMap;
use wasm_encoder::{
    ArrayType, CodeSection, CompositeInnerType, CompositeType, DataSection, ElementSection,
    Elements, EntityType, ExportKind, ExportSection, FieldType, FunctionSection, GlobalSection,
    GlobalType, ImportSection, MemorySection, MemoryType, StorageType, StructType, SubType,
    TableSection, TableType, TypeSection, ValType,
};

use crate::codegen::CodegenError;

/// メモリレイアウト定数
const NEWLINE_ADDR: i32 = 0;
const IOV_ADDR: i32 = 16;
const NWRITTEN_ADDR: i32 = 24;
const BUF_END: i32 = 276;

/// IR 側の内部ヘルパー関数数
const IR_IMPORT_COUNT: u32 = 5;

/// WASI モードで Wasm バイナリを生成
pub fn emit_wasm_wasi(module: &Module) -> Result<Vec<u8>, CodegenError> {
    let mut wasm_module = wasm_encoder::Module::new();

    // 関数インデックス:
    // 0: fd_write (import)
    // 1: __print_i64
    // 2: __alloc
    // 3: __string_concat
    // 4: __string_eq
    // 5: __print_string
    // 6..6+N-1: ユーザー関数
    // 6+N: _start
    let print_helper_idx: u32 = 1;
    let alloc_func_idx: u32 = 2;
    let string_concat_idx: u32 = 3;
    let string_eq_idx: u32 = 4;
    let print_string_idx: u32 = 5;
    let user_func_base: u32 = 6;
    let start_func_idx: u32 = user_func_base + module.functions.len() as u32;

    let gc_type_count = module.gc_types.len() as u32;

    // === Type Section ===
    let mut types = TypeSection::new();

    for gc_type in &module.gc_types {
        match &gc_type.kind {
            GcTypeKind::Struct(fields) => {
                let wasm_fields: Vec<FieldType> = fields
                    .iter()
                    .map(|f| FieldType {
                        element_type: StorageType::Val(crate::emit::ir_to_wasm_valtype(f.ty)),
                        mutable: f.mutable,
                    })
                    .collect();
                types.ty().subtype(&SubType {
                    is_final: true,
                    supertype_idx: None,
                    composite_type: CompositeType {
                        inner: CompositeInnerType::Struct(StructType {
                            fields: wasm_fields.into_boxed_slice(),
                        }),
                        shared: false,
                        descriptor: None,
                        describes: None,
                    },
                });
            }
            GcTypeKind::Array(elem_ty) => {
                types.ty().subtype(&SubType {
                    is_final: true,
                    supertype_idx: None,
                    composite_type: CompositeType {
                        inner: CompositeInnerType::Array(ArrayType(FieldType {
                            element_type: StorageType::Val(crate::emit::ir_to_wasm_valtype(*elem_ty)),
                            mutable: true,
                        })),
                        shared: false,
                        descriptor: None,
                        describes: None,
                    },
                });
            }
        }
    }

    let fd_write_type_idx = types.len();
    types.ty().function(vec![ValType::I32; 4], vec![ValType::I32]);

    let print_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![]);

    let alloc_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    let string_concat_type_idx = types.len();
    types.ty().function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    let string_eq_type_idx = types.len();
    types.ty().function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    let print_string_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![]);

    let mut user_type_indices = Vec::new();
    for func in &module.functions {
        let type_idx = types.len();
        let params: Vec<ValType> = func.params.iter().map(|t| crate::emit::ir_to_wasm_valtype(*t)).collect();
        let results = vec![crate::emit::ir_to_wasm_valtype(func.result)];
        types.ty().function(params, results);
        user_type_indices.push(type_idx);
    }

    let start_type_idx = types.len();
    types.ty().function(vec![], vec![]);

    // CallIndirect 用の型を登録
    // IR の CallIndirect(param_count) に対して (i64 * param_count) -> i64 の型を生成
    let mut call_indirect_type_map: HashMap<u32, u32> = HashMap::new();
    let mut needs_table = false;
    for func in &module.functions {
        for instr in &func.body {
            if let Instruction::CallIndirect(param_count) = instr {
                needs_table = true;
                if !call_indirect_type_map.contains_key(param_count) {
                    let type_idx = types.len();
                    let params = vec![ValType::I64; *param_count as usize];
                    types.ty().function(params, vec![ValType::I64]);
                    call_indirect_type_map.insert(*param_count, type_idx);
                }
            }
        }
    }

    wasm_module.section(&types);

    // === Import Section ===
    let mut imports = ImportSection::new();
    imports.import("wasi_snapshot_preview1", "fd_write", EntityType::Function(fd_write_type_idx));
    wasm_module.section(&imports);

    // === Function Section ===
    let mut functions = FunctionSection::new();
    functions.function(print_type_idx);
    functions.function(alloc_type_idx);
    functions.function(string_concat_type_idx);
    functions.function(string_eq_type_idx);
    functions.function(print_string_type_idx);
    for &type_idx in &user_type_indices {
        functions.function(type_idx);
    }
    functions.function(start_type_idx);
    wasm_module.section(&functions);

    // === Table Section (クロージャ用) ===
    if needs_table {
        let total_funcs = (start_func_idx + 1) as u64; // 全関数数
        let mut tables = TableSection::new();
        tables.table(TableType {
            element_type: wasm_encoder::RefType::FUNCREF,
            minimum: total_funcs,
            maximum: Some(total_funcs),
            table64: false,
            shared: false,
        });
        wasm_module.section(&tables);
    }

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

    // === Global Section ===
    let total_string_data_size: i32 = module.string_data.iter()
        .map(|(_, bytes)| bytes.len() as i32)
        .sum();
    let heap_start = ((512 + total_string_data_size) + 7) & !7;
    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(heap_start),
    );
    wasm_module.section(&globals);

    // === Export Section ===
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("_start", ExportKind::Func, start_func_idx);
    wasm_module.section(&exports);

    // === Element Section (クロージャ用テーブル初期化) ===
    if needs_table {
        let total_funcs = start_func_idx + 1;
        let mut elements = ElementSection::new();
        // テーブル 0 を全関数で初期化
        let func_indices: Vec<u32> = (0..total_funcs).collect();
        elements.active(
            Some(0), // table index
            &wasm_encoder::ConstExpr::i32_const(0), // offset
            Elements::Functions(std::borrow::Cow::Owned(func_indices)),
        );
        wasm_module.section(&elements);
    }

    // === Code Section ===
    let mut codes = CodeSection::new();
    emit_print_i64_func(&mut codes);
    emit_alloc_func(&mut codes);
    emit_string_concat_func(&mut codes, alloc_func_idx);
    emit_string_eq_func(&mut codes);
    emit_print_string_func(&mut codes);

    for func in &module.functions {
        let mut f = wasm_encoder::Function::new(
            func.locals
                .iter()
                .map(|t| (1, crate::emit::ir_to_wasm_valtype(*t)))
                .collect::<Vec<_>>(),
        );
        emit_instructions_wasi(
            &mut f, &func.body,
            print_helper_idx, alloc_func_idx,
            string_concat_idx, string_eq_idx,
            print_string_idx, user_func_base,
            &call_indirect_type_map,
        )?;
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
    let mut str_offset = 512i32;
    for (_label, bytes) in &module.string_data {
        data.active(0, &wasm_encoder::ConstExpr::i32_const(str_offset), bytes.iter().copied());
        str_offset += bytes.len() as i32;
    }
    wasm_module.section(&data);

    Ok(wasm_module.finish())
}

/// __print_i64: i64 の値を10進文字列に変換して stdout に出力
fn emit_print_i64_func(codes: &mut CodeSection) {
    use wasm_encoder::Instruction as W;
    use wasm_encoder::MemArg;

    let mem = |offset: u64| MemArg { offset, align: 0, memory_index: 0 };
    let mem32 = |offset: u64| MemArg { offset, align: 2, memory_index: 0 };

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32),
        (1, ValType::I32),
        (1, ValType::I64),
    ]);

    f.instruction(&W::I32Const(BUF_END));
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::LocalSet(3));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(2));

    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::I64LtS);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::LocalSet(2));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64Sub);
    f.instruction(&W::LocalSet(3));
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I64Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(48));
    f.instruction(&W::I32Store8(mem(0)));
    f.instruction(&W::Else);
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
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I64Const(10));
    f.instruction(&W::I64RemU);
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(48));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Store8(mem(0)));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I64Const(10));
    f.instruction(&W::I64DivU);
    f.instruction(&W::LocalSet(3));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(2));
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(45));
    f.instruction(&W::I32Store8(mem(0)));
    f.instruction(&W::End);

    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::I32Const(IOV_ADDR + 4));
    f.instruction(&W::I32Const(BUF_END));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(NWRITTEN_ADDR));
    f.instruction(&W::Call(0));
    f.instruction(&W::Drop);

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

/// __alloc: Bump Allocator (i64 サイズ) -> i64 アドレス
fn emit_alloc_func(codes: &mut CodeSection) {
    use wasm_encoder::Instruction as W;

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32),
        (1, ValType::I32),
        (1, ValType::I32),
    ]);

    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(7));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(-8));
    f.instruction(&W::I32And);
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::GlobalGet(0));
    f.instruction(&W::LocalSet(2));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(3));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::MemorySize(0));
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32Mul);
    f.instruction(&W::I32GtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::MemorySize(0));
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32Mul);
    f.instruction(&W::I32Sub);
    f.instruction(&W::I32Const(65535));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32DivU);
    f.instruction(&W::MemoryGrow(0));
    f.instruction(&W::Drop);
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::GlobalSet(0));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::End);
    codes.function(&f);
}

/// __string_concat: 2 つのパック文字列を結合
fn emit_string_concat_func(codes: &mut CodeSection, alloc_func_idx: u32) {
    use wasm_encoder::Instruction as W;

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // local 2: off1
        (1, ValType::I32), // local 3: len1
        (1, ValType::I32), // local 4: off2
        (1, ValType::I32), // local 5: len2
        (1, ValType::I32), // local 6: total_len
        (1, ValType::I32), // local 7: new_addr
    ]);

    // off1 = (s1 >> 32) as i32
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64Const(32));
    f.instruction(&W::I64ShrU);
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(2));
    // len1 = s1 as i32
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(3));
    // off2 = (s2 >> 32) as i32
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I64Const(32));
    f.instruction(&W::I64ShrU);
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(4));
    // len2 = s2 as i32
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(5));
    // total_len = len1 + len2
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(6));
    // new_addr = __alloc(total_len)
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(7));
    // memory.copy(new_addr, off1, len1)
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::MemoryCopy { src_mem: 0, dst_mem: 0 });
    // memory.copy(new_addr + len1, off2, len2)
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::MemoryCopy { src_mem: 0, dst_mem: 0 });
    // return pack(new_addr, total_len)
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::I64Const(32));
    f.instruction(&W::I64Shl);
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::I64Or);
    f.instruction(&W::End);
    codes.function(&f);
}

/// __string_eq: 2 つのパック文字列を比較
fn emit_string_eq_func(codes: &mut CodeSection) {
    use wasm_encoder::Instruction as W;

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // local 2: off1
        (1, ValType::I32), // local 3: len1
        (1, ValType::I32), // local 4: off2
        (1, ValType::I32), // local 5: len2
        (1, ValType::I32), // local 6: i
    ]);

    // アンパック
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64Const(32));
    f.instruction(&W::I64ShrU);
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(2));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(3));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I64Const(32));
    f.instruction(&W::I64ShrU);
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32WrapI64);
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

    // バイト比較ループ
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Load8U(wasm_encoder::MemArg { offset: 0, align: 0, memory_index: 0 }));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Load8U(wasm_encoder::MemArg { offset: 0, align: 0, memory_index: 0 }));
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

/// __print_string: パック文字列 (offset<<32|len) を stdout に出力 (改行なし)
fn emit_print_string_func(codes: &mut CodeSection) {
    use wasm_encoder::Instruction as W;
    use wasm_encoder::MemArg;

    let mem32 = |offset: u64| MemArg { offset, align: 2, memory_index: 0 };

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // local 1: offset
        (1, ValType::I32), // local 2: len
    ]);

    // offset = (s >> 32) as i32
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64Const(32));
    f.instruction(&W::I64ShrU);
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(1));
    // len = s as i32
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(2));

    // len == 0 なら何もしない
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // iov[0].buf = offset
    f.instruction(&W::I32Const(IOV_ADDR));
    f.instruction(&W::LocalGet(1));
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

/// IR 命令を WASI 用にリマップして出力
fn emit_instructions_wasi(
    func: &mut wasm_encoder::Function,
    instructions: &[Instruction],
    print_helper_idx: u32,
    alloc_func_idx: u32,
    string_concat_idx: u32,
    string_eq_idx: u32,
    print_string_idx: u32,
    user_func_base: u32,
    call_indirect_type_map: &HashMap<u32, u32>,
) -> Result<(), CodegenError> {
    use wasm_encoder::Instruction as W;

    // CallIndirect の型インデックスと FuncIdx をリマップした命令列を作成
    let remapped: Vec<Instruction> = instructions.iter().map(|instr| {
        match instr {
            Instruction::CallIndirect(param_count) => {
                if let Some(&wasm_type_idx) = call_indirect_type_map.get(param_count) {
                    Instruction::CallIndirect(wasm_type_idx)
                } else {
                    instr.clone()
                }
            }
            Instruction::FuncIdx(ir_idx) => {
                // Call と同じリマップ: IR func_idx → Wasm func_idx
                let wasm_idx = match *ir_idx {
                    0 => print_helper_idx,
                    1 => alloc_func_idx,
                    2 => string_concat_idx,
                    3 => string_eq_idx,
                    4 => print_string_idx,
                    i => user_func_base + (i - IR_IMPORT_COUNT),
                };
                Instruction::FuncIdx(wasm_idx)
            }
            _ => instr.clone(),
        }
    }).collect();

    crate::emit::emit_instructions_common(func, &remapped, |f, i| {
        match i {
            0 => { f.instruction(&W::Call(print_helper_idx)); }
            1 => { f.instruction(&W::Call(alloc_func_idx)); }
            2 => { f.instruction(&W::Call(string_concat_idx)); }
            3 => { f.instruction(&W::Call(string_eq_idx)); }
            4 => { f.instruction(&W::Call(print_string_idx)); }
            _ => { f.instruction(&W::Call(user_func_base + (i - IR_IMPORT_COUNT))); }
        }
        Ok(())
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use lsharp_ir::lower::Lower;
    use lsharp_types::infer::Infer;

    fn compile_wasi(source: &str) -> Vec<u8> {
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lower = Lower::new();
        let module = lower.lower_program(&program, &type_results).unwrap();
        emit_wasm_wasi(&module).unwrap()
    }

    fn run_wasi(wasm_bytes: &[u8]) -> String {
        use wasmtime::*;
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t).unwrap();

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(1024);
        let wasi = WasiCtxBuilder::new().stdout(stdout.clone()).build_p1();

        let mut store = Store::new(&engine, wasi);
        let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();

        let start = instance.get_typed_func::<(), ()>(&mut store, "_start").unwrap();
        start.call(&mut store, ()).unwrap();

        drop(store);
        let bytes = stdout.try_into_inner().unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[test]
    fn test_wasi_print_positive() {
        let wasm = compile_wasi("(defn main [] (print 42))");
        assert_eq!(run_wasi(&wasm), "42\n");
    }

    #[test]
    fn test_wasi_print_zero() {
        let wasm = compile_wasi("(defn main [] (print 0))");
        assert_eq!(run_wasi(&wasm), "0\n");
    }

    #[test]
    fn test_wasi_print_large_number() {
        let wasm = compile_wasi("(defn main [] (print 1234567890))");
        assert_eq!(run_wasi(&wasm), "1234567890\n");
    }

    #[test]
    fn test_wasi_print_one() {
        let wasm = compile_wasi("(defn main [] (print 1))");
        assert_eq!(run_wasi(&wasm), "1\n");
    }

    #[test]
    fn test_wasi_print_arithmetic_result() {
        let wasm = compile_wasi("(defn main [] (print (+ (* 3 4) 5)))");
        assert_eq!(run_wasi(&wasm), "17\n");
    }

    #[test]
    fn test_wasi_multiple_prints() {
        let wasm = compile_wasi("(defn main [] (do (print 1) (print 2) (print 3) 0))");
        assert_eq!(run_wasi(&wasm), "1\n2\n3\n");
    }

    #[test]
    fn test_wasi_print_function_result() {
        let wasm = compile_wasi(
            "(defn double [x] (* x 2))
             (defn main [] (print (double 21)))",
        );
        assert_eq!(run_wasi(&wasm), "42\n");
    }

    #[test]
    fn test_wasi_print_fib() {
        let wasm = compile_wasi(
            "(defn fib [n]
               (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))
             (defn main [] (print (fib 10)))",
        );
        assert_eq!(run_wasi(&wasm), "55\n");
    }

    #[test]
    fn test_wasi_gc_type_section_with_record() {
        let wasm = compile_wasi(
            "(type Point (record (: x Int) (: y Int)))
             (defn main [] (print 42))",
        );
        assert!(wasm.len() > 8);
        assert_eq!(&wasm[0..4], b"\0asm");
        let wasm_no_gc = compile_wasi("(defn main [] (print 42))");
        assert!(wasm.len() > wasm_no_gc.len());
    }

    #[test]
    fn test_wasi_closure_module_validates() {
        // クロージャを含むモジュールが wasmtime で読み込めることを検証
        let source = r#"
            (defn make-inc [] (fn [x] (+ x 1)))
            (defn apply [f x] (f x))
            (defn main [] (print (apply (make-inc) 41)))
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lower = Lower::new();
        let module = lower.lower_program(&program, &type_results).unwrap();
        eprintln!("IR dump:\n{}", module.dump());
        for (i, f) in module.functions.iter().enumerate() {
            eprintln!("func[{}] = {} ({} params, {} locals)", i, f.name, f.params.len(), f.locals.len());
            for (j, instr) in f.body.iter().enumerate() {
                eprintln!("  [{j}] {instr:?}");
            }
        }
        let wasm_bytes = emit_wasm_wasi(&module).unwrap();
        eprintln!("Wasm bytes: {} bytes", wasm_bytes.len());

        // wasmtime でモジュールを読み込めるか
        use wasmtime::Engine;
        let engine = Engine::default();
        match wasmtime::Module::new(&engine, &wasm_bytes) {
            Ok(_) => eprintln!("Module loaded successfully"),
            Err(e) => panic!("Module load error: {e}"),
        }
    }
}
