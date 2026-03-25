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

/// IR 側の内部ヘルパー関数数 (print, __alloc, __string_concat, __string_eq, print-string, proc-exit, __int_to_string, read-file, write-file, file-exists?, command-line-args, __fnv1a_hash)
const IR_IMPORT_COUNT: u32 = 12;

/// WASI import 関数数
const WASI_IMPORT_COUNT: u32 = 9;

/// WASI モードで Wasm バイナリを生成
pub fn emit_wasm_wasi(module: &Module) -> Result<Vec<u8>, CodegenError> {
    let mut wasm_module = wasm_encoder::Module::new();

    // 関数インデックス:
    // 0: fd_write (import)
    // 1: proc_exit (import)
    // 2: args_get (import)
    // 3: args_sizes_get (import)
    // 4: fd_read (import)
    // 5: fd_close (import)
    // 6: path_open (import)
    // 7: fd_seek (import)
    // 8: fd_filestat_get (import)
    // 9: __print_i64
    // 10: __alloc
    // 11: __string_concat
    // 12: __string_eq
    // 13: __print_string
    // 14: __int_to_string
    // 15: __read_file
    // 16: __write_file
    // 17: __file_exists
    // 18: __command_line_args
    // 19: __fnv1a_hash
    // 20..20+N-1: ユーザー関数
    // 20+N: _start
    let fd_write_idx: u32 = 0;
    let proc_exit_wasm_idx: u32 = 1;
    let _args_get_idx: u32 = 2;
    let args_sizes_get_idx: u32 = 3;
    let fd_read_idx: u32 = 4;
    let fd_close_idx: u32 = 5;
    let path_open_idx: u32 = 6;
    let _fd_seek_idx: u32 = 7;
    let fd_filestat_get_idx: u32 = 8;
    let print_helper_idx: u32 = WASI_IMPORT_COUNT;
    let alloc_func_idx: u32 = WASI_IMPORT_COUNT + 1;
    let string_concat_idx: u32 = WASI_IMPORT_COUNT + 2;
    let string_eq_idx: u32 = WASI_IMPORT_COUNT + 3;
    let print_string_idx: u32 = WASI_IMPORT_COUNT + 4;
    let int_to_string_idx: u32 = WASI_IMPORT_COUNT + 5;
    let read_file_idx: u32 = WASI_IMPORT_COUNT + 6;
    let write_file_idx: u32 = WASI_IMPORT_COUNT + 7;
    let file_exists_idx: u32 = WASI_IMPORT_COUNT + 8;
    let command_line_args_idx: u32 = WASI_IMPORT_COUNT + 9;
    let fnv1a_hash_idx: u32 = WASI_IMPORT_COUNT + 10;
    let user_func_base: u32 = WASI_IMPORT_COUNT + 11;
    let start_func_idx: u32 = user_func_base + module.functions.len() as u32;

    let _gc_type_count = module.gc_types.len() as u32;

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

    // proc_exit(code: i32) -> ()
    let proc_exit_type_idx = types.len();
    types.ty().function(vec![ValType::I32], vec![]);

    // args_get(argv: i32, argv_buf: i32) -> i32
    let args_get_type_idx = types.len();
    types.ty().function(vec![ValType::I32; 2], vec![ValType::I32]);

    // args_sizes_get(argc: i32, argv_buf_size: i32) -> i32
    let args_sizes_get_type_idx = types.len();
    types.ty().function(vec![ValType::I32; 2], vec![ValType::I32]);

    // fd_read(fd: i32, iovs: i32, iovs_len: i32, nread: i32) -> i32
    let fd_read_type_idx = types.len();
    types.ty().function(vec![ValType::I32; 4], vec![ValType::I32]);

    // fd_close(fd: i32) -> i32
    let fd_close_type_idx = types.len();
    types.ty().function(vec![ValType::I32], vec![ValType::I32]);

    // path_open(dirfd: i32, dirflags: i32, path: i32, path_len: i32,
    //           oflags: i32, fs_rights_base: i64, fs_rights_inheriting: i64,
    //           fdflags: i32, fd: i32) -> i32
    let path_open_type_idx = types.len();
    types.ty().function(
        vec![
            ValType::I32, ValType::I32, ValType::I32, ValType::I32,
            ValType::I32, ValType::I64, ValType::I64,
            ValType::I32, ValType::I32,
        ],
        vec![ValType::I32],
    );

    // fd_seek(fd: i32, offset: i64, whence: i32, newoffset_ptr: i32) -> i32
    let fd_seek_type_idx = types.len();
    types.ty().function(
        vec![ValType::I32, ValType::I64, ValType::I32, ValType::I32],
        vec![ValType::I32],
    );

    // fd_filestat_get(fd: i32, buf_ptr: i32) -> i32
    let fd_filestat_get_type_idx = types.len();
    types.ty().function(vec![ValType::I32, ValType::I32], vec![ValType::I32]);

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

    // __int_to_string: (i64) -> i64 (パック文字列を返す)
    let int_to_string_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    // __read_file: (i64) -> i64 (パック文字列パス → パック文字列内容)
    let read_file_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    // __write_file: (i64, i64) -> i64 (パス, 内容 → 書き込みバイト数)
    let write_file_type_idx = types.len();
    types.ty().function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    // __file_exists: (i64) -> i64 (パス → 0 or 1)
    let file_exists_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    // __command_line_args: () -> i64 (引数の数を返す)
    let command_line_args_type_idx = types.len();
    types.ty().function(vec![], vec![ValType::I64]);

    // __fnv1a_hash: (i64) -> i64 (パック文字列 → FNV-1a ハッシュ値)
    let fnv1a_hash_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

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
    imports.import("wasi_snapshot_preview1", "proc_exit", EntityType::Function(proc_exit_type_idx));
    imports.import("wasi_snapshot_preview1", "args_get", EntityType::Function(args_get_type_idx));
    imports.import("wasi_snapshot_preview1", "args_sizes_get", EntityType::Function(args_sizes_get_type_idx));
    imports.import("wasi_snapshot_preview1", "fd_read", EntityType::Function(fd_read_type_idx));
    imports.import("wasi_snapshot_preview1", "fd_close", EntityType::Function(fd_close_type_idx));
    imports.import("wasi_snapshot_preview1", "path_open", EntityType::Function(path_open_type_idx));
    imports.import("wasi_snapshot_preview1", "fd_seek", EntityType::Function(fd_seek_type_idx));
    imports.import("wasi_snapshot_preview1", "fd_filestat_get", EntityType::Function(fd_filestat_get_type_idx));
    wasm_module.section(&imports);

    // === Function Section ===
    let mut functions = FunctionSection::new();
    functions.function(print_type_idx);
    functions.function(alloc_type_idx);
    functions.function(string_concat_type_idx);
    functions.function(string_eq_type_idx);
    functions.function(print_string_type_idx);
    functions.function(int_to_string_type_idx);
    functions.function(read_file_type_idx);
    functions.function(write_file_type_idx);
    functions.function(file_exists_type_idx);
    functions.function(command_line_args_type_idx);
    functions.function(fnv1a_hash_type_idx);
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
    emit_int_to_string_func(&mut codes, alloc_func_idx);
    emit_read_file_func(&mut codes, alloc_func_idx, path_open_idx, fd_read_idx, fd_close_idx, fd_filestat_get_idx);
    emit_write_file_func(&mut codes, path_open_idx, fd_write_idx, fd_close_idx);
    emit_file_exists_func(&mut codes, path_open_idx, fd_close_idx);
    emit_command_line_args_func(&mut codes, args_sizes_get_idx);
    emit_fnv1a_hash_func(&mut codes);

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
            print_string_idx, proc_exit_wasm_idx,
            int_to_string_idx, read_file_idx,
            write_file_idx, file_exists_idx,
            command_line_args_idx, fnv1a_hash_idx,
            user_func_base,
            &call_indirect_type_map,
        )?;
        f.instruction(&wasm_encoder::Instruction::End);
        codes.function(&f);
    }

    // _start
    {
        let mut f = wasm_encoder::Function::new(vec![]);
        // マルチファイル結合時に各モジュールが (defn main []) を持つため、先頭の main は先頭ファイルのテスト用になる。
        // エントリ Main.ls の main を選ぶため、最後に定義された main を呼ぶ。
        if let Some(main_idx) = module.functions.iter().rposition(|f| f.name == "main") {
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

/// __string_concat: 2 つの String オブジェクト (ヒープ上) を結合
/// String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
fn emit_string_concat_func(codes: &mut CodeSection, alloc_func_idx: u32) {
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
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));
    f.instruction(&W::LocalSet(3));
    // addr2 = s2 as i32
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(4));
    // len2 = i32.load(addr2 + 4)
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));
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
    f.instruction(&W::I32Store(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }));
    // len = total_len
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Store(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));
    // memory.copy(new_obj + 8, addr1 + 8, len1)
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::MemoryCopy { src_mem: 0, dst_mem: 0 });
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
    f.instruction(&W::MemoryCopy { src_mem: 0, dst_mem: 0 });
    // return new_obj as i64
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::End);
    codes.function(&f);
}

/// __string_eq: 2 つの String オブジェクト (ヒープ上) を比較
/// String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
fn emit_string_eq_func(codes: &mut CodeSection) {
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
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));
    f.instruction(&W::LocalSet(3));
    // addr2 = s2 as i32
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(4));
    // len2 = i32.load(addr2 + 4)
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));
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
    f.instruction(&W::I32Load8U(wasm_encoder::MemArg { offset: 0, align: 0, memory_index: 0 }));
    // mem[addr2 + 8 + i]
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
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

/// __print_string: ヒープ上 String オブジェクトを stdout に出力 (改行なし)
/// String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
fn emit_print_string_func(codes: &mut CodeSection) {
    use wasm_encoder::Instruction as W;
    use wasm_encoder::MemArg;

    let mem32 = |offset: u64| MemArg { offset, align: 2, memory_index: 0 };

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
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));
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

/// __int_to_string: i64 の値を10進文字列に変換してヒープに格納し、パック文字列を返す
/// __print_i64 と同じ数値→文字列変換ロジックだが、stdout ではなくヒープに書き込む
fn emit_int_to_string_func(codes: &mut CodeSection, alloc_func_idx: u32) {
    use wasm_encoder::Instruction as W;
    use wasm_encoder::MemArg;

    let mem = |offset: u64| MemArg { offset, align: 0, memory_index: 0 };

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
    f.instruction(&W::I32Store(MemArg { offset: 0, align: 2, memory_index: 0 }));
    // len
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Store(MemArg { offset: 4, align: 2, memory_index: 0 }));
    // memory.copy(new_obj + 8, buf_end, str_len)
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::MemoryCopy { src_mem: 0, dst_mem: 0 });

    // String オブジェクトのアドレスを返す
    f.instruction(&W::LocalGet(5));

    f.instruction(&W::End);
    codes.function(&f);
}

/// __read_file: String オブジェクトパスを受け取り、ファイル内容を String オブジェクトで返す
/// path_open → fd_filestat_get → __alloc → fd_read → fd_close
fn emit_read_file_func(
    codes: &mut CodeSection,
    alloc_func_idx: u32,
    path_open_idx: u32,
    fd_read_idx: u32,
    fd_close_idx: u32,
    fd_filestat_get_idx: u32,
) {
    use wasm_encoder::Instruction as W;

    // locals: 0=path(i64 param), 1=path_offset(i32), 2=path_len(i32), 3=fd(i32),
    //         4=file_size(i32), 5=buf_addr(i32), 6=nread(i32)
    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // 1: path_offset (bytes の開始アドレス = path_addr + 8)
        (1, ValType::I32), // 2: path_len
        (1, ValType::I32), // 3: fd
        (1, ValType::I32), // 4: file_size
        (1, ValType::I32), // 5: buf_addr (String オブジェクトのアドレス)
        (1, ValType::I32), // 6: nread
    ]);

    // String オブジェクトからパス情報を取得
    // path_offset = path_addr + 8 (bytes の開始位置)
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(1)); // path_offset

    // path_len = i32.load(path_addr + 4)
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));
    f.instruction(&W::LocalSet(2)); // path_len

    // fd を格納するスクラッチ領域 (アドレス 280)
    // path_open(dirfd=3, dirflags=0, path, path_len, oflags=0, rights_base, rights_inheriting, fdflags=0, fd_ptr)
    f.instruction(&W::I32Const(3));       // dirfd = 3 (preopened dir)
    f.instruction(&W::I32Const(0));       // dirflags = 0
    f.instruction(&W::LocalGet(1));       // path
    f.instruction(&W::LocalGet(2));       // path_len
    f.instruction(&W::I32Const(0));       // oflags = 0 (read only)
    f.instruction(&W::I64Const(0x42));    // rights_base = fd_read | fd_seek | fd_filestat_get
    f.instruction(&W::I64Const(0));       // rights_inheriting
    f.instruction(&W::I32Const(0));       // fdflags = 0
    f.instruction(&W::I32Const(280));     // fd_ptr (スクラッチ領域)
    f.instruction(&W::Call(path_open_idx));
    f.instruction(&W::Drop);             // errno を無視 (簡略化)

    // fd を読み出し
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }));
    f.instruction(&W::LocalSet(3)); // fd

    // fd_filestat_get でファイルサイズ取得 (stat バッファは 288 から 64 バイト)
    f.instruction(&W::LocalGet(3));       // fd
    f.instruction(&W::I32Const(288));     // stat buf (288..352)
    f.instruction(&W::Call(fd_filestat_get_idx));
    f.instruction(&W::Drop);             // errno

    // file_size = stat[32..40] の下位 32bit (filesize は offset 32 の i64)
    f.instruction(&W::I32Const(288));
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 32, align: 2, memory_index: 0 })); // stat.st_size の下位 32bit
    f.instruction(&W::LocalSet(4)); // file_size

    // String オブジェクト確保: __alloc(8 + file_size)
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(5)); // buf_addr = String オブジェクトのアドレス
    // tag = 1
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }));
    // len = file_size (後で nread に更新)
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Store(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));

    // iov を設定: iov[0].buf = buf_addr + 8, iov[0].len = file_size (スクラッチ 352)
    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Store(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 })); // iov.buf

    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Store(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 })); // iov.len

    // fd_read(fd, iov_ptr=352, iov_count=1, nread_ptr=360)
    f.instruction(&W::LocalGet(3));       // fd
    f.instruction(&W::I32Const(352));     // iovs
    f.instruction(&W::I32Const(1));       // iovs_len
    f.instruction(&W::I32Const(360));     // nread ptr
    f.instruction(&W::Call(fd_read_idx));
    f.instruction(&W::Drop);             // errno

    // nread を読み取り
    f.instruction(&W::I32Const(360));
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }));
    f.instruction(&W::LocalSet(6)); // nread

    // fd_close
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::Drop);

    // String オブジェクトの len を nread に更新
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Store(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));
    // String オブジェクトのアドレスを返す
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I64ExtendI32U);

    f.instruction(&W::End);
    codes.function(&f);
}

/// __write_file: String オブジェクトパスと String オブジェクト内容を受け取り、書き込みバイト数を返す
fn emit_write_file_func(
    codes: &mut CodeSection,
    path_open_idx: u32,
    fd_write_idx: u32,
    fd_close_idx: u32,
) {
    use wasm_encoder::Instruction as W;

    // locals: 0=path(i64), 1=content(i64), 2=path_offset(i32), 3=path_len(i32),
    //         4=content_offset(i32), 5=content_len(i32), 6=fd(i32), 7=nwritten(i32)
    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // 2: path_offset (= path_addr + 8)
        (1, ValType::I32), // 3: path_len
        (1, ValType::I32), // 4: content_offset (= content_addr + 8)
        (1, ValType::I32), // 5: content_len
        (1, ValType::I32), // 6: fd
        (1, ValType::I32), // 7: nwritten
    ]);

    // パスの bytes を取得: path_offset = path_addr + 8
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(2)); // path_offset

    // path_len = i32.load(path_addr + 4)
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));
    f.instruction(&W::LocalSet(3)); // path_len

    // 内容の bytes を取得: content_offset = content_addr + 8
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(4)); // content_offset

    // content_len = i32.load(content_addr + 4)
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));
    f.instruction(&W::LocalSet(5)); // content_len

    // path_open(dirfd=3, dirflags=0, path, path_len, oflags=1(creat)|4(trunc), rights, 0, 0, fd_ptr=280)
    f.instruction(&W::I32Const(3));       // dirfd = 3
    f.instruction(&W::I32Const(0));       // dirflags
    f.instruction(&W::LocalGet(2));       // path
    f.instruction(&W::LocalGet(3));       // path_len
    f.instruction(&W::I32Const(5));       // oflags = O_CREAT(1) | O_TRUNC(4)
    f.instruction(&W::I64Const(0x40));    // rights_base = fd_write
    f.instruction(&W::I64Const(0));       // rights_inheriting
    f.instruction(&W::I32Const(0));       // fdflags
    f.instruction(&W::I32Const(280));     // fd_ptr
    f.instruction(&W::Call(path_open_idx));
    f.instruction(&W::Drop);

    // fd を読み出し
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }));
    f.instruction(&W::LocalSet(6)); // fd

    // iov 設定 (スクラッチ 352)
    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Store(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 })); // iov.buf

    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Store(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 })); // iov.len

    // fd_write(fd, iovs=352, iovs_len=1, nwritten_ptr=360)
    f.instruction(&W::LocalGet(6));       // fd
    f.instruction(&W::I32Const(352));     // iovs
    f.instruction(&W::I32Const(1));       // iovs_len
    f.instruction(&W::I32Const(360));     // nwritten
    f.instruction(&W::Call(fd_write_idx));
    f.instruction(&W::Drop);

    // nwritten を読み取り
    f.instruction(&W::I32Const(360));
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }));
    f.instruction(&W::LocalSet(7));

    // fd_close
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::Drop);

    // 書き込みバイト数を返す
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I64ExtendI32U);

    f.instruction(&W::End);
    codes.function(&f);
}

/// __file_exists: String オブジェクトパスを受け取り、存在すれば 1、しなければ 0 を返す
fn emit_file_exists_func(
    codes: &mut CodeSection,
    path_open_idx: u32,
    fd_close_idx: u32,
) {
    use wasm_encoder::Instruction as W;

    // locals: 0=path(i64), 1=path_offset(i32), 2=path_len(i32), 3=errno(i32)
    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // 1: path_offset (= path_addr + 8)
        (1, ValType::I32), // 2: path_len
        (1, ValType::I32), // 3: errno
    ]);

    // String オブジェクトからパス情報を取得
    // path_offset = path_addr + 8
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(1)); // path_offset

    // path_len = i32.load(path_addr + 4)
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));
    f.instruction(&W::LocalSet(2)); // path_len

    // path_open(dirfd=3, 0, path, path_len, 0, rights, 0, 0, fd_ptr=280)
    f.instruction(&W::I32Const(3));       // dirfd = 3
    f.instruction(&W::I32Const(0));       // dirflags
    f.instruction(&W::LocalGet(1));       // path
    f.instruction(&W::LocalGet(2));       // path_len
    f.instruction(&W::I32Const(0));       // oflags = 0 (read)
    f.instruction(&W::I64Const(0x02));    // rights_base = fd_read
    f.instruction(&W::I64Const(0));       // rights_inheriting
    f.instruction(&W::I32Const(0));       // fdflags
    f.instruction(&W::I32Const(280));     // fd_ptr
    f.instruction(&W::Call(path_open_idx));
    f.instruction(&W::LocalSet(3)); // errno

    // errno == 0 → ファイル存在、fd_close して 1 を返す
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    // fd_close
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::Drop);
    f.instruction(&W::End);

    // 結果: errno == 0 なら 1、それ以外 0
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::I64ExtendI32U);

    f.instruction(&W::End);
    codes.function(&f);
}

/// __command_line_args: コマンドライン引数の数を返す
fn emit_command_line_args_func(
    codes: &mut CodeSection,
    args_sizes_get_idx: u32,
) {
    use wasm_encoder::Instruction as W;

    // locals: なし (スクラッチ領域を使用)
    let mut f = wasm_encoder::Function::new(vec![]);

    // args_sizes_get(argc_ptr=280, argv_buf_size_ptr=284)
    f.instruction(&W::I32Const(280));     // argc ptr
    f.instruction(&W::I32Const(284));     // argv_buf_size ptr
    f.instruction(&W::Call(args_sizes_get_idx));
    f.instruction(&W::Drop);             // errno

    // argc を読み取って返す
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 0, align: 2, memory_index: 0 }));
    f.instruction(&W::I64ExtendI32U);

    f.instruction(&W::End);
    codes.function(&f);
}


/// __fnv1a_hash: String オブジェクト (ヒープ上) の FNV-1a ハッシュ値を計算
/// String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
/// パラメータ: local 0 = str_obj (i64: String オブジェクトのアドレス)
/// 戻り値: ハッシュ値 (i64)、0 と -1 を避けるため +2 する
fn emit_fnv1a_hash_func(codes: &mut CodeSection) {
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
    f.instruction(&W::I32Load(wasm_encoder::MemArg { offset: 4, align: 2, memory_index: 0 }));
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
    f.instruction(&W::I32Load8U(wasm_encoder::MemArg { offset: 0, align: 0, memory_index: 0 }));
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

/// IR 命令を WASI 用にリマップして出力
#[allow(clippy::too_many_arguments)]
fn emit_instructions_wasi(
    func: &mut wasm_encoder::Function,
    instructions: &[Instruction],
    print_helper_idx: u32,
    alloc_func_idx: u32,
    string_concat_idx: u32,
    string_eq_idx: u32,
    print_string_idx: u32,
    proc_exit_wasm_idx: u32,
    int_to_string_idx: u32,
    read_file_idx: u32,
    write_file_idx: u32,
    file_exists_idx: u32,
    command_line_args_idx: u32,
    fnv1a_hash_idx: u32,
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
                    5 => proc_exit_wasm_idx,
                    6 => int_to_string_idx,
                    7 => read_file_idx,
                    8 => write_file_idx,
                    9 => file_exists_idx,
                    10 => command_line_args_idx,
                    11 => fnv1a_hash_idx,
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
            5 => { f.instruction(&W::Call(proc_exit_wasm_idx)); }
            6 => { f.instruction(&W::Call(int_to_string_idx)); }
            7 => { f.instruction(&W::Call(read_file_idx)); }
            8 => { f.instruction(&W::Call(write_file_idx)); }
            9 => { f.instruction(&W::Call(file_exists_idx)); }
            10 => { f.instruction(&W::Call(command_line_args_idx)); }
            11 => { f.instruction(&W::Call(fnv1a_hash_idx)); }
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
    fn test_wasi_proc_exit_type_check() {
        // proc-exit が型チェックを通ること (Int -> Unit)
        let source = "(defn main [] (do (proc-exit 0) 0))";
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let result = infer.infer_program(&program);
        assert!(result.is_ok(), "proc-exit の型チェックが失敗: {:?}", result.err());
    }

    #[test]
    fn test_wasi_proc_exit_compile() {
        // proc-exit を含むコードがコンパイルでき、wasmtime で検証できること
        let wasm = compile_wasi("(defn main [] (do (proc-exit 0) 0))");
        assert!(wasm.len() > 8);
        assert_eq!(&wasm[0..4], b"\0asm");

        // wasmtime でモジュールを読み込めるか検証
        use wasmtime::Engine;
        let engine = Engine::default();
        wasmtime::Module::new(&engine, &wasm).expect("proc-exit を含むモジュールの読み込みに失敗");
    }

    #[test]
    fn test_wasi_proc_exit_run() {
        // proc-exit(0) を呼ぶと正常終了すること
        // wasmtime では proc_exit(0) は Trap ではなく正常終了として扱われる
        let wasm = compile_wasi("(defn main [] (do (print 42) (proc-exit 0) 0))");

        use wasmtime::*;
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t).unwrap();

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(1024);
        let wasi = WasiCtxBuilder::new().stdout(stdout.clone()).build_p1();

        let mut store = Store::new(&engine, wasi);
        let module = wasmtime::Module::new(&engine, &wasm).unwrap();
        let instance = linker.instantiate(&mut store, &module).unwrap();

        let start = instance.get_typed_func::<(), ()>(&mut store, "_start").unwrap();
        // proc_exit(0) は I32Exit(0) をトラップするが、exit code 0 は成功
        let result = start.call(&mut store, ());
        match result {
            Ok(()) => {} // 正常終了
            Err(e) => {
                // wasmtime は proc_exit を I32Exit として Trap する
                let exit_status = e.downcast_ref::<wasmtime_wasi::I32Exit>();
                assert!(exit_status.is_some(), "予期しないエラー: {e}");
                assert_eq!(exit_status.unwrap().0, 0, "exit code が 0 でない");
            }
        }

        drop(store);
        let bytes = stdout.try_into_inner().unwrap();
        let output = String::from_utf8(bytes.to_vec()).unwrap();
        assert_eq!(output, "42\n", "proc-exit 前の print 出力が正しくない");
    }

    #[test]
    fn test_wasi_additional_imports_validate() {
        // 新しい WASI import が追加されていても既存のコードが正しく動くことを検証
        let wasm = compile_wasi(
            "(defn fib [n]
               (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))
             (defn main [] (print (fib 10)))",
        );
        assert_eq!(run_wasi(&wasm), "55\n");
    }

    #[test]
    fn test_wasi_import_section_count() {
        // Import Section に 9 つの WASI 関数が含まれていることを検証
        // (fd_write, proc_exit, args_get, args_sizes_get, fd_read, fd_close, path_open, fd_seek, fd_filestat_get)
        let wasm = compile_wasi("(defn main [] (print 42))");

        // wasmtime でモジュールを読み込んで import 数を検証
        use wasmtime::Engine;
        let engine = Engine::default();
        let module = wasmtime::Module::new(&engine, &wasm).unwrap();
        let imports: Vec<_> = module.imports().collect();
        assert_eq!(imports.len(), 9, "WASI import 数が 9 でない: {:?}",
            imports.iter().map(|i| i.name().to_string()).collect::<Vec<_>>());

        // 各 import 名を検証
        let import_names: Vec<_> = imports.iter().map(|i| i.name().to_string()).collect();
        assert!(import_names.contains(&"fd_write".to_string()));
        assert!(import_names.contains(&"proc_exit".to_string()));
        assert!(import_names.contains(&"args_get".to_string()));
        assert!(import_names.contains(&"args_sizes_get".to_string()));
        assert!(import_names.contains(&"fd_read".to_string()));
        assert!(import_names.contains(&"fd_close".to_string()));
        assert!(import_names.contains(&"path_open".to_string()));
        assert!(import_names.contains(&"fd_seek".to_string()));
        assert!(import_names.contains(&"fd_filestat_get".to_string()));
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
