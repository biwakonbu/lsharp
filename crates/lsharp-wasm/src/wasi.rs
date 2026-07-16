//! WASI 対応の Wasm コード生成
//!
//! wasmtime で直接実行可能な Wasm バイナリを生成する。
//! print 関数を WASI の fd_write で実装し、_start エントリポイントを生成。

use lsharp_ir::{GcTypeKind, Instruction, Module};
use std::{collections::HashMap, path::PathBuf};
use wasm_encoder::{
    CodeSection, DataSection, ElementSection, Elements, EntityType, ExportKind, ExportSection,
    FunctionSection, GlobalSection, GlobalType, ImportSection, MemorySection, MemoryType,
    TableSection, TableType, TypeSection, ValType,
};

use crate::codegen::CodegenError;

/// メモリレイアウト定数
const NEWLINE_ADDR: i32 = 0;
const IOV_ADDR: i32 = 16;
const NWRITTEN_ADDR: i32 = 24;
const BUF_END: i32 = 276;
const ROOT_STACK_SLOT_CAPACITY: i32 = 32768;
const ROOT_STACK_BYTES: i32 = ROOT_STACK_SLOT_CAPACITY * 8;
const GC_OBJECT_SLOT_CAPACITY: i32 = 4096;
const GC_OBJECT_SLOT_BYTES: i32 = 16;
const GC_OBJECT_TABLE_BYTES: i32 = GC_OBJECT_SLOT_CAPACITY * GC_OBJECT_SLOT_BYTES;
const GC_FREE_LIST_SLOT_CAPACITY: i32 = 4096;
const GC_FREE_LIST_SLOT_BYTES: i32 = 8;
const GC_FREE_LIST_BYTES: i32 = GC_FREE_LIST_SLOT_CAPACITY * GC_FREE_LIST_SLOT_BYTES;
const TAGGED_POINTER_MASK: i64 = 1i64 << 63;
const HEAP_TAG_RECORD: i32 = 2;
const HEAP_TAG_ADT: i32 = 3;
const HEAP_TAG_CLOSURE: i32 = 4;
const HEAP_TAG_VECTOR: i32 = 5;
const HEAP_TAG_HASHMAP: i32 = 6;
const HEAP_TAG_REF: i32 = 7;
const GC_MARK_UNMARKED: i32 = 0;
const GC_MARK_PENDING: i32 = 1;
const GC_MARK_SCANNED: i32 = 2;
const HEAP_PTR_GLOBAL_IDX: u32 = 0;
const ROOT_STACK_TOP_GLOBAL_IDX: u32 = 1;
const ALLOC_COUNT_GLOBAL_IDX: u32 = 2;
const HEAP_START_GLOBAL_IDX: u32 = 3;
const GC_OBJECT_COUNT_GLOBAL_IDX: u32 = 4;
const GC_FREE_LIST_COUNT_GLOBAL_IDX: u32 = 5;
const GC_COLLECTION_COUNT_GLOBAL_IDX: u32 = 6;
const GC_FREED_COUNT_GLOBAL_IDX: u32 = 7;
const INTERNAL_HEAP_PTR_EXPORT: &str = "__lsharp_heap_ptr";
const INTERNAL_HEAP_START_EXPORT: &str = "__lsharp_heap_start";
const INTERNAL_ALLOC_COUNT_EXPORT: &str = "__lsharp_alloc_count";
const INTERNAL_ROOT_STACK_TOP_EXPORT: &str = "__lsharp_root_stack_top";
const INTERNAL_GC_LIVE_ALLOC_COUNT_EXPORT: &str = "__lsharp_gc_live_alloc_count";
const INTERNAL_GC_FREE_LIST_COUNT_EXPORT: &str = "__lsharp_gc_free_list_count";
const INTERNAL_GC_COLLECTION_COUNT_EXPORT: &str = "__lsharp_gc_collection_count";
const INTERNAL_GC_FREED_COUNT_EXPORT: &str = "__lsharp_gc_freed_count";
const INTERNAL_GC_COLLECT_EXPORT: &str = "__lsharp_gc_collect";

#[derive(Copy, Clone)]
struct AllocatorGlobals {
    heap_ptr_global_idx: u32,
    alloc_count_global_idx: u32,
    object_count_global_idx: u32,
    free_list_count_global_idx: u32,
}

#[derive(Copy, Clone)]
struct CollectorGlobals {
    heap_ptr_global_idx: u32,
    heap_start_global_idx: u32,
    root_stack_top_global_idx: u32,
    object_count_global_idx: u32,
    free_list_count_global_idx: u32,
    gc_collection_count_global_idx: u32,
    gc_freed_count_global_idx: u32,
}

#[derive(Copy, Clone)]
struct GcRuntimeLayout {
    gc_object_table_base: i32,
    gc_free_list_base: i32,
    root_stack_base: i32,
}

#[derive(Copy, Clone)]
struct GcMarkHelperLocals {
    old_count_local: u32,
    candidate_value_local: u32,
    candidate_addr_local: u32,
    search_idx_local: u32,
    search_entry_ptr_local: u32,
    temp_i64_local: u32,
}

/// IR 側の内部ヘルパー関数数
/// (print, __alloc, __string_concat, __string_eq, print-string, proc-exit, __int_to_string,
///  read-file, write-file, file-exists?, command-line-args, command-line-arg, read-stdin,
///  __fnv1a_hash, root_push, root_pop, root_set)
const IR_IMPORT_COUNT: u32 = 17;

/// WASI import 関数数
const WASI_IMPORT_COUNT: u32 = 9;

fn emit_tagged_pointer_from_i32_local(func: &mut wasm_encoder::Function, local_idx: u32) {
    use wasm_encoder::Instruction as W;

    func.instruction(&W::LocalGet(local_idx));
    func.instruction(&W::I64ExtendI32U);
    func.instruction(&W::I64Const(TAGGED_POINTER_MASK));
    func.instruction(&W::I64Add);
}

fn emit_tagged_pointer_from_i64_local(func: &mut wasm_encoder::Function, local_idx: u32) {
    use wasm_encoder::Instruction as W;

    func.instruction(&W::LocalGet(local_idx));
    func.instruction(&W::I64Const(TAGGED_POINTER_MASK));
    func.instruction(&W::I64Add);
}

/// WASI モードで Wasm バイナリを生成
pub fn emit_wasm_wasi(module: &Module) -> Result<Vec<u8>, CodegenError> {
    emit_wasm_wasi_with_options(module, false)
}

fn emit_wasm_wasi_with_options(
    module: &Module,
    export_component_run: bool,
) -> Result<Vec<u8>, CodegenError> {
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
    // 19: __command_line_arg
    // 20: __read_stdin
    // 21: __fnv1a_hash
    // 22: root_push
    // 23: root_pop
    // 24: root_set
    // 25: __write_file_bytes
    // 26: __gc_collect
    // 27..27+N-1: ユーザー関数
    // 27+N: __proc_exit_with_collect
    // 28+N: _start
    // 29+N: wasi:cli/run@0.2.3#run
    let fd_write_idx: u32 = 0;
    let proc_exit_wasm_idx: u32 = 1;
    let args_get_idx: u32 = 2;
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
    let command_line_arg_idx: u32 = WASI_IMPORT_COUNT + 10;
    let read_stdin_idx: u32 = WASI_IMPORT_COUNT + 11;
    let fnv1a_hash_idx: u32 = WASI_IMPORT_COUNT + 12;
    let root_push_idx: u32 = WASI_IMPORT_COUNT + 13;
    let root_pop_idx: u32 = WASI_IMPORT_COUNT + 14;
    let root_set_idx: u32 = WASI_IMPORT_COUNT + 15;
    let write_file_bytes_idx: u32 = WASI_IMPORT_COUNT + 16;
    let gc_collect_idx: u32 = WASI_IMPORT_COUNT + 17;
    let user_func_base: u32 = WASI_IMPORT_COUNT + 18;
    let proc_exit_helper_idx: u32 = user_func_base + module.functions.len() as u32;
    let start_func_idx: u32 = proc_exit_helper_idx + 1;
    let component_run_func_idx: u32 = start_func_idx + 1;

    // === Type Section ===
    let mut types = TypeSection::new();

    let fd_write_type_idx = types.len();
    types
        .ty()
        .function(vec![ValType::I32; 4], vec![ValType::I32]);

    // proc_exit(code: i32) -> ()
    let proc_exit_type_idx = types.len();
    types.ty().function(vec![ValType::I32], vec![]);

    // args_get(argv: i32, argv_buf: i32) -> i32
    let args_get_type_idx = types.len();
    types
        .ty()
        .function(vec![ValType::I32; 2], vec![ValType::I32]);

    // args_sizes_get(argc: i32, argv_buf_size: i32) -> i32
    let args_sizes_get_type_idx = types.len();
    types
        .ty()
        .function(vec![ValType::I32; 2], vec![ValType::I32]);

    // fd_read(fd: i32, iovs: i32, iovs_len: i32, nread: i32) -> i32
    let fd_read_type_idx = types.len();
    types
        .ty()
        .function(vec![ValType::I32; 4], vec![ValType::I32]);

    // fd_close(fd: i32) -> i32
    let fd_close_type_idx = types.len();
    types.ty().function(vec![ValType::I32], vec![ValType::I32]);

    // path_open(dirfd: i32, dirflags: i32, path: i32, path_len: i32,
    //           oflags: i32, fs_rights_base: i64, fs_rights_inheriting: i64,
    //           fdflags: i32, fd: i32) -> i32
    let path_open_type_idx = types.len();
    types.ty().function(
        vec![
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I64,
            ValType::I64,
            ValType::I32,
            ValType::I32,
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
    types
        .ty()
        .function(vec![ValType::I32, ValType::I32], vec![ValType::I32]);

    let print_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![]);

    let alloc_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    let string_concat_type_idx = types.len();
    types
        .ty()
        .function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    let string_eq_type_idx = types.len();
    types
        .ty()
        .function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

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
    types
        .ty()
        .function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    // __file_exists: (i64) -> i64 (パス → 0 or 1)
    let file_exists_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    // __command_line_args: () -> i64 (引数の数を返す)
    let command_line_args_type_idx = types.len();
    types.ty().function(vec![], vec![ValType::I64]);

    // __command_line_arg: (i64) -> i64 (index → String object)
    let command_line_arg_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    // __read_stdin: () -> i64 (stdin 全体を String object で返す)
    let read_stdin_type_idx = types.len();
    types.ty().function(vec![], vec![ValType::I64]);

    // __fnv1a_hash: (i64) -> i64 (パック文字列 → FNV-1a ハッシュ値)
    let fnv1a_hash_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    let mut user_type_indices = Vec::new();
    for func in &module.functions {
        let type_idx = types.len();
        let params: Vec<ValType> = func
            .params
            .iter()
            .map(|t| crate::emit::ir_to_wasm_valtype(*t))
            .collect();
        let results = vec![crate::emit::ir_to_wasm_valtype(func.result)];
        types.ty().function(params, results);
        user_type_indices.push(type_idx);
    }

    let start_type_idx = types.len();
    types.ty().function(vec![], vec![]);

    let component_run_type_idx = if export_component_run {
        let type_idx = types.len();
        types.ty().function(vec![], vec![ValType::I32]);
        Some(type_idx)
    } else {
        None
    };

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
    imports.import(
        "wasi_snapshot_preview1",
        "fd_write",
        EntityType::Function(fd_write_type_idx),
    );
    imports.import(
        "wasi_snapshot_preview1",
        "proc_exit",
        EntityType::Function(proc_exit_type_idx),
    );
    imports.import(
        "wasi_snapshot_preview1",
        "args_get",
        EntityType::Function(args_get_type_idx),
    );
    imports.import(
        "wasi_snapshot_preview1",
        "args_sizes_get",
        EntityType::Function(args_sizes_get_type_idx),
    );
    imports.import(
        "wasi_snapshot_preview1",
        "fd_read",
        EntityType::Function(fd_read_type_idx),
    );
    imports.import(
        "wasi_snapshot_preview1",
        "fd_close",
        EntityType::Function(fd_close_type_idx),
    );
    imports.import(
        "wasi_snapshot_preview1",
        "path_open",
        EntityType::Function(path_open_type_idx),
    );
    imports.import(
        "wasi_snapshot_preview1",
        "fd_seek",
        EntityType::Function(fd_seek_type_idx),
    );
    imports.import(
        "wasi_snapshot_preview1",
        "fd_filestat_get",
        EntityType::Function(fd_filestat_get_type_idx),
    );
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
    functions.function(command_line_arg_type_idx);
    functions.function(read_stdin_type_idx);
    functions.function(fnv1a_hash_type_idx);
    functions.function(alloc_type_idx);
    functions.function(read_stdin_type_idx);
    functions.function(string_concat_type_idx);
    functions.function(write_file_type_idx);
    functions.function(read_stdin_type_idx);
    for &type_idx in &user_type_indices {
        functions.function(type_idx);
    }
    functions.function(proc_exit_type_idx);
    functions.function(start_type_idx);
    if let Some(type_idx) = component_run_type_idx {
        functions.function(type_idx);
    }
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
    let total_string_data_size: i32 = module
        .string_data
        .iter()
        .map(|(_, bytes)| bytes.len() as i32)
        .sum();
    let gc_object_table_base = ((512 + total_string_data_size) + 7) & !7;
    let gc_free_list_base = ((gc_object_table_base + GC_OBJECT_TABLE_BYTES) + 7) & !7;
    let root_stack_base = ((gc_free_list_base + GC_FREE_LIST_BYTES) + 7) & !7;
    let heap_start = ((root_stack_base + ROOT_STACK_BYTES) + 7) & !7;
    let minimum_pages = (heap_start as u64).div_ceil(65536);
    let allocator_globals = AllocatorGlobals {
        heap_ptr_global_idx: HEAP_PTR_GLOBAL_IDX,
        alloc_count_global_idx: ALLOC_COUNT_GLOBAL_IDX,
        object_count_global_idx: GC_OBJECT_COUNT_GLOBAL_IDX,
        free_list_count_global_idx: GC_FREE_LIST_COUNT_GLOBAL_IDX,
    };
    let collector_globals = CollectorGlobals {
        heap_ptr_global_idx: HEAP_PTR_GLOBAL_IDX,
        heap_start_global_idx: HEAP_START_GLOBAL_IDX,
        root_stack_top_global_idx: ROOT_STACK_TOP_GLOBAL_IDX,
        object_count_global_idx: GC_OBJECT_COUNT_GLOBAL_IDX,
        free_list_count_global_idx: GC_FREE_LIST_COUNT_GLOBAL_IDX,
        gc_collection_count_global_idx: GC_COLLECTION_COUNT_GLOBAL_IDX,
        gc_freed_count_global_idx: GC_FREED_COUNT_GLOBAL_IDX,
    };
    let gc_layout = GcRuntimeLayout {
        gc_object_table_base,
        gc_free_list_base,
        root_stack_base,
    };

    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: minimum_pages.max(1),
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    wasm_module.section(&memories);

    // === Global Section ===
    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(heap_start),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(0),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(0),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: false,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(heap_start),
    );
    for _ in 0..4 {
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &wasm_encoder::ConstExpr::i32_const(0),
        );
    }
    wasm_module.section(&globals);

    // === Export Section ===
    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export("_start", ExportKind::Func, start_func_idx);
    exports.export(
        INTERNAL_HEAP_PTR_EXPORT,
        ExportKind::Global,
        HEAP_PTR_GLOBAL_IDX,
    );
    exports.export(
        INTERNAL_ROOT_STACK_TOP_EXPORT,
        ExportKind::Global,
        ROOT_STACK_TOP_GLOBAL_IDX,
    );
    exports.export(
        INTERNAL_ALLOC_COUNT_EXPORT,
        ExportKind::Global,
        ALLOC_COUNT_GLOBAL_IDX,
    );
    exports.export(
        INTERNAL_HEAP_START_EXPORT,
        ExportKind::Global,
        HEAP_START_GLOBAL_IDX,
    );
    exports.export(
        INTERNAL_GC_LIVE_ALLOC_COUNT_EXPORT,
        ExportKind::Global,
        GC_OBJECT_COUNT_GLOBAL_IDX,
    );
    exports.export(
        INTERNAL_GC_FREE_LIST_COUNT_EXPORT,
        ExportKind::Global,
        GC_FREE_LIST_COUNT_GLOBAL_IDX,
    );
    exports.export(
        INTERNAL_GC_COLLECTION_COUNT_EXPORT,
        ExportKind::Global,
        GC_COLLECTION_COUNT_GLOBAL_IDX,
    );
    exports.export(
        INTERNAL_GC_FREED_COUNT_EXPORT,
        ExportKind::Global,
        GC_FREED_COUNT_GLOBAL_IDX,
    );
    exports.export(INTERNAL_GC_COLLECT_EXPORT, ExportKind::Func, gc_collect_idx);
    if export_component_run {
        exports.export(
            "wasi:cli/run@0.2.3#run",
            ExportKind::Func,
            component_run_func_idx,
        );
    }
    wasm_module.section(&exports);

    // === Element Section (クロージャ用テーブル初期化) ===
    if needs_table {
        let total_funcs = start_func_idx + 1;
        let mut elements = ElementSection::new();
        // テーブル 0 を全関数で初期化
        let func_indices: Vec<u32> = (0..total_funcs).collect();
        elements.active(
            Some(0),                                // table index
            &wasm_encoder::ConstExpr::i32_const(0), // offset
            Elements::Functions(std::borrow::Cow::Owned(func_indices)),
        );
        wasm_module.section(&elements);
    }

    // === Code Section ===
    let mut codes = CodeSection::new();
    emit_print_i64_func(&mut codes);
    emit_alloc_func(&mut codes, allocator_globals, gc_layout);
    emit_string_concat_func(&mut codes, alloc_func_idx);
    emit_string_eq_func(&mut codes);
    emit_print_string_func(&mut codes);
    emit_int_to_string_func(&mut codes, alloc_func_idx);
    emit_read_file_func(
        &mut codes,
        alloc_func_idx,
        path_open_idx,
        fd_read_idx,
        fd_close_idx,
        fd_filestat_get_idx,
    );
    emit_write_file_func(&mut codes, path_open_idx, fd_write_idx, fd_close_idx);
    emit_file_exists_func(&mut codes, path_open_idx, fd_close_idx);
    emit_command_line_args_func(&mut codes, args_sizes_get_idx);
    emit_command_line_arg_func(&mut codes, alloc_func_idx, args_get_idx, args_sizes_get_idx);
    emit_read_stdin_func(&mut codes, alloc_func_idx, string_concat_idx, fd_read_idx);
    emit_fnv1a_hash_func(&mut codes);
    emit_root_push_func(&mut codes, ROOT_STACK_TOP_GLOBAL_IDX, root_stack_base);
    emit_root_pop_func(&mut codes, ROOT_STACK_TOP_GLOBAL_IDX, root_stack_base);
    emit_root_set_func(&mut codes, ROOT_STACK_TOP_GLOBAL_IDX, root_stack_base);
    emit_write_file_bytes_func(
        &mut codes,
        alloc_func_idx,
        path_open_idx,
        fd_write_idx,
        fd_close_idx,
    );
    emit_gc_collect_func(&mut codes, collector_globals, gc_layout);

    let struct_scratch_fields = max_struct_field_count(module);
    for func in &module.functions {
        let scratch_base = func.params.len() as u32 + func.locals.len() as u32;
        let mut locals = func
            .locals
            .iter()
            .map(|t| (1, crate::emit::ir_to_wasm_valtype(*t)))
            .collect::<Vec<_>>();
        locals.push((struct_scratch_fields, ValType::I64));
        locals.push((1, ValType::I64));
        locals.push((1, ValType::I32));
        let scratch = WasiStructScratch {
            field_base: scratch_base,
            ptr_local: scratch_base + struct_scratch_fields,
            addr_local: scratch_base + struct_scratch_fields + 1,
        };
        let mut f = wasm_encoder::Function::new(locals);
        emit_instructions_wasi(
            &mut f,
            &func.body,
            &module.gc_types,
            scratch,
            print_helper_idx,
            alloc_func_idx,
            string_concat_idx,
            string_eq_idx,
            print_string_idx,
            proc_exit_helper_idx,
            int_to_string_idx,
            read_file_idx,
            write_file_idx,
            Some(write_file_bytes_idx),
            file_exists_idx,
            command_line_args_idx,
            command_line_arg_idx,
            read_stdin_idx,
            fnv1a_hash_idx,
            root_push_idx,
            root_pop_idx,
            root_set_idx,
            user_func_base,
            &call_indirect_type_map,
        )?;
        f.instruction(&wasm_encoder::Instruction::End);
        codes.function(&f);
    }

    // __proc_exit_with_collect
    {
        let mut f = wasm_encoder::Function::new(vec![]);
        f.instruction(&wasm_encoder::Instruction::LocalGet(0));
        f.instruction(&wasm_encoder::Instruction::I32Eqz);
        f.instruction(&wasm_encoder::Instruction::If(
            wasm_encoder::BlockType::Empty,
        ));
        f.instruction(&wasm_encoder::Instruction::Call(gc_collect_idx));
        f.instruction(&wasm_encoder::Instruction::Drop);
        f.instruction(&wasm_encoder::Instruction::End);
        f.instruction(&wasm_encoder::Instruction::LocalGet(0));
        f.instruction(&wasm_encoder::Instruction::Call(proc_exit_wasm_idx));
        f.instruction(&wasm_encoder::Instruction::End);
        codes.function(&f);
    }

    // _start
    {
        let mut f = wasm_encoder::Function::new(vec![]);
        // マルチファイル結合時に各モジュールが (defn main []) を持つため、先頭の main は先頭ファイルのテスト用になる。
        // エントリ Main.ls の main を選ぶため、最後に定義された main を呼ぶ。
        if let Some(main_idx) = module.functions.iter().rposition(|f| f.name == "main") {
            f.instruction(&wasm_encoder::Instruction::Call(
                user_func_base + main_idx as u32,
            ));
            f.instruction(&wasm_encoder::Instruction::Drop);
            f.instruction(&wasm_encoder::Instruction::Call(gc_collect_idx));
            f.instruction(&wasm_encoder::Instruction::Drop);
        }
        f.instruction(&wasm_encoder::Instruction::End);
        codes.function(&f);
    }

    if export_component_run {
        let mut f = wasm_encoder::Function::new(vec![]);
        f.instruction(&wasm_encoder::Instruction::Call(start_func_idx));
        f.instruction(&wasm_encoder::Instruction::I32Const(0));
        f.instruction(&wasm_encoder::Instruction::End);
        codes.function(&f);
    }

    wasm_module.section(&codes);

    // === Data Section ===
    let mut data = DataSection::new();
    data.active(
        0,
        &wasm_encoder::ConstExpr::i32_const(NEWLINE_ADDR),
        b"\n".iter().copied(),
    );
    let mut str_offset = 512i32;
    for (_label, bytes) in &module.string_data {
        data.active(
            0,
            &wasm_encoder::ConstExpr::i32_const(str_offset),
            bytes.iter().copied(),
        );
        str_offset += bytes.len() as i32;
    }
    wasm_module.section(&data);

    Ok(wasm_module.finish())
}

/// Preview2/component 化向けの Wasm Component を生成する。
pub fn emit_wasm_wasi_p2(module: &Module) -> Result<Vec<u8>, CodegenError> {
    if is_http_handler_module(module) {
        return emit_wasm_http_handler_p2(module);
    }

    let core_wasm = emit_wasm_wasi_with_options(module, true)?;
    let wit_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-compiler.wit");
    let adapter = crate::preview1_component_adapter::build_preview1_component_adapter(&wit_dir)
        .map_err(|err| CodegenError::Error {
            msg: format!("Preview1 adapter の構築に失敗しました: {err}"),
        })?;
    crate::component_adapter::componentize_core_module(
        &core_wasm,
        &wit_dir,
        "lsharp-compiler",
        &[crate::component_adapter::NamedAdapter {
            name: "wasi_snapshot_preview1",
            bytes: &adapter,
        }],
    )
    .map_err(|err| CodegenError::Error {
        msg: format!("Preview2 component 化に失敗しました: {err}"),
    })
}

fn is_http_handler_module(module: &Module) -> bool {
    !module.functions.iter().any(|func| func.name == "main")
        && module
            .functions
            .iter()
            .any(|func| func.name == "handle" && func.params.len() == 1)
}

/// HTTP handler world 向けの Wasm Component を生成する。
pub fn emit_wasm_http_handler_p2(module: &Module) -> Result<Vec<u8>, CodegenError> {
    let core_wasm = emit_wasm_http_handler_core(module)?;
    let wit_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("wit")
        .join("lsharp-http-handler.wit");
    crate::component_adapter::componentize_core_module(
        &core_wasm,
        &wit_file,
        "lsharp-http-handler",
        &[],
    )
    .map_err(|err| CodegenError::Error {
        msg: format!("HTTP handler component 化に失敗しました: {err}"),
    })
}

fn emit_wasm_http_handler_core(module: &Module) -> Result<Vec<u8>, CodegenError> {
    use wasm_encoder::Instruction as W;

    const HTTP_IMPORT_COUNT: u32 = 3;
    const HTTP_EXPORT_HANDLE: &str = "cm32p2|wasi:http/incoming-handler@0.2|handle";
    const HTTP_EXPORT_HANDLE_POST: &str = "cm32p2|wasi:http/incoming-handler@0.2|handle_post";
    const HTTP_MEMORY_EXPORT: &str = "cm32p2_memory";
    const HTTP_REALLOC_EXPORT: &str = "cm32p2_realloc";
    const HTTP_INITIALIZE_EXPORT: &str = "cm32p2_initialize";

    let handle_idx = module
        .functions
        .iter()
        .rposition(|func| func.name == "handle" && func.params.len() == 1)
        .ok_or_else(|| CodegenError::Error {
            msg: "HTTP handler component には `(defn handle [request] response)` が必要です"
                .to_string(),
        })? as u32;

    let fields_ctor_idx: u32 = 0;
    let outgoing_response_ctor_idx: u32 = 1;
    let response_outparam_set_idx: u32 = 2;
    let print_helper_idx: u32 = HTTP_IMPORT_COUNT;
    let alloc_func_idx: u32 = HTTP_IMPORT_COUNT + 1;
    let string_concat_idx: u32 = HTTP_IMPORT_COUNT + 2;
    let string_eq_idx: u32 = HTTP_IMPORT_COUNT + 3;
    let print_string_idx: u32 = HTTP_IMPORT_COUNT + 4;
    let proc_exit_idx: u32 = HTTP_IMPORT_COUNT + 5;
    let int_to_string_idx: u32 = HTTP_IMPORT_COUNT + 6;
    let read_file_idx: u32 = HTTP_IMPORT_COUNT + 7;
    let write_file_idx: u32 = HTTP_IMPORT_COUNT + 8;
    let file_exists_idx: u32 = HTTP_IMPORT_COUNT + 9;
    let command_line_args_idx: u32 = HTTP_IMPORT_COUNT + 10;
    let command_line_arg_idx: u32 = HTTP_IMPORT_COUNT + 11;
    let read_stdin_idx: u32 = HTTP_IMPORT_COUNT + 12;
    let fnv1a_hash_idx: u32 = HTTP_IMPORT_COUNT + 13;
    let root_push_idx: u32 = HTTP_IMPORT_COUNT + 14;
    let root_pop_idx: u32 = HTTP_IMPORT_COUNT + 15;
    let root_set_idx: u32 = HTTP_IMPORT_COUNT + 16;
    let _gc_collect_idx: u32 = HTTP_IMPORT_COUNT + 17;
    let user_func_base: u32 = HTTP_IMPORT_COUNT + IR_IMPORT_COUNT + 1;
    let handle_wrapper_idx: u32 = user_func_base + module.functions.len() as u32;
    let handle_post_idx: u32 = handle_wrapper_idx + 1;
    let realloc_idx: u32 = handle_post_idx + 1;
    let initialize_idx: u32 = realloc_idx + 1;

    let mut wasm_module = wasm_encoder::Module::new();
    let mut types = TypeSection::new();

    let fields_ctor_type_idx = types.len();
    types.ty().function(vec![], vec![ValType::I32]);

    let outgoing_response_ctor_type_idx = types.len();
    types.ty().function(vec![ValType::I32], vec![ValType::I32]);

    let response_outparam_set_type_idx = types.len();
    types.ty().function(
        vec![
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I64,
            ValType::I32,
            ValType::I32,
            ValType::I32,
            ValType::I32,
        ],
        vec![],
    );

    let print_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![]);

    let alloc_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    let string_concat_type_idx = types.len();
    types
        .ty()
        .function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    let string_eq_type_idx = types.len();
    types
        .ty()
        .function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    let print_string_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![]);

    let proc_exit_type_idx = types.len();
    types.ty().function(vec![ValType::I32], vec![]);

    let int_to_string_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    let read_file_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    let write_file_type_idx = types.len();
    types
        .ty()
        .function(vec![ValType::I64, ValType::I64], vec![ValType::I64]);

    let file_exists_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    let command_line_args_type_idx = types.len();
    types.ty().function(vec![], vec![ValType::I64]);

    let command_line_arg_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    let read_stdin_type_idx = types.len();
    types.ty().function(vec![], vec![ValType::I64]);

    let fnv1a_hash_type_idx = types.len();
    types.ty().function(vec![ValType::I64], vec![ValType::I64]);

    let mut user_type_indices = Vec::new();
    for func in &module.functions {
        let type_idx = types.len();
        let params: Vec<ValType> = func
            .params
            .iter()
            .map(|ty| crate::emit::ir_to_wasm_valtype(*ty))
            .collect();
        let results = vec![crate::emit::ir_to_wasm_valtype(func.result)];
        types.ty().function(params, results);
        user_type_indices.push(type_idx);
    }

    let handle_wrapper_type_idx = types.len();
    types
        .ty()
        .function(vec![ValType::I32, ValType::I32], vec![]);

    let handle_post_type_idx = types.len();
    types.ty().function(vec![], vec![]);

    let realloc_type_idx = types.len();
    types.ty().function(
        vec![ValType::I32, ValType::I32, ValType::I32, ValType::I32],
        vec![ValType::I32],
    );

    let initialize_type_idx = types.len();
    types.ty().function(vec![], vec![]);

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

    let mut imports = ImportSection::new();
    imports.import(
        "cm32p2|wasi:http/types@0.2",
        "[constructor]fields",
        EntityType::Function(fields_ctor_type_idx),
    );
    imports.import(
        "cm32p2|wasi:http/types@0.2",
        "[constructor]outgoing-response",
        EntityType::Function(outgoing_response_ctor_type_idx),
    );
    imports.import(
        "cm32p2|wasi:http/types@0.2",
        "[static]response-outparam.set",
        EntityType::Function(response_outparam_set_type_idx),
    );
    wasm_module.section(&imports);

    let mut functions = FunctionSection::new();
    functions.function(print_type_idx);
    functions.function(alloc_type_idx);
    functions.function(string_concat_type_idx);
    functions.function(string_eq_type_idx);
    functions.function(print_string_type_idx);
    functions.function(proc_exit_type_idx);
    functions.function(int_to_string_type_idx);
    functions.function(read_file_type_idx);
    functions.function(write_file_type_idx);
    functions.function(file_exists_type_idx);
    functions.function(command_line_args_type_idx);
    functions.function(command_line_arg_type_idx);
    functions.function(read_stdin_type_idx);
    functions.function(fnv1a_hash_type_idx);
    functions.function(alloc_type_idx);
    functions.function(read_stdin_type_idx);
    functions.function(string_concat_type_idx);
    functions.function(read_stdin_type_idx);
    for &type_idx in &user_type_indices {
        functions.function(type_idx);
    }
    functions.function(handle_wrapper_type_idx);
    functions.function(handle_post_type_idx);
    functions.function(realloc_type_idx);
    functions.function(initialize_type_idx);
    wasm_module.section(&functions);

    if needs_table {
        let total_funcs = (initialize_idx + 1) as u64;
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

    let total_string_data_size: i32 = module
        .string_data
        .iter()
        .map(|(_, bytes)| bytes.len() as i32)
        .sum();
    let gc_object_table_base = ((512 + total_string_data_size) + 7) & !7;
    let gc_free_list_base = ((gc_object_table_base + GC_OBJECT_TABLE_BYTES) + 7) & !7;
    let root_stack_base = ((gc_free_list_base + GC_FREE_LIST_BYTES) + 7) & !7;
    let heap_start = ((root_stack_base + ROOT_STACK_BYTES) + 7) & !7;
    let minimum_pages = (heap_start as u64).div_ceil(65536);
    let allocator_globals = AllocatorGlobals {
        heap_ptr_global_idx: HEAP_PTR_GLOBAL_IDX,
        alloc_count_global_idx: ALLOC_COUNT_GLOBAL_IDX,
        object_count_global_idx: GC_OBJECT_COUNT_GLOBAL_IDX,
        free_list_count_global_idx: GC_FREE_LIST_COUNT_GLOBAL_IDX,
    };
    let collector_globals = CollectorGlobals {
        heap_ptr_global_idx: HEAP_PTR_GLOBAL_IDX,
        heap_start_global_idx: HEAP_START_GLOBAL_IDX,
        root_stack_top_global_idx: ROOT_STACK_TOP_GLOBAL_IDX,
        object_count_global_idx: GC_OBJECT_COUNT_GLOBAL_IDX,
        free_list_count_global_idx: GC_FREE_LIST_COUNT_GLOBAL_IDX,
        gc_collection_count_global_idx: GC_COLLECTION_COUNT_GLOBAL_IDX,
        gc_freed_count_global_idx: GC_FREED_COUNT_GLOBAL_IDX,
    };
    let gc_layout = GcRuntimeLayout {
        gc_object_table_base,
        gc_free_list_base,
        root_stack_base,
    };

    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: minimum_pages.max(1),
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    wasm_module.section(&memories);

    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(heap_start),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(0),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(0),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: false,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(heap_start),
    );
    for _ in 0..4 {
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &wasm_encoder::ConstExpr::i32_const(0),
        );
    }
    wasm_module.section(&globals);

    let mut exports = ExportSection::new();
    exports.export(HTTP_MEMORY_EXPORT, ExportKind::Memory, 0);
    exports.export(HTTP_EXPORT_HANDLE, ExportKind::Func, handle_wrapper_idx);
    exports.export(HTTP_EXPORT_HANDLE_POST, ExportKind::Func, handle_post_idx);
    exports.export(HTTP_REALLOC_EXPORT, ExportKind::Func, realloc_idx);
    exports.export(HTTP_INITIALIZE_EXPORT, ExportKind::Func, initialize_idx);
    wasm_module.section(&exports);

    if needs_table {
        let total_funcs = initialize_idx + 1;
        let mut elements = ElementSection::new();
        let func_indices: Vec<u32> = (0..total_funcs).collect();
        elements.active(
            Some(0),
            &wasm_encoder::ConstExpr::i32_const(0),
            Elements::Functions(std::borrow::Cow::Owned(func_indices)),
        );
        wasm_module.section(&elements);
    }

    let mut codes = CodeSection::new();
    emit_trap_i64_to_unit_func(&mut codes);
    emit_alloc_func(&mut codes, allocator_globals, gc_layout);
    emit_string_concat_func(&mut codes, alloc_func_idx);
    emit_string_eq_func(&mut codes);
    emit_trap_i64_to_unit_func(&mut codes);
    emit_trap_i32_to_unit_func(&mut codes);
    emit_int_to_string_func(&mut codes, alloc_func_idx);
    emit_trap_i64_to_i64_func(&mut codes);
    emit_trap_i64_i64_to_i64_func(&mut codes);
    emit_trap_i64_to_i64_func(&mut codes);
    emit_trap_void_to_i64_func(&mut codes);
    emit_trap_i64_to_i64_func(&mut codes);
    emit_trap_void_to_i64_func(&mut codes);
    emit_fnv1a_hash_func(&mut codes);
    emit_root_push_func(&mut codes, ROOT_STACK_TOP_GLOBAL_IDX, root_stack_base);
    emit_root_pop_func(&mut codes, ROOT_STACK_TOP_GLOBAL_IDX, root_stack_base);
    emit_root_set_func(&mut codes, ROOT_STACK_TOP_GLOBAL_IDX, root_stack_base);
    emit_gc_collect_func(&mut codes, collector_globals, gc_layout);

    let struct_scratch_fields = max_struct_field_count(module);
    for func in &module.functions {
        let scratch_base = func.params.len() as u32 + func.locals.len() as u32;
        let mut locals = func
            .locals
            .iter()
            .map(|ty| (1, crate::emit::ir_to_wasm_valtype(*ty)))
            .collect::<Vec<_>>();
        locals.push((struct_scratch_fields, ValType::I64));
        locals.push((1, ValType::I64));
        locals.push((1, ValType::I32));
        let scratch = WasiStructScratch {
            field_base: scratch_base,
            ptr_local: scratch_base + struct_scratch_fields,
            addr_local: scratch_base + struct_scratch_fields + 1,
        };
        let mut f = wasm_encoder::Function::new(locals);
        emit_instructions_wasi(
            &mut f,
            &func.body,
            &module.gc_types,
            scratch,
            print_helper_idx,
            alloc_func_idx,
            string_concat_idx,
            string_eq_idx,
            print_string_idx,
            proc_exit_idx,
            int_to_string_idx,
            read_file_idx,
            write_file_idx,
            None,
            file_exists_idx,
            command_line_args_idx,
            command_line_arg_idx,
            read_stdin_idx,
            fnv1a_hash_idx,
            root_push_idx,
            root_pop_idx,
            root_set_idx,
            user_func_base,
            &call_indirect_type_map,
        )?;
        f.instruction(&W::End);
        codes.function(&f);
    }

    {
        let mut f = wasm_encoder::Function::new(vec![(2, ValType::I32)]);
        f.instruction(&W::LocalGet(0));
        f.instruction(&W::I64ExtendI32U);
        f.instruction(&W::Call(user_func_base + handle_idx));
        f.instruction(&W::Drop);
        f.instruction(&W::Call(fields_ctor_idx));
        f.instruction(&W::LocalSet(2));
        f.instruction(&W::LocalGet(2));
        f.instruction(&W::Call(outgoing_response_ctor_idx));
        f.instruction(&W::LocalSet(3));
        f.instruction(&W::LocalGet(1));
        f.instruction(&W::I32Const(0));
        f.instruction(&W::LocalGet(3));
        f.instruction(&W::I32Const(0));
        f.instruction(&W::I64Const(0));
        f.instruction(&W::I32Const(0));
        f.instruction(&W::I32Const(0));
        f.instruction(&W::I32Const(0));
        f.instruction(&W::I32Const(0));
        f.instruction(&W::Call(response_outparam_set_idx));
        f.instruction(&W::End);
        codes.function(&f);
    }

    {
        let mut f = wasm_encoder::Function::new(vec![]);
        f.instruction(&W::End);
        codes.function(&f);
    }

    {
        let mut f = wasm_encoder::Function::new(vec![]);
        f.instruction(&W::LocalGet(3));
        f.instruction(&W::I64ExtendI32U);
        f.instruction(&W::Call(alloc_func_idx));
        f.instruction(&W::I32WrapI64);
        f.instruction(&W::End);
        codes.function(&f);
    }

    {
        let mut f = wasm_encoder::Function::new(vec![]);
        f.instruction(&W::End);
        codes.function(&f);
    }

    wasm_module.section(&codes);

    let mut data = DataSection::new();
    data.active(
        0,
        &wasm_encoder::ConstExpr::i32_const(NEWLINE_ADDR),
        b"\n".iter().copied(),
    );
    let mut str_offset = 512i32;
    for (_label, bytes) in &module.string_data {
        data.active(
            0,
            &wasm_encoder::ConstExpr::i32_const(str_offset),
            bytes.iter().copied(),
        );
        str_offset += bytes.len() as i32;
    }
    wasm_module.section(&data);

    Ok(wasm_module.finish())
}

fn emit_trap_i64_to_unit_func(codes: &mut CodeSection) {
    emit_trap_func(codes, vec![]);
}

fn emit_trap_i32_to_unit_func(codes: &mut CodeSection) {
    emit_trap_func(codes, vec![]);
}

fn emit_trap_i64_to_i64_func(codes: &mut CodeSection) {
    emit_trap_func(codes, vec![]);
}

fn emit_trap_i64_i64_to_i64_func(codes: &mut CodeSection) {
    emit_trap_func(codes, vec![]);
}

fn emit_trap_void_to_i64_func(codes: &mut CodeSection) {
    emit_trap_func(codes, vec![]);
}

fn emit_trap_func(codes: &mut CodeSection, locals: Vec<(u32, ValType)>) {
    use wasm_encoder::Instruction as W;

    let mut f = wasm_encoder::Function::new(locals);
    f.instruction(&W::Unreachable);
    f.instruction(&W::End);
    codes.function(&f);
}

/// __print_i64: i64 の値を10進文字列に変換して stdout に出力
fn emit_print_i64_func(codes: &mut CodeSection) {
    use wasm_encoder::Instruction as W;
    use wasm_encoder::MemArg;

    let mem = |offset: u64| MemArg {
        offset,
        align: 0,
        memory_index: 0,
    };
    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

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

/// __alloc: free-list reuse を持つ allocator
fn emit_alloc_func(codes: &mut CodeSection, globals: AllocatorGlobals, layout: GcRuntimeLayout) {
    use wasm_encoder::{Instruction as W, MemArg};

    let AllocatorGlobals {
        heap_ptr_global_idx,
        alloc_count_global_idx,
        object_count_global_idx,
        free_list_count_global_idx,
    } = globals;
    let GcRuntimeLayout {
        gc_object_table_base,
        gc_free_list_base,
        ..
    } = layout;

    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

    let mut f = wasm_encoder::Function::new(vec![(11, ValType::I32)]);

    // local1 = aligned size
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(7));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(-8));
    f.instruction(&W::I32And);
    f.instruction(&W::LocalSet(1));

    // local8 = allocated address (0 means not found in free-list)
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(8));

    // free-list first-fit search
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));

    f.instruction(&W::I32Const(gc_free_list_base));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(5));

    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(6));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(7));

    f.instruction(&W::LocalGet(7));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(6));
    f.instruction(&W::LocalSet(8));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::LocalSet(9));

    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::LocalSet(10));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::LocalGet(10));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Else);
    f.instruction(&W::I32Const(gc_free_list_base));
    f.instruction(&W::LocalGet(10));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(11));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(11));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(11));
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::I32Store(mem32(4)));
    f.instruction(&W::End);
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::GlobalSet(free_list_count_global_idx));
    f.instruction(&W::Else);
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I32Store(mem32(4)));
    f.instruction(&W::End);
    f.instruction(&W::Br(1));
    f.instruction(&W::End);
    f.instruction(&W::End);

    // free-list miss -> bump allocate
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(heap_ptr_global_idx));
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
    f.instruction(&W::GlobalSet(heap_ptr_global_idx));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalSet(8));
    f.instruction(&W::End);

    // live object metadata を記録
    f.instruction(&W::GlobalGet(object_count_global_idx));
    f.instruction(&W::I32Const(GC_OBJECT_SLOT_CAPACITY));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(gc_object_table_base));
    f.instruction(&W::GlobalGet(object_count_global_idx));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(5));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Store(mem32(4)));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(GC_MARK_UNMARKED));
    f.instruction(&W::I32Store(mem32(8)));
    f.instruction(&W::GlobalGet(object_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(object_count_global_idx));
    f.instruction(&W::End);

    f.instruction(&W::GlobalGet(alloc_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(alloc_count_global_idx));
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::End);
    codes.function(&f);
}

fn emit_gc_mark_candidate(
    f: &mut wasm_encoder::Function,
    globals: CollectorGlobals,
    layout: GcRuntimeLayout,
    locals: GcMarkHelperLocals,
) {
    use wasm_encoder::{Instruction as W, MemArg};

    let CollectorGlobals {
        heap_ptr_global_idx,
        heap_start_global_idx,
        ..
    } = globals;
    let GcRuntimeLayout {
        gc_object_table_base,
        ..
    } = layout;
    let GcMarkHelperLocals {
        old_count_local,
        candidate_value_local,
        candidate_addr_local,
        search_idx_local,
        search_entry_ptr_local,
        temp_i64_local,
    } = locals;

    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

    // raw address または tagged handle からヒープ先頭アドレスを抽出する。
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(candidate_addr_local));

    f.instruction(&W::LocalGet(candidate_value_local));
    f.instruction(&W::GlobalGet(heap_start_global_idx));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::I64GeS);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(candidate_value_local));
    f.instruction(&W::GlobalGet(heap_ptr_global_idx));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::I64LtS);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(candidate_value_local));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(candidate_addr_local));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(candidate_addr_local));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(candidate_value_local));
    f.instruction(&W::I64Const(TAGGED_POINTER_MASK));
    f.instruction(&W::I64GeU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(candidate_value_local));
    f.instruction(&W::I64Const(TAGGED_POINTER_MASK));
    f.instruction(&W::I64Sub);
    f.instruction(&W::LocalSet(temp_i64_local));
    f.instruction(&W::LocalGet(temp_i64_local));
    f.instruction(&W::GlobalGet(heap_start_global_idx));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::I64GeS);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(temp_i64_local));
    f.instruction(&W::GlobalGet(heap_ptr_global_idx));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::I64LtS);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(temp_i64_local));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(candidate_addr_local));
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);

    // object table 上の matching entry を探し、未マークなら pending にする。
    f.instruction(&W::LocalGet(candidate_addr_local));
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(search_idx_local));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(search_idx_local));
    f.instruction(&W::LocalGet(old_count_local));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));

    f.instruction(&W::I32Const(gc_object_table_base));
    f.instruction(&W::LocalGet(search_idx_local));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(search_entry_ptr_local));

    f.instruction(&W::LocalGet(search_entry_ptr_local));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalGet(candidate_addr_local));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(search_entry_ptr_local));
    f.instruction(&W::I32Load(mem32(8)));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(search_entry_ptr_local));
    f.instruction(&W::I32Const(GC_MARK_PENDING));
    f.instruction(&W::I32Store(mem32(8)));
    f.instruction(&W::End);
    f.instruction(&W::Br(2));
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(search_idx_local));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(search_idx_local));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);
}

fn emit_gc_collect_func(
    codes: &mut CodeSection,
    globals: CollectorGlobals,
    layout: GcRuntimeLayout,
) {
    use wasm_encoder::{Instruction as W, MemArg};

    const OLD_COUNT_LOCAL: u32 = 0;
    const READ_IDX_LOCAL: u32 = 1;
    const WRITE_IDX_LOCAL: u32 = 2;
    const ENTRY_PTR_LOCAL: u32 = 3;
    const OBJ_ADDR_LOCAL: u32 = 4;
    const OBJ_SIZE_LOCAL: u32 = 5;
    const MARK_STATE_LOCAL: u32 = 6;
    const ROOT_IDX_LOCAL: u32 = 7;
    const SLOT_ADDR_LOCAL: u32 = 8;
    const FREED_THIS_CYCLE_LOCAL: u32 = 9;
    const MARK_PROGRESS_LOCAL: u32 = 10;
    const CHILD_IDX_LOCAL: u32 = 11;
    const CHILD_LIMIT_LOCAL: u32 = 12;
    const CHILD_ENTRY_ADDR_LOCAL: u32 = 13;
    const TAG_LOCAL: u32 = 14;
    const TEMP_I32_LOCAL: u32 = 15;
    const CANDIDATE_ADDR_LOCAL: u32 = 16;
    const SEARCH_IDX_LOCAL: u32 = 17;
    const SEARCH_ENTRY_PTR_LOCAL: u32 = 18;
    const SLOT_VALUE_LOCAL: u32 = 19;
    const TEMP_I64_LOCAL: u32 = 20;
    const CHILD_VALUE_LOCAL: u32 = 21;

    let CollectorGlobals {
        heap_ptr_global_idx: _,
        heap_start_global_idx: _,
        root_stack_top_global_idx,
        object_count_global_idx,
        free_list_count_global_idx,
        gc_collection_count_global_idx,
        gc_freed_count_global_idx,
    } = globals;
    let GcRuntimeLayout {
        gc_object_table_base,
        gc_free_list_base,
        root_stack_base,
    } = layout;

    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };
    let mem64 = |offset: u64| MemArg {
        offset,
        align: 3,
        memory_index: 0,
    };

    let mut f = wasm_encoder::Function::new(vec![(19, ValType::I32), (3, ValType::I64)]);

    // mark bit をクリアしてから root stack を seed に fixed-point で trace する。
    f.instruction(&W::GlobalGet(object_count_global_idx));
    f.instruction(&W::LocalSet(OLD_COUNT_LOCAL));

    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(READ_IDX_LOCAL));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::LocalGet(OLD_COUNT_LOCAL));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::I32Const(gc_object_table_base));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Const(GC_MARK_UNMARKED));
    f.instruction(&W::I32Store(mem32(8)));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(READ_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(ROOT_IDX_LOCAL));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(ROOT_IDX_LOCAL));
    f.instruction(&W::GlobalGet(root_stack_top_global_idx));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::I32Const(root_stack_base));
    f.instruction(&W::LocalGet(ROOT_IDX_LOCAL));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(SLOT_ADDR_LOCAL));
    f.instruction(&W::LocalGet(SLOT_ADDR_LOCAL));
    f.instruction(&W::I64Load(mem64(0)));
    f.instruction(&W::LocalSet(SLOT_VALUE_LOCAL));
    emit_gc_mark_candidate(
        &mut f,
        globals,
        layout,
        GcMarkHelperLocals {
            old_count_local: OLD_COUNT_LOCAL,
            candidate_value_local: SLOT_VALUE_LOCAL,
            candidate_addr_local: CANDIDATE_ADDR_LOCAL,
            search_idx_local: SEARCH_IDX_LOCAL,
            search_entry_ptr_local: SEARCH_ENTRY_PTR_LOCAL,
            temp_i64_local: TEMP_I64_LOCAL,
        },
    );
    f.instruction(&W::LocalGet(ROOT_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ROOT_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(MARK_PROGRESS_LOCAL));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(READ_IDX_LOCAL));

    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::LocalGet(OLD_COUNT_LOCAL));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));

    f.instruction(&W::I32Const(gc_object_table_base));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(8)));
    f.instruction(&W::LocalSet(MARK_STATE_LOCAL));

    f.instruction(&W::LocalGet(MARK_STATE_LOCAL));
    f.instruction(&W::I32Const(GC_MARK_PENDING));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::LocalSet(MARK_PROGRESS_LOCAL));

    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(OBJ_ADDR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(OBJ_SIZE_LOCAL));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(TAG_LOCAL));

    f.instruction(&W::LocalGet(TAG_LOCAL));
    f.instruction(&W::I32Const(HEAP_TAG_REF));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::I64Load(mem64(8)));
    f.instruction(&W::LocalSet(CHILD_VALUE_LOCAL));
    emit_gc_mark_candidate(
        &mut f,
        globals,
        layout,
        GcMarkHelperLocals {
            old_count_local: OLD_COUNT_LOCAL,
            candidate_value_local: CHILD_VALUE_LOCAL,
            candidate_addr_local: CANDIDATE_ADDR_LOCAL,
            search_idx_local: SEARCH_IDX_LOCAL,
            search_entry_ptr_local: SEARCH_ENTRY_PTR_LOCAL,
            temp_i64_local: TEMP_I64_LOCAL,
        },
    );
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(TAG_LOCAL));
    f.instruction(&W::I32Const(HEAP_TAG_VECTOR));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::I32Load(mem32(8)));
    f.instruction(&W::LocalSet(CHILD_LIMIT_LOCAL));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(CHILD_IDX_LOCAL));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::LocalGet(CHILD_LIMIT_LOCAL));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(16));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::LocalGet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::I64Load(mem64(0)));
    f.instruction(&W::LocalSet(CHILD_VALUE_LOCAL));
    emit_gc_mark_candidate(
        &mut f,
        globals,
        layout,
        GcMarkHelperLocals {
            old_count_local: OLD_COUNT_LOCAL,
            candidate_value_local: CHILD_VALUE_LOCAL,
            candidate_addr_local: CANDIDATE_ADDR_LOCAL,
            search_idx_local: SEARCH_IDX_LOCAL,
            search_entry_ptr_local: SEARCH_ENTRY_PTR_LOCAL,
            temp_i64_local: TEMP_I64_LOCAL,
        },
    );
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(CHILD_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(TAG_LOCAL));
    f.instruction(&W::I32Const(HEAP_TAG_HASHMAP));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(CHILD_LIMIT_LOCAL));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(CHILD_IDX_LOCAL));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::LocalGet(CHILD_LIMIT_LOCAL));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(16));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::LocalGet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::I64Load(mem64(0)));
    f.instruction(&W::LocalSet(CHILD_VALUE_LOCAL));
    f.instruction(&W::LocalGet(CHILD_VALUE_LOCAL));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::I64Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Else);
    f.instruction(&W::LocalGet(CHILD_VALUE_LOCAL));
    f.instruction(&W::I64Const(-1));
    f.instruction(&W::I64Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Else);
    emit_gc_mark_candidate(
        &mut f,
        globals,
        layout,
        GcMarkHelperLocals {
            old_count_local: OLD_COUNT_LOCAL,
            candidate_value_local: CHILD_VALUE_LOCAL,
            candidate_addr_local: CANDIDATE_ADDR_LOCAL,
            search_idx_local: SEARCH_IDX_LOCAL,
            search_entry_ptr_local: SEARCH_ENTRY_PTR_LOCAL,
            temp_i64_local: TEMP_I64_LOCAL,
        },
    );
    f.instruction(&W::LocalGet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::I64Load(mem64(8)));
    f.instruction(&W::LocalSet(CHILD_VALUE_LOCAL));
    emit_gc_mark_candidate(
        &mut f,
        globals,
        layout,
        GcMarkHelperLocals {
            old_count_local: OLD_COUNT_LOCAL,
            candidate_value_local: CHILD_VALUE_LOCAL,
            candidate_addr_local: CANDIDATE_ADDR_LOCAL,
            search_idx_local: SEARCH_IDX_LOCAL,
            search_entry_ptr_local: SEARCH_ENTRY_PTR_LOCAL,
            temp_i64_local: TEMP_I64_LOCAL,
        },
    );
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(CHILD_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(TAG_LOCAL));
    f.instruction(&W::I32Const(HEAP_TAG_CLOSURE));
    f.instruction(&W::I32Eq);
    f.instruction(&W::LocalGet(TAG_LOCAL));
    f.instruction(&W::I32Const(HEAP_TAG_RECORD));
    f.instruction(&W::I32Eq);
    f.instruction(&W::I32Or);
    f.instruction(&W::LocalGet(TAG_LOCAL));
    f.instruction(&W::I32Const(HEAP_TAG_ADT));
    f.instruction(&W::I32Eq);
    f.instruction(&W::I32Or);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_SIZE_LOCAL));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32GtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_SIZE_LOCAL));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Sub);
    f.instruction(&W::LocalSet(TEMP_I32_LOCAL));
    f.instruction(&W::LocalGet(TEMP_I32_LOCAL));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32ShrU);
    f.instruction(&W::LocalSet(CHILD_LIMIT_LOCAL));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(CHILD_IDX_LOCAL));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::LocalGet(CHILD_LIMIT_LOCAL));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::LocalGet(CHILD_ENTRY_ADDR_LOCAL));
    f.instruction(&W::I64Load(mem64(0)));
    f.instruction(&W::LocalSet(CHILD_VALUE_LOCAL));
    emit_gc_mark_candidate(
        &mut f,
        globals,
        layout,
        GcMarkHelperLocals {
            old_count_local: OLD_COUNT_LOCAL,
            candidate_value_local: CHILD_VALUE_LOCAL,
            candidate_addr_local: CANDIDATE_ADDR_LOCAL,
            search_idx_local: SEARCH_IDX_LOCAL,
            search_entry_ptr_local: SEARCH_ENTRY_PTR_LOCAL,
            temp_i64_local: TEMP_I64_LOCAL,
        },
    );
    f.instruction(&W::LocalGet(CHILD_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(CHILD_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Const(GC_MARK_SCANNED));
    f.instruction(&W::I32Store(mem32(8)));
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(READ_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(MARK_PROGRESS_LOCAL));
    f.instruction(&W::BrIf(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(WRITE_IDX_LOCAL));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(FREED_THIS_CYCLE_LOCAL));

    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::LocalGet(OLD_COUNT_LOCAL));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));

    f.instruction(&W::I32Const(gc_object_table_base));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(OBJ_ADDR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(OBJ_SIZE_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(8)));
    f.instruction(&W::LocalSet(MARK_STATE_LOCAL));

    f.instruction(&W::LocalGet(MARK_STATE_LOCAL));
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(gc_object_table_base));
    f.instruction(&W::LocalGet(WRITE_IDX_LOCAL));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(OBJ_SIZE_LOCAL));
    f.instruction(&W::I32Store(mem32(4)));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Const(GC_MARK_UNMARKED));
    f.instruction(&W::I32Store(mem32(8)));
    f.instruction(&W::LocalGet(WRITE_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(WRITE_IDX_LOCAL));
    f.instruction(&W::Else);
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32Const(GC_FREE_LIST_SLOT_CAPACITY));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(gc_free_list_base));
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(OBJ_ADDR_LOCAL));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(OBJ_SIZE_LOCAL));
    f.instruction(&W::I32Store(mem32(4)));
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(free_list_count_global_idx));
    f.instruction(&W::LocalGet(FREED_THIS_CYCLE_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(FREED_THIS_CYCLE_LOCAL));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(READ_IDX_LOCAL));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(WRITE_IDX_LOCAL));
    f.instruction(&W::GlobalSet(object_count_global_idx));
    f.instruction(&W::GlobalGet(gc_collection_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(gc_collection_count_global_idx));
    f.instruction(&W::GlobalGet(gc_freed_count_global_idx));
    f.instruction(&W::LocalGet(FREED_THIS_CYCLE_LOCAL));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(gc_freed_count_global_idx));
    f.instruction(&W::LocalGet(FREED_THIS_CYCLE_LOCAL));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::End);
    codes.function(&f);
}

fn emit_root_push_func(
    codes: &mut CodeSection,
    root_stack_top_global_idx: u32,
    root_stack_base: i32,
) {
    use wasm_encoder::{Instruction as W, MemArg};

    let mem64 = |offset: u64| MemArg {
        offset,
        align: 3,
        memory_index: 0,
    };

    let mut f = wasm_encoder::Function::new(vec![(1, ValType::I32), (1, ValType::I32)]);
    f.instruction(&W::GlobalGet(root_stack_top_global_idx));
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(ROOT_STACK_SLOT_CAPACITY));
    f.instruction(&W::I32GeU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Unreachable);
    f.instruction(&W::End);
    f.instruction(&W::I32Const(root_stack_base));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(2));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64Store(mem64(0)));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(root_stack_top_global_idx));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::End);
    codes.function(&f);
}

fn emit_root_pop_func(
    codes: &mut CodeSection,
    root_stack_top_global_idx: u32,
    root_stack_base: i32,
) {
    use wasm_encoder::{Instruction as W, MemArg};

    let mem64 = |offset: u64| MemArg {
        offset,
        align: 3,
        memory_index: 0,
    };

    let mut f = wasm_encoder::Function::new(vec![(1, ValType::I32), (1, ValType::I32)]);
    f.instruction(&W::GlobalGet(root_stack_top_global_idx));
    f.instruction(&W::LocalSet(0));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Result(ValType::I64)));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::Else);
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::LocalSet(0));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::GlobalSet(root_stack_top_global_idx));
    f.instruction(&W::I32Const(root_stack_base));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I64Load(mem64(0)));
    f.instruction(&W::End);
    f.instruction(&W::End);
    codes.function(&f);
}

fn emit_root_set_func(
    codes: &mut CodeSection,
    root_stack_top_global_idx: u32,
    root_stack_base: i32,
) {
    use wasm_encoder::{Instruction as W, MemArg};

    let mem64 = |offset: u64| MemArg {
        offset,
        align: 3,
        memory_index: 0,
    };

    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32),
        (1, ValType::I32),
        (1, ValType::I32),
    ]);
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(2));
    f.instruction(&W::GlobalGet(root_stack_top_global_idx));
    f.instruction(&W::LocalSet(3));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32GeU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Unreachable);
    f.instruction(&W::End);
    f.instruction(&W::I32Const(root_stack_base));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I64Store(mem64(0)));
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

/// __print_string: ヒープ上 String オブジェクトを stdout に出力 (改行なし)
/// String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
fn emit_print_string_func(codes: &mut CodeSection) {
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

/// __int_to_string: i64 の値を10進文字列に変換してヒープに格納し、パック文字列を返す
/// __print_i64 と同じ数値→文字列変換ロジックだが、stdout ではなくヒープに書き込む
fn emit_int_to_string_func(codes: &mut CodeSection, alloc_func_idx: u32) {
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
    //         4=file_size(i32), 5=buf_addr(i32), 6=fd_read_errno(i32), 7=nread(i32),
    //         8=path_open_errno(i32), 9=fd_filestat_get_errno(i32)
    let mut f = wasm_encoder::Function::new(vec![
        (1, ValType::I32), // 1: path_offset (bytes の開始アドレス = path_addr + 8)
        (1, ValType::I32), // 2: path_len
        (1, ValType::I32), // 3: fd
        (1, ValType::I32), // 4: file_size
        (1, ValType::I32), // 5: buf_addr (String オブジェクトのアドレス)
        (1, ValType::I32), // 6: fd_read_errno
        (1, ValType::I32), // 7: nread
        (1, ValType::I32), // 8: path_open_errno
        (1, ValType::I32), // 9: fd_filestat_get_errno
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
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(2)); // path_len

    // fd を格納するスクラッチ領域 (アドレス 280)
    // path_open(dirfd=3, dirflags=0, path, path_len, oflags=0, rights_base, rights_inheriting, fdflags=0, fd_ptr)
    f.instruction(&W::I32Const(3)); // dirfd = 3 (preopened dir)
    f.instruction(&W::I32Const(0)); // dirflags = 0
    f.instruction(&W::LocalGet(1)); // path
    f.instruction(&W::LocalGet(2)); // path_len
    f.instruction(&W::I32Const(0)); // oflags = 0 (read only)
    f.instruction(&W::I64Const(0x42)); // rights_base = fd_read | fd_seek | fd_filestat_get
    f.instruction(&W::I64Const(0)); // rights_inheriting
    f.instruction(&W::I32Const(0)); // fdflags = 0
    f.instruction(&W::I32Const(280)); // fd_ptr (スクラッチ領域)
    f.instruction(&W::Call(path_open_idx));
    f.instruction(&W::LocalSet(8)); // path_open errno

    // open 失敗時は未初期化の fd を使わず、空文字列を返す。
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I64Const(8));
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(5));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    emit_tagged_pointer_from_i32_local(&mut f, 5);
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // fd を読み出し
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(3)); // fd

    // fd_filestat_get でファイルサイズ取得 (stat バッファは 288 から 64 バイト)
    f.instruction(&W::LocalGet(3)); // fd
    f.instruction(&W::I32Const(288)); // stat buf (288..352)
    f.instruction(&W::Call(fd_filestat_get_idx));
    f.instruction(&W::LocalSet(9)); // fd_filestat_get errno

    // stat 失敗時は開いた fd を閉じ、空文字列を返す。
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::LocalSet(8)); // close errno は結果を返さず fail-closed
    f.instruction(&W::I64Const(8));
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(5));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    emit_tagged_pointer_from_i32_local(&mut f, 5);
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // file_size = stat[32..40] の下位 32bit (filesize は offset 32 の i64)
    f.instruction(&W::I32Const(288));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 32,
        align: 2,
        memory_index: 0,
    })); // stat.st_size の下位 32bit
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
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    // len = file_size (後で nread に更新)
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));

    // iov を設定: iov[0].buf = buf_addr + 8, iov[0].len = file_size (スクラッチ 352)
    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    })); // iov.buf

    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    })); // iov.len

    // fd_read(fd, iov_ptr=352, iov_count=1, nread_ptr=360)
    f.instruction(&W::LocalGet(3)); // fd
    f.instruction(&W::I32Const(352)); // iovs
    f.instruction(&W::I32Const(1)); // iovs_len
    f.instruction(&W::I32Const(360)); // nread ptr
    f.instruction(&W::Call(fd_read_idx));
    f.instruction(&W::LocalSet(6)); // errno

    // nread を読み取り
    f.instruction(&W::I32Const(360));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(7)); // nread

    // fd_read errno は payload を公開せず fail-closed にする。
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::End);

    // fd_close
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::Drop);

    // String オブジェクトの len を nread に更新
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    // タグ付き String handle を返す
    emit_tagged_pointer_from_i32_local(&mut f, 5);

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
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
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
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(5)); // content_len

    // path_open(dirfd=3, dirflags=0, path, path_len, oflags=1(creat)|4(trunc), rights, 0, 0, fd_ptr=280)
    f.instruction(&W::I32Const(3)); // dirfd = 3
    f.instruction(&W::I32Const(0)); // dirflags
    f.instruction(&W::LocalGet(2)); // path
    f.instruction(&W::LocalGet(3)); // path_len
    f.instruction(&W::I32Const(5)); // oflags = O_CREAT(1) | O_TRUNC(4)
    f.instruction(&W::I64Const(0x40)); // rights_base = fd_write
    f.instruction(&W::I64Const(0)); // rights_inheriting
    f.instruction(&W::I32Const(0)); // fdflags
    f.instruction(&W::I32Const(280)); // fd_ptr
    f.instruction(&W::Call(path_open_idx));
    f.instruction(&W::LocalSet(3)); // path_open errno (path_len は以後不要)

    // open 失敗時は fd_write / fd_close を呼ばず、-1 を返す。
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I64ExtendI32S);
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // fd を読み出し
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(6)); // fd

    // iov 設定 (スクラッチ 352)
    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    })); // iov.buf

    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    })); // iov.len

    // fd_write(fd, iovs=352, iovs_len=1, nwritten_ptr=360)
    f.instruction(&W::LocalGet(6)); // fd
    f.instruction(&W::I32Const(352)); // iovs
    f.instruction(&W::I32Const(1)); // iovs_len
    f.instruction(&W::I32Const(360)); // nwritten
    f.instruction(&W::Call(fd_write_idx));
    f.instruction(&W::Drop);

    // nwritten を読み取り
    f.instruction(&W::I32Const(360));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(7));

    // fd_close の errno を path_len local に保存する。
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::LocalSet(3));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::End);

    // 書き込みバイト数を返す
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I64ExtendI32S);

    f.instruction(&W::End);
    codes.function(&f);
}

/// __write_file_bytes: Vector の各要素の下位 8 bit を raw bytes として書き込む。
///
/// 呼び出し元は path/vector を root stack に保持してからこの helper を呼ぶため、
/// packed buffer の確保中に GC が走っても入力オブジェクトは回収されない。
fn emit_write_file_bytes_func(
    codes: &mut CodeSection,
    alloc_func_idx: u32,
    path_open_idx: u32,
    fd_write_idx: u32,
    fd_close_idx: u32,
) {
    use wasm_encoder::{Instruction as W, MemArg};

    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };
    let mem64 = |offset: u64| MemArg {
        offset,
        align: 3,
        memory_index: 0,
    };

    // params: 0=path(i64), 1=Vector(i64)
    // locals: 2=path_offset, 3=path_len, 4=vector_addr, 5=vector_len,
    //         6=buffer_addr, 7=index, 8=fd, 9=nwritten (all i32)
    let mut f = wasm_encoder::Function::new(vec![(8, ValType::I32)]);

    // String path の bytes を取得する。
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(2));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(3));

    // Vector layout: [tag:i32, capacity:i32, length:i32, pad:i32, i64 elements...]
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Load(mem32(8)));
    f.instruction(&W::LocalSet(5));

    // Vector の i64 要素を packed bytes へ詰めるための一時バッファを確保する。
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(6));

    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));

    // buffer[index] = low_byte(vector[index])
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::I32Const(16));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::I64Load(mem64(0)));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Store8(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));

    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    // path_open(dirfd=3, dirflags=0, path, path_len, oflags=CREAT|TRUNC,
    //           rights=fd_write, rights_inheriting=0, fdflags=0, fd_ptr=280)
    f.instruction(&W::I32Const(3));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Const(5));
    f.instruction(&W::I64Const(0x40));
    f.instruction(&W::I64Const(0));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Const(280));
    f.instruction(&W::Call(path_open_idx));
    f.instruction(&W::LocalSet(3)); // path_open errno (path_len は以後不要)

    // open 失敗時は fd_write / fd_close を呼ばず、-1 を返す。
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::LocalSet(9));
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I64ExtendI32S);
    f.instruction(&W::Return);
    f.instruction(&W::End);

    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(8));

    // iovec = { buffer_addr, vector_len } at scratch 352.
    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Store(mem32(4)));

    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Const(352));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(360));
    f.instruction(&W::Call(fd_write_idx));
    f.instruction(&W::Drop);

    f.instruction(&W::I32Const(360));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(9));
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::LocalSet(9));
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I64ExtendI32S);
    f.instruction(&W::End);
    codes.function(&f);
}

/// __file_exists: String オブジェクトパスを受け取り、存在すれば 1、しなければ 0 を返す
fn emit_file_exists_func(codes: &mut CodeSection, path_open_idx: u32, fd_close_idx: u32) {
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
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(2)); // path_len

    // path_open(dirfd=3, 0, path, path_len, 0, rights, 0, 0, fd_ptr=280)
    f.instruction(&W::I32Const(3)); // dirfd = 3
    f.instruction(&W::I32Const(0)); // dirflags
    f.instruction(&W::LocalGet(1)); // path
    f.instruction(&W::LocalGet(2)); // path_len
    f.instruction(&W::I32Const(0)); // oflags = 0 (read)
    f.instruction(&W::I64Const(0x02)); // rights_base = fd_read
    f.instruction(&W::I64Const(0)); // rights_inheriting
    f.instruction(&W::I32Const(0)); // fdflags
    f.instruction(&W::I32Const(280)); // fd_ptr
    f.instruction(&W::Call(path_open_idx));
    f.instruction(&W::LocalSet(3)); // errno

    // errno == 0 → ファイル存在、fd_close して 1 を返す
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    // fd_close
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
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
fn emit_command_line_args_func(codes: &mut CodeSection, args_sizes_get_idx: u32) {
    use wasm_encoder::Instruction as W;

    // locals: なし (スクラッチ領域を使用)
    let mut f = wasm_encoder::Function::new(vec![]);

    // args_sizes_get(argc_ptr=280, argv_buf_size_ptr=284)
    f.instruction(&W::I32Const(280)); // argc ptr
    f.instruction(&W::I32Const(284)); // argv_buf_size ptr
    f.instruction(&W::Call(args_sizes_get_idx));
    f.instruction(&W::Drop); // errno

    // argc を読み取って返す
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::I64ExtendI32U);

    f.instruction(&W::End);
    codes.function(&f);
}

/// __command_line_arg: 指定 index のコマンドライン引数を String オブジェクトで返す
fn emit_command_line_arg_func(
    codes: &mut CodeSection,
    alloc_func_idx: u32,
    args_get_idx: u32,
    args_sizes_get_idx: u32,
) {
    use wasm_encoder::Instruction as W;

    // locals:
    // 1=index_i32 2=argc 3=argv_buf_size 4=argv_ptr 5=argv_buf
    // 6=arg_ptr 7=scan_ptr 8=arg_len 9=str_ptr 10=i
    let mut f = wasm_encoder::Function::new(vec![
        (8, ValType::I32),
        (1, ValType::I64),
        (1, ValType::I32),
    ]);

    // index_i32 = i32.wrap_i64(index)
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(1));

    // args_sizes_get(argc_ptr=280, argv_buf_size_ptr=284)
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Const(284));
    f.instruction(&W::Call(args_sizes_get_idx));
    f.instruction(&W::Drop);

    // argc / argv_buf_size
    f.instruction(&W::I32Const(280));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(2));
    f.instruction(&W::I32Const(284));
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(3));

    // index < 0 -> empty string
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32LtS);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I64Const(8));
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::LocalTee(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    emit_tagged_pointer_from_i64_local(&mut f, 9);
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // index >= argc -> empty string
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32GeS);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I64Const(8));
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::LocalTee(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));
    emit_tagged_pointer_from_i64_local(&mut f, 9);
    f.instruction(&W::Return);
    f.instruction(&W::End);

    // argv_ptr = __alloc(argc * 4)
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Mul);
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(4));

    // argv_buf = __alloc(argv_buf_size)
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(5));

    // args_get(argv_ptr, argv_buf)
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::Call(args_get_idx));
    f.instruction(&W::Drop);

    // arg_ptr = i32.load(argv_ptr + index * 4)
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Mul);
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Load(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalSet(6));

    // scan_ptr = arg_ptr, arg_len = 0
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(8));

    // nul 終端まで長さを数える
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Load8U(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(7));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(8));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    // str_ptr = __alloc(8 + arg_len)
    f.instruction(&W::I32Const(8));
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::LocalTee(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 0,
        align: 2,
        memory_index: 0,
    }));
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Store(wasm_encoder::MemArg {
        offset: 4,
        align: 2,
        memory_index: 0,
    }));

    // i = 0
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(10));

    // bytes を String object に copy
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(10));
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(9));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(10));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::LocalGet(10));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Load8U(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    f.instruction(&W::I32Store8(wasm_encoder::MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));
    f.instruction(&W::LocalGet(10));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(10));
    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    emit_tagged_pointer_from_i64_local(&mut f, 9);
    f.instruction(&W::End);
    codes.function(&f);
}

/// __read_stdin: stdin(fd=0) を 4KiB chunk で EOF まで繰り返し読み、String object を返す
fn emit_read_stdin_func(
    codes: &mut CodeSection,
    alloc_func_idx: u32,
    string_concat_idx: u32,
    fd_read_idx: u32,
) {
    use wasm_encoder::Instruction as W;
    use wasm_encoder::MemArg;

    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

    // locals: 0=result_addr(i32), 1=chunk_addr(i32), 2=nread(i32)
    let mut f = wasm_encoder::Function::new(vec![(3, ValType::I32)]);

    // 空文字列を初期値にする
    f.instruction(&W::I64Const(8));
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(0));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(mem32(4)));

    // 読み込み chunk は再利用する
    f.instruction(&W::I64Const(4104));
    f.instruction(&W::Call(alloc_func_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(mem32(4)));

    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));

    f.instruction(&W::I32Const(352));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::I32Const(352));
    f.instruction(&W::I32Const(4096));
    f.instruction(&W::I32Store(mem32(4)));

    f.instruction(&W::I32Const(360));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Store(mem32(0)));

    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Const(352));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Const(360));
    f.instruction(&W::Call(fd_read_idx));
    f.instruction(&W::Drop);

    f.instruction(&W::I32Const(360));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(2));

    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::BrIf(1));

    f.instruction(&W::LocalGet(1));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::I32Store(mem32(4)));

    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I64ExtendI32U);
    f.instruction(&W::Call(string_concat_idx));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::LocalSet(0));

    f.instruction(&W::Br(0));
    f.instruction(&W::End);
    f.instruction(&W::End);

    emit_tagged_pointer_from_i32_local(&mut f, 0);
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

/// IR 命令を WASI 用にリマップして出力
#[derive(Debug, Clone, Copy)]
struct WasiStructScratch {
    field_base: u32,
    ptr_local: u32,
    addr_local: u32,
}

fn max_struct_field_count(module: &Module) -> u32 {
    module
        .gc_types
        .iter()
        .filter_map(|ty| match &ty.kind {
            GcTypeKind::Struct(fields) => Some(fields.len() as u32),
            GcTypeKind::Array(_) => None,
        })
        .max()
        .unwrap_or(0)
        .max(1)
}

fn struct_field_count(
    gc_types: &[lsharp_ir::GcTypeDef],
    type_index: u32,
) -> Result<u32, CodegenError> {
    let Some(gc_type) = gc_types.get(type_index as usize) else {
        return Err(CodegenError::Error {
            msg: format!("struct type index out of bounds: {type_index}"),
        });
    };
    match &gc_type.kind {
        GcTypeKind::Struct(fields) => Ok(fields.len() as u32),
        GcTypeKind::Array(_) => Err(CodegenError::Error {
            msg: format!(
                "array GC type cannot be emitted as linear-memory struct: {}",
                gc_type.name
            ),
        }),
    }
}

fn emit_wasi_struct_instruction(
    func: &mut wasm_encoder::Function,
    instruction: &Instruction,
    gc_types: &[lsharp_ir::GcTypeDef],
    alloc_func_idx: u32,
    scratch: WasiStructScratch,
) -> Result<bool, CodegenError> {
    use wasm_encoder::{Instruction as W, MemArg};

    let mem64 = |offset: u64| MemArg {
        offset,
        align: 3,
        memory_index: 0,
    };

    match instruction {
        Instruction::StructNew(type_index) => {
            let field_count = struct_field_count(gc_types, *type_index)?;
            for field_index in (0..field_count).rev() {
                func.instruction(&W::LocalSet(scratch.field_base + field_index));
            }
            func.instruction(&W::I64Const(i64::from(field_count * 8)));
            func.instruction(&W::Call(alloc_func_idx));
            func.instruction(&W::LocalTee(scratch.ptr_local));
            func.instruction(&W::I32WrapI64);
            func.instruction(&W::LocalSet(scratch.addr_local));
            for field_index in 0..field_count {
                func.instruction(&W::LocalGet(scratch.addr_local));
                func.instruction(&W::LocalGet(scratch.field_base + field_index));
                func.instruction(&W::I64Store(mem64(u64::from(field_index * 8))));
            }
            func.instruction(&W::LocalGet(scratch.ptr_local));
            Ok(true)
        }
        Instruction::StructGet(type_index, field_index) => {
            let field_count = struct_field_count(gc_types, *type_index)?;
            if *field_index >= field_count {
                return Err(CodegenError::Error {
                    msg: format!(
                        "struct field index out of bounds: type={type_index} field={field_index}"
                    ),
                });
            }
            func.instruction(&W::I32WrapI64);
            func.instruction(&W::I64Load(mem64(u64::from(field_index * 8))));
            Ok(true)
        }
        Instruction::StructSet(type_index, field_index) => {
            let field_count = struct_field_count(gc_types, *type_index)?;
            if *field_index >= field_count {
                return Err(CodegenError::Error {
                    msg: format!(
                        "struct field index out of bounds: type={type_index} field={field_index}"
                    ),
                });
            }
            func.instruction(&W::LocalSet(scratch.field_base));
            func.instruction(&W::I32WrapI64);
            func.instruction(&W::LocalGet(scratch.field_base));
            func.instruction(&W::I64Store(mem64(u64::from(field_index * 8))));
            Ok(true)
        }
        _ => Ok(false),
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_instructions_wasi(
    func: &mut wasm_encoder::Function,
    instructions: &[Instruction],
    gc_types: &[lsharp_ir::GcTypeDef],
    scratch: WasiStructScratch,
    print_helper_idx: u32,
    alloc_func_idx: u32,
    string_concat_idx: u32,
    string_eq_idx: u32,
    print_string_idx: u32,
    proc_exit_wasm_idx: u32,
    int_to_string_idx: u32,
    read_file_idx: u32,
    write_file_idx: u32,
    write_file_bytes_idx: Option<u32>,
    file_exists_idx: u32,
    command_line_args_idx: u32,
    command_line_arg_idx: u32,
    read_stdin_idx: u32,
    fnv1a_hash_idx: u32,
    root_push_idx: u32,
    root_pop_idx: u32,
    root_set_idx: u32,
    user_func_base: u32,
    call_indirect_type_map: &HashMap<u32, u32>,
) -> Result<(), CodegenError> {
    use wasm_encoder::Instruction as W;

    // CallIndirect の型インデックスと FuncIdx をリマップした命令列を作成
    let remapped: Vec<Instruction> = instructions
        .iter()
        .map(|instr| {
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
                        11 => command_line_arg_idx,
                        12 => read_stdin_idx,
                        13 => fnv1a_hash_idx,
                        14 => root_push_idx,
                        15 => root_pop_idx,
                        16 => root_set_idx,
                        i => user_func_base + (i - IR_IMPORT_COUNT),
                    };
                    Instruction::FuncIdx(wasm_idx)
                }
                _ => instr.clone(),
            }
        })
        .collect();

    crate::emit::emit_instructions_common_with_handler(
        func,
        &remapped,
        |f, i| {
            match i {
                0 => {
                    f.instruction(&W::Call(print_helper_idx));
                }
                1 => {
                    f.instruction(&W::Call(alloc_func_idx));
                }
                2 => {
                    f.instruction(&W::Call(string_concat_idx));
                }
                3 => {
                    f.instruction(&W::Call(string_eq_idx));
                }
                4 => {
                    f.instruction(&W::Call(print_string_idx));
                }
                5 => {
                    f.instruction(&W::Call(proc_exit_wasm_idx));
                }
                6 => {
                    f.instruction(&W::Call(int_to_string_idx));
                }
                7 => {
                    f.instruction(&W::Call(read_file_idx));
                }
                8 => {
                    f.instruction(&W::Call(write_file_idx));
                }
                9 => {
                    f.instruction(&W::Call(file_exists_idx));
                }
                10 => {
                    f.instruction(&W::Call(command_line_args_idx));
                }
                11 => {
                    f.instruction(&W::Call(command_line_arg_idx));
                }
                12 => {
                    f.instruction(&W::Call(read_stdin_idx));
                }
                13 => {
                    f.instruction(&W::Call(fnv1a_hash_idx));
                }
                14 => {
                    f.instruction(&W::Call(root_push_idx));
                }
                15 => {
                    f.instruction(&W::Call(root_pop_idx));
                }
                16 => {
                    f.instruction(&W::Call(root_set_idx));
                }
                _ => {
                    f.instruction(&W::Call(user_func_base + (i - IR_IMPORT_COUNT)));
                }
            }
            Ok(())
        },
        |f, instruction| {
            if matches!(instruction, Instruction::WriteFileBytes) {
                let helper_idx = write_file_bytes_idx.ok_or_else(|| CodegenError::Error {
                    msg: "write-file-bytes はこの target では未対応です".to_string(),
                })?;
                f.instruction(&W::Call(helper_idx));
                return Ok(true);
            }

            emit_wasi_struct_instruction(f, instruction, gc_types, alloc_func_idx, scratch)
        },
    )
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

    fn compile_wasi_p2(source: &str) -> Vec<u8> {
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lower = Lower::new();
        let module = lower.lower_program(&program, &type_results).unwrap();
        emit_wasm_wasi_p2(&module).unwrap()
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

        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .unwrap();
        start.call(&mut store, ()).unwrap();

        drop(store);
        let bytes = stdout.try_into_inner().unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    fn assert_close_errno_is_saved(wasm_bytes: &[u8], code_ordinal: usize) {
        use wasmparser::{Operator, Parser, Payload};

        let mut ordinal = 0usize;
        let mut found_saved_errno = false;
        let mut found_dropped_errno = false;
        for payload in Parser::new(0).parse_all(wasm_bytes) {
            let payload = payload.expect("Wasm payload の読み取りに失敗");
            let Payload::CodeSectionEntry(body) = payload else {
                continue;
            };
            if ordinal != code_ordinal {
                ordinal += 1;
                continue;
            }

            let mut close_call = false;
            for operator in body
                .get_operators_reader()
                .expect("helper body の operator reader 作成に失敗")
            {
                match operator.expect("helper body の operator 読み取りに失敗") {
                    Operator::Call { function_index: 5 } => close_call = true,
                    Operator::LocalSet { .. } if close_call => {
                        found_saved_errno = true;
                        close_call = false;
                    }
                    Operator::Drop if close_call => {
                        found_dropped_errno = true;
                        close_call = false;
                    }
                    _ => close_call = false,
                }
            }
            break;
        }

        assert!(
            found_saved_errno,
            "fd_close errno を local へ保存する必要がある"
        );
        assert!(
            !found_dropped_errno,
            "fd_close errno を drop してはいけない"
        );
    }

    fn assert_fd_read_errno_is_saved(wasm_bytes: &[u8]) {
        use wasmparser::{Operator, Parser, Payload};

        let mut found_saved_errno = false;
        for payload in Parser::new(0).parse_all(wasm_bytes) {
            let payload = payload.expect("Wasm payload の読み取りに失敗");
            let Payload::CodeSectionEntry(body) = payload else {
                continue;
            };

            let mut read_call = false;
            for operator in body
                .get_operators_reader()
                .expect("helper body の operator reader 作成に失敗")
            {
                match operator.expect("helper body の operator 読み取りに失敗") {
                    Operator::Call { function_index: 4 } => read_call = true,
                    Operator::LocalSet { .. } if read_call => {
                        found_saved_errno = true;
                        read_call = false;
                    }
                    _ => {}
                }
            }
        }

        assert!(
            found_saved_errno,
            "fd_read errno を local へ保存する必要がある"
        );
    }

    fn assert_call_result_is_saved(
        wasm_bytes: &[u8],
        code_ordinal: usize,
        function_index: u32,
        result_name: &str,
    ) {
        use wasmparser::{Operator, Parser, Payload};

        let mut ordinal = 0usize;
        let mut found_saved_result = false;
        let mut found_dropped_result = false;
        for payload in Parser::new(0).parse_all(wasm_bytes) {
            let payload = payload.expect("Wasm payload の読み取りに失敗");
            let Payload::CodeSectionEntry(body) = payload else {
                continue;
            };
            if ordinal != code_ordinal {
                ordinal += 1;
                continue;
            }

            let mut call_result_pending = false;
            for operator in body
                .get_operators_reader()
                .expect("helper body の operator reader 作成に失敗")
            {
                match operator.expect("helper body の operator 読み取りに失敗") {
                    Operator::Call {
                        function_index: current_index,
                    } if current_index == function_index => call_result_pending = true,
                    Operator::LocalSet { .. } if call_result_pending => {
                        found_saved_result = true;
                        call_result_pending = false;
                    }
                    Operator::Drop if call_result_pending => {
                        found_dropped_result = true;
                        call_result_pending = false;
                    }
                    _ => call_result_pending = false,
                }
            }
            break;
        }

        assert!(
            found_saved_result,
            "{result_name} の errno を local へ保存する必要がある"
        );
        assert!(
            !found_dropped_result,
            "{result_name} の errno を drop してはいけない"
        );
    }

    #[test]
    fn test_wasi_write_helpers_preserve_fd_close_errno() {
        let wasm = compile_wasi(
            r#"
            (defn main []
              (do
                (write-file "output.txt" "payload")
                (let [bytes (vector-push (vector-new 1) 97)]
                  (write-file-bytes "raw.bin" bytes))
                0))
            "#,
        );
        assert_close_errno_is_saved(&wasm, 7);
        assert_close_errno_is_saved(&wasm, 16);
    }

    #[test]
    fn test_wasi_read_file_preserves_fd_read_errno() {
        let wasm = compile_wasi(r#"(defn main [] (print-string (read-file "input.txt")))"#);
        assert_fd_read_errno_is_saved(&wasm);
    }

    #[test]
    fn test_wasi_file_helpers_preserve_path_open_errno() {
        let wasm = compile_wasi(
            r#"
            (defn main []
              (do
                (read-file "input.txt")
                (write-file "output.txt" "payload")
                (let [bytes (vector-push (vector-new 1) 97)]
                  (write-file-bytes "raw.bin" bytes))
                0))
            "#,
        );
        assert_call_result_is_saved(&wasm, 6, 6, "read-file path_open");
        assert_call_result_is_saved(&wasm, 6, 8, "read-file fd_filestat_get");
        assert_call_result_is_saved(&wasm, 7, 6, "write-file path_open");
        assert_call_result_is_saved(&wasm, 16, 6, "write-file-bytes path_open");
    }

    #[test]
    fn test_wasi_file_helpers_fail_closed_on_path_open_errno() {
        let wasm = compile_wasi(
            r#"
            (defn main []
              (let [bytes (vector-push (vector-new 1) 97)]
                (do
                  (print-string (read-file "input.txt"))
                  (print (write-file "output.txt" "payload"))
                  (print (write-file-bytes "raw.bin" bytes))
                  0)))
            "#,
        );
        assert_eq!(run_wasi(&wasm), "-1\n-1\n");
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
    fn test_wasi_write_file_bytes_writes_vector_low_bytes() {
        let wasm = compile_wasi(
            r#"
            (defn main []
              (let [bytes (vector-push
                            (vector-push
                              (vector-push
                                (vector-push (vector-new 4) 0)
                                97)
                              115)
                            109)]
                (write-file-bytes "raw.wasm" bytes)))
            "#,
        );
        let dir = std::env::temp_dir().join(format!(
            "lsharp_wasi_write_file_bytes_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");

        let result = (|| {
            let output = crate::wasi_runner::run_wasm_wasi_with_dir(&wasm, Some(&dir))
                .expect("write-file-bytes program の実行に失敗");
            assert_eq!(output, "");
            assert_eq!(
                std::fs::read(dir.join("raw.wasm")).expect("raw.wasm の読み込みに失敗"),
                b"\0asm"
            );
        })();
        let _ = std::fs::remove_dir_all(&dir);
        result
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
    fn test_wasi_record_access_uses_linear_memory_fallback() {
        let wasm = compile_wasi(
            "(type Point (record (: x Int) (: y Int)))
             (defn make-point [x y] {Point x x y y})
             (defn main [] (print (Point.x (make-point 10 20))))",
        );
        assert_eq!(run_wasi(&wasm), "10\n");
    }

    #[test]
    fn test_emit_wasm_wasi_p2_basic_program_compiles() {
        let component = compile_wasi_p2("(defn main [] (print 42))");
        assert!(component.len() > 8);
        assert_eq!(&component[0..4], b"\0asm");

        let engine = wasmtime::Engine::default();
        wasmtime::component::Component::new(&engine, &component)
            .expect("P2 entrypoint は valid component を生成するべき");
    }

    #[test]
    fn test_emit_wasm_wasi_p2_runs_print_via_component_runner() {
        let component = compile_wasi_p2("(defn main [] (print 42))");

        let output = crate::wasi_runner::run_wasm_component(&component)
            .expect("P2 component は preview2 runner で実行できるべき");
        assert_eq!(output, "42\n");
    }

    #[test]
    fn test_emit_wasm_wasi_p2_supports_stdin_and_args() {
        let component = compile_wasi_p2(
            r#"
            (defn main []
              (do
                (print-string (command-line-arg 0))
                (print-string ":")
                (print-string (read-stdin))
                0))
            "#,
        );

        let output = crate::wasi_runner::run_wasm_component_with_args_and_stdin(
            &component,
            &["alpha"],
            "stdin-smoke",
        )
        .expect("P2 component は argv/stdin bridge を使えるべき");
        assert_eq!(output, "alpha:stdin-smoke");
    }

    #[test]
    fn test_emit_wasm_wasi_p2_supports_large_stdout_write() {
        let payload = "x".repeat(4097);
        let component = compile_wasi_p2(&format!(
            r#"
            (defn main []
              (do
                (print-string "{payload}")
                0))
            "#
        ));

        let output = crate::wasi_runner::run_wasm_component(&component)
            .expect("P2 component は 4KiB 超の stdout write を処理できるべき");
        assert_eq!(output, payload);
    }

    #[test]
    fn test_emit_wasm_wasi_p2_supports_file_roundtrip() {
        let component = compile_wasi_p2(
            r#"
            (defn main []
              (do
                (write-file "roundtrip.txt" "hello component")
                (print-string (read-file "roundtrip.txt"))
                0))
            "#,
        );

        let dir = std::env::temp_dir().join("lsharp_wasi_p2_file_roundtrip");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let output = crate::wasi_runner::run_wasm_component_with_dir_args_and_stdin(
            &component,
            Some(&dir),
            &[],
            "",
        )
        .expect("P2 component は preview2 filesystem bridge 経由で file roundtrip できるべき");
        assert_eq!(output, "hello component");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_wasi_proc_exit_type_check() {
        // proc-exit が型チェックを通ること (Int -> Unit)
        let source = "(defn main [] (do (proc-exit 0) 0))";
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let result = infer.infer_program(&program);
        assert!(
            result.is_ok(),
            "proc-exit の型チェックが失敗: {:?}",
            result.err()
        );
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

        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .unwrap();
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
        assert_eq!(
            imports.len(),
            9,
            "WASI import 数が 9 でない: {:?}",
            imports
                .iter()
                .map(|i| i.name().to_string())
                .collect::<Vec<_>>()
        );

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
            eprintln!(
                "func[{}] = {} ({} params, {} locals)",
                i,
                f.name,
                f.params.len(),
                f.locals.len()
            );
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
