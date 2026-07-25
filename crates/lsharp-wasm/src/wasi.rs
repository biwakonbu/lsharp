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

mod argv;
#[cfg(test)]
mod argv_tests;
mod file_exists;
#[cfg(test)]
mod file_exists_tests;
mod hash;
#[cfg(test)]
mod hash_tests;
mod root;
#[cfg(test)]
mod root_tests;
mod stdin;
#[cfg(test)]
mod stdin_tests;
mod write_file_bytes;
#[cfg(test)]
mod write_file_bytes_tests;

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
/// 小さい allocation はサイズクラスの singly-linked free-list で再利用する。
/// 最後の class (index 7) は 1024 bytes 超の oversize fallback である。
const GC_FREE_CLASS_COUNT: i32 = 8;
const GC_FREE_CLASS_HEAD_GLOBAL_BASE_IDX: u32 = 17;
const GC_FREE_LIST_SCAN_STEPS_GLOBAL_IDX: u32 = 25;
const GC_FREE_CLASS_LIMITS: [i32; 7] = [16, 32, 64, 128, 256, 512, 1024];
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
const GC_OBJECT_TABLE_BASE_GLOBAL_IDX: u32 = 8;
const GC_OBJECT_TABLE_CAPACITY_GLOBAL_IDX: u32 = 9;
const GC_FREE_LIST_BASE_GLOBAL_IDX: u32 = 10;
const GC_FREE_LIST_CAPACITY_GLOBAL_IDX: u32 = 11;
const ROOT_STACK_BASE_GLOBAL_IDX: u32 = 12;
const ROOT_STACK_CAPACITY_GLOBAL_IDX: u32 = 13;
const ROOT_SLOT_FAILURE_SLOT_GLOBAL_IDX: u32 = 14;
const ROOT_SLOT_FAILURE_TOP_GLOBAL_IDX: u32 = 15;
const ROOT_SLOT_FAILURE_COUNT_GLOBAL_IDX: u32 = 16;
const INTERNAL_HEAP_PTR_EXPORT: &str = "__lsharp_heap_ptr";
const INTERNAL_HEAP_START_EXPORT: &str = "__lsharp_heap_start";
const INTERNAL_ALLOC_COUNT_EXPORT: &str = "__lsharp_alloc_count";
const INTERNAL_ROOT_STACK_TOP_EXPORT: &str = "__lsharp_root_stack_top";
const INTERNAL_ROOT_STACK_BASE_EXPORT: &str = "__lsharp_root_stack_base";
const INTERNAL_ROOT_STACK_CAPACITY_EXPORT: &str = "__lsharp_root_stack_capacity";
const INTERNAL_ROOT_SLOT_FAILURE_SLOT_EXPORT: &str = "__lsharp_root_slot_failure_slot";
const INTERNAL_ROOT_SLOT_FAILURE_TOP_EXPORT: &str = "__lsharp_root_slot_failure_top";
const INTERNAL_ROOT_SLOT_FAILURE_COUNT_EXPORT: &str = "__lsharp_root_slot_failure_count";
const INTERNAL_GC_LIVE_ALLOC_COUNT_EXPORT: &str = "__lsharp_gc_live_alloc_count";
const INTERNAL_GC_FREE_LIST_COUNT_EXPORT: &str = "__lsharp_gc_free_list_count";
const INTERNAL_GC_COLLECTION_COUNT_EXPORT: &str = "__lsharp_gc_collection_count";
const INTERNAL_GC_FREED_COUNT_EXPORT: &str = "__lsharp_gc_freed_count";
const INTERNAL_GC_FREE_LIST_SCAN_STEPS_EXPORT: &str = "__lsharp_gc_free_list_scan_steps";
const INTERNAL_GC_OBJECT_CAPACITY_EXPORT: &str = "__lsharp_gc_object_capacity";
const INTERNAL_GC_COLLECT_EXPORT: &str = "__lsharp_gc_collect";

#[derive(Copy, Clone)]
struct AllocatorGlobals {
    heap_ptr_global_idx: u32,
    alloc_count_global_idx: u32,
    object_count_global_idx: u32,
    free_list_count_global_idx: u32,
    free_list_base_global_idx: u32,
    object_table_base_global_idx: u32,
    object_table_capacity_global_idx: u32,
    free_class_heads_base_global_idx: u32,
    free_list_scan_steps_global_idx: u32,
}

#[derive(Copy, Clone)]
struct CollectorGlobals {
    heap_ptr_global_idx: u32,
    heap_start_global_idx: u32,
    root_stack_top_global_idx: u32,
    root_stack_base_global_idx: u32,
    object_count_global_idx: u32,
    free_list_count_global_idx: u32,
    free_list_base_global_idx: u32,
    free_list_capacity_global_idx: u32,
    object_table_base_global_idx: u32,
    gc_collection_count_global_idx: u32,
    gc_freed_count_global_idx: u32,
    free_class_heads_base_global_idx: u32,
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

/// `memory.grow` failure を起こしうる core WASI runtime helper の関数 index。
///
/// ユーザー関数は `WASI_IMPORT_COUNT + 18` 以降から始まるため、これらの index は
/// allocator/root capacity trap の classifier が user code の `unreachable` と
/// 区別するために使える。
pub(crate) const CAPACITY_FAILURE_FUNCTION_INDICES: [u32; 2] = [
    WASI_IMPORT_COUNT + 1,  // __alloc
    WASI_IMPORT_COUNT + 13, // root_push
];

/// GC-safe-point の root slot 更新失敗を識別する Wasm helper function index。
pub(crate) const ROOT_SLOT_INVARIANT_FUNCTION_INDICES: [u32; 1] = [
    WASI_IMPORT_COUNT + 15, // root_set
];

pub(super) fn emit_tagged_pointer_from_i32_local(
    func: &mut wasm_encoder::Function,
    local_idx: u32,
) {
    use wasm_encoder::Instruction as W;

    func.instruction(&W::LocalGet(local_idx));
    func.instruction(&W::I64ExtendI32U);
    func.instruction(&W::I64Const(TAGGED_POINTER_MASK));
    func.instruction(&W::I64Add);
}

pub(super) fn emit_tagged_pointer_from_i64_local(
    func: &mut wasm_encoder::Function,
    local_idx: u32,
) {
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
        free_list_base_global_idx: GC_FREE_LIST_BASE_GLOBAL_IDX,
        object_table_base_global_idx: GC_OBJECT_TABLE_BASE_GLOBAL_IDX,
        object_table_capacity_global_idx: GC_OBJECT_TABLE_CAPACITY_GLOBAL_IDX,
        free_class_heads_base_global_idx: GC_FREE_CLASS_HEAD_GLOBAL_BASE_IDX,
        free_list_scan_steps_global_idx: GC_FREE_LIST_SCAN_STEPS_GLOBAL_IDX,
    };
    let collector_globals = CollectorGlobals {
        heap_ptr_global_idx: HEAP_PTR_GLOBAL_IDX,
        heap_start_global_idx: HEAP_START_GLOBAL_IDX,
        root_stack_top_global_idx: ROOT_STACK_TOP_GLOBAL_IDX,
        root_stack_base_global_idx: ROOT_STACK_BASE_GLOBAL_IDX,
        object_count_global_idx: GC_OBJECT_COUNT_GLOBAL_IDX,
        free_list_count_global_idx: GC_FREE_LIST_COUNT_GLOBAL_IDX,
        free_list_base_global_idx: GC_FREE_LIST_BASE_GLOBAL_IDX,
        free_list_capacity_global_idx: GC_FREE_LIST_CAPACITY_GLOBAL_IDX,
        object_table_base_global_idx: GC_OBJECT_TABLE_BASE_GLOBAL_IDX,
        gc_collection_count_global_idx: GC_COLLECTION_COUNT_GLOBAL_IDX,
        gc_freed_count_global_idx: GC_FREED_COUNT_GLOBAL_IDX,
        free_class_heads_base_global_idx: GC_FREE_CLASS_HEAD_GLOBAL_BASE_IDX,
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
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(gc_object_table_base),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(GC_OBJECT_SLOT_CAPACITY),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(gc_free_list_base),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(GC_FREE_LIST_SLOT_CAPACITY),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(root_stack_base),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(ROOT_STACK_SLOT_CAPACITY),
    );
    for _ in 0..3 {
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &wasm_encoder::ConstExpr::i32_const(0),
        );
    }
    for _ in 0..GC_FREE_CLASS_COUNT {
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &wasm_encoder::ConstExpr::i32_const(0),
        );
    }
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(0),
    );
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
        INTERNAL_ROOT_STACK_BASE_EXPORT,
        ExportKind::Global,
        ROOT_STACK_BASE_GLOBAL_IDX,
    );
    exports.export(
        INTERNAL_ROOT_STACK_CAPACITY_EXPORT,
        ExportKind::Global,
        ROOT_STACK_CAPACITY_GLOBAL_IDX,
    );
    exports.export(
        INTERNAL_ROOT_SLOT_FAILURE_SLOT_EXPORT,
        ExportKind::Global,
        ROOT_SLOT_FAILURE_SLOT_GLOBAL_IDX,
    );
    exports.export(
        INTERNAL_ROOT_SLOT_FAILURE_TOP_EXPORT,
        ExportKind::Global,
        ROOT_SLOT_FAILURE_TOP_GLOBAL_IDX,
    );
    exports.export(
        INTERNAL_ROOT_SLOT_FAILURE_COUNT_EXPORT,
        ExportKind::Global,
        ROOT_SLOT_FAILURE_COUNT_GLOBAL_IDX,
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
    exports.export(
        INTERNAL_GC_FREE_LIST_SCAN_STEPS_EXPORT,
        ExportKind::Global,
        GC_FREE_LIST_SCAN_STEPS_GLOBAL_IDX,
    );
    exports.export(
        INTERNAL_GC_OBJECT_CAPACITY_EXPORT,
        ExportKind::Global,
        GC_OBJECT_TABLE_CAPACITY_GLOBAL_IDX,
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
    emit_alloc_func(&mut codes, allocator_globals);
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
    file_exists::emit_file_exists_func(&mut codes, path_open_idx, fd_close_idx);
    argv::emit_command_line_args_func(&mut codes, args_sizes_get_idx);
    argv::emit_command_line_arg_func(&mut codes, alloc_func_idx, args_get_idx, args_sizes_get_idx);
    stdin::emit_read_stdin_func(&mut codes, alloc_func_idx, string_concat_idx, fd_read_idx);
    hash::emit_fnv1a_hash_func(&mut codes);
    root::emit_root_push_func(
        &mut codes,
        HEAP_PTR_GLOBAL_IDX,
        ROOT_STACK_TOP_GLOBAL_IDX,
        ROOT_STACK_BASE_GLOBAL_IDX,
        ROOT_STACK_CAPACITY_GLOBAL_IDX,
    );
    root::emit_root_pop_func(
        &mut codes,
        ROOT_STACK_TOP_GLOBAL_IDX,
        ROOT_STACK_BASE_GLOBAL_IDX,
    );
    root::emit_root_set_func(
        &mut codes,
        ROOT_STACK_TOP_GLOBAL_IDX,
        ROOT_STACK_BASE_GLOBAL_IDX,
        ROOT_SLOT_FAILURE_SLOT_GLOBAL_IDX,
        ROOT_SLOT_FAILURE_TOP_GLOBAL_IDX,
        ROOT_SLOT_FAILURE_COUNT_GLOBAL_IDX,
    );
    write_file_bytes::emit_write_file_bytes_func(
        &mut codes,
        alloc_func_idx,
        path_open_idx,
        fd_write_idx,
        fd_close_idx,
    );
    emit_gc_collect_func(&mut codes, collector_globals);

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
        free_list_base_global_idx: GC_FREE_LIST_BASE_GLOBAL_IDX,
        object_table_base_global_idx: GC_OBJECT_TABLE_BASE_GLOBAL_IDX,
        object_table_capacity_global_idx: GC_OBJECT_TABLE_CAPACITY_GLOBAL_IDX,
        free_class_heads_base_global_idx: GC_FREE_CLASS_HEAD_GLOBAL_BASE_IDX,
        free_list_scan_steps_global_idx: GC_FREE_LIST_SCAN_STEPS_GLOBAL_IDX,
    };
    let collector_globals = CollectorGlobals {
        heap_ptr_global_idx: HEAP_PTR_GLOBAL_IDX,
        heap_start_global_idx: HEAP_START_GLOBAL_IDX,
        root_stack_top_global_idx: ROOT_STACK_TOP_GLOBAL_IDX,
        root_stack_base_global_idx: ROOT_STACK_BASE_GLOBAL_IDX,
        object_count_global_idx: GC_OBJECT_COUNT_GLOBAL_IDX,
        free_list_count_global_idx: GC_FREE_LIST_COUNT_GLOBAL_IDX,
        free_list_base_global_idx: GC_FREE_LIST_BASE_GLOBAL_IDX,
        free_list_capacity_global_idx: GC_FREE_LIST_CAPACITY_GLOBAL_IDX,
        object_table_base_global_idx: GC_OBJECT_TABLE_BASE_GLOBAL_IDX,
        gc_collection_count_global_idx: GC_COLLECTION_COUNT_GLOBAL_IDX,
        gc_freed_count_global_idx: GC_FREED_COUNT_GLOBAL_IDX,
        free_class_heads_base_global_idx: GC_FREE_CLASS_HEAD_GLOBAL_BASE_IDX,
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
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(gc_object_table_base),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(GC_OBJECT_SLOT_CAPACITY),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(gc_free_list_base),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(GC_FREE_LIST_SLOT_CAPACITY),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(root_stack_base),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(ROOT_STACK_SLOT_CAPACITY),
    );
    for _ in 0..3 {
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &wasm_encoder::ConstExpr::i32_const(0),
        );
    }
    for _ in 0..GC_FREE_CLASS_COUNT {
        globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &wasm_encoder::ConstExpr::i32_const(0),
        );
    }
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &wasm_encoder::ConstExpr::i32_const(0),
    );
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
    emit_alloc_func(&mut codes, allocator_globals);
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
    hash::emit_fnv1a_hash_func(&mut codes);
    root::emit_root_push_func(
        &mut codes,
        HEAP_PTR_GLOBAL_IDX,
        ROOT_STACK_TOP_GLOBAL_IDX,
        ROOT_STACK_BASE_GLOBAL_IDX,
        ROOT_STACK_CAPACITY_GLOBAL_IDX,
    );
    root::emit_root_pop_func(
        &mut codes,
        ROOT_STACK_TOP_GLOBAL_IDX,
        ROOT_STACK_BASE_GLOBAL_IDX,
    );
    root::emit_root_set_func(
        &mut codes,
        ROOT_STACK_TOP_GLOBAL_IDX,
        ROOT_STACK_BASE_GLOBAL_IDX,
        ROOT_SLOT_FAILURE_SLOT_GLOBAL_IDX,
        ROOT_SLOT_FAILURE_TOP_GLOBAL_IDX,
        ROOT_SLOT_FAILURE_COUNT_GLOBAL_IDX,
    );
    emit_gc_collect_func(&mut codes, collector_globals);

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

fn emit_free_class_index(f: &mut wasm_encoder::Function, size_local: u32, class_local: u32) {
    use wasm_encoder::Instruction as W;

    // まず oversize class を選び、下限を満たす小さい class で上書きする。
    f.instruction(&W::I32Const(GC_FREE_CLASS_COUNT - 1));
    f.instruction(&W::LocalSet(class_local));
    for (idx, limit) in GC_FREE_CLASS_LIMITS.iter().enumerate().rev() {
        f.instruction(&W::LocalGet(size_local));
        f.instruction(&W::I32Const(*limit));
        f.instruction(&W::I32LeU);
        f.instruction(&W::If(wasm_encoder::BlockType::Empty));
        f.instruction(&W::I32Const(idx as i32));
        f.instruction(&W::LocalSet(class_local));
        f.instruction(&W::End);
    }
}

fn emit_free_class_capacity(f: &mut wasm_encoder::Function, size_local: u32, capacity_local: u32) {
    use wasm_encoder::Instruction as W;

    // bump allocation は従来の linear-memory ABI を保ち、要求された aligned size
    // だけを進める。サイズ class は free-list の探索分岐にだけ使い、既存の
    // heap_ptr/telemetry の差分を発生させない。
    f.instruction(&W::LocalGet(size_local));
    f.instruction(&W::LocalSet(capacity_local));
}

fn emit_small_free_class_pop(
    f: &mut wasm_encoder::Function,
    class_local: u32,
    addr_local: u32,
    capacity_local: u32,
    next_local: u32,
    free_class_heads_base_global_idx: u32,
    free_list_count_global_idx: u32,
) {
    use wasm_encoder::{Instruction as W, MemArg};
    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

    // class 7 は oversize fallback の線形探索に残す。
    for idx in 0..(GC_FREE_CLASS_COUNT - 1) {
        f.instruction(&W::LocalGet(class_local));
        f.instruction(&W::I32Const(idx));
        f.instruction(&W::I32Eq);
        f.instruction(&W::If(wasm_encoder::BlockType::Empty));
        f.instruction(&W::GlobalGet(free_class_heads_base_global_idx + idx as u32));
        f.instruction(&W::LocalSet(next_local));
        f.instruction(&W::LocalGet(next_local));
        f.instruction(&W::I32Eqz);
        f.instruction(&W::If(wasm_encoder::BlockType::Empty));
        f.instruction(&W::Else);
        f.instruction(&W::LocalGet(next_local));
        f.instruction(&W::LocalSet(addr_local));
        f.instruction(&W::LocalGet(next_local));
        f.instruction(&W::I32Load(mem32(4)));
        f.instruction(&W::LocalSet(capacity_local));
        f.instruction(&W::LocalGet(next_local));
        f.instruction(&W::I32Load(mem32(0)));
        f.instruction(&W::GlobalSet(free_class_heads_base_global_idx + idx as u32));
        f.instruction(&W::GlobalGet(free_list_count_global_idx));
        f.instruction(&W::I32Const(1));
        f.instruction(&W::I32Sub);
        f.instruction(&W::GlobalSet(free_list_count_global_idx));
        f.instruction(&W::End);
        f.instruction(&W::End);
    }
}

fn emit_free_class_push(
    f: &mut wasm_encoder::Function,
    class_local: u32,
    addr_local: u32,
    capacity_local: u32,
    next_local: u32,
    free_class_heads_base_global_idx: u32,
) {
    use wasm_encoder::{Instruction as W, MemArg};
    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

    for idx in 0..GC_FREE_CLASS_COUNT {
        f.instruction(&W::LocalGet(class_local));
        f.instruction(&W::I32Const(idx));
        f.instruction(&W::I32Eq);
        f.instruction(&W::If(wasm_encoder::BlockType::Empty));
        f.instruction(&W::GlobalGet(free_class_heads_base_global_idx + idx as u32));
        f.instruction(&W::LocalSet(next_local));
        // 既に解放された object の先頭 8 bytes を free-list node として使う。
        f.instruction(&W::LocalGet(addr_local));
        f.instruction(&W::LocalGet(next_local));
        f.instruction(&W::I32Store(mem32(0)));
        f.instruction(&W::LocalGet(addr_local));
        f.instruction(&W::LocalGet(capacity_local));
        f.instruction(&W::I32Store(mem32(4)));
        f.instruction(&W::LocalGet(addr_local));
        f.instruction(&W::GlobalSet(free_class_heads_base_global_idx + idx as u32));
        f.instruction(&W::End);
    }
}

/// __alloc: サイズクラス別 free-list と oversize fallback を持つ allocator
fn emit_alloc_func(codes: &mut CodeSection, globals: AllocatorGlobals) {
    use wasm_encoder::{Instruction as W, MemArg};

    let AllocatorGlobals {
        heap_ptr_global_idx,
        alloc_count_global_idx,
        object_count_global_idx,
        free_list_count_global_idx,
        free_list_base_global_idx,
        object_table_base_global_idx,
        object_table_capacity_global_idx,
        free_class_heads_base_global_idx,
        free_list_scan_steps_global_idx,
    } = globals;
    let mem32 = |offset: u64| MemArg {
        offset,
        align: 2,
        memory_index: 0,
    };

    let mut f = wasm_encoder::Function::new(vec![(24, ValType::I32)]);

    // local1 = aligned size
    f.instruction(&W::LocalGet(0));
    f.instruction(&W::I32WrapI64);
    f.instruction(&W::I32Const(7));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(-8));
    f.instruction(&W::I32And);
    f.instruction(&W::LocalSet(1));
    // free-list node の next/capacity を置ける最小 block を保証する。
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::LocalSet(1));
    f.instruction(&W::End);

    // local8 = allocated address (0 means not found in free-list)
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(8));

    // local21 = class, local22 = physical capacity, local24 = linked-list next
    emit_free_class_index(&mut f, 1, 21);
    emit_small_free_class_pop(
        &mut f,
        21,
        8,
        22,
        24,
        free_class_heads_base_global_idx,
        free_list_count_global_idx,
    );

    // oversize class は block size を確認する first-fit fallback とする。
    f.instruction(&W::LocalGet(21));
    f.instruction(&W::I32Const(GC_FREE_CLASS_COUNT - 1));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(
        free_class_heads_base_global_idx + (GC_FREE_CLASS_COUNT as u32 - 1),
    ));
    f.instruction(&W::LocalSet(5));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(6));
    // local4 = oversize search hit flag (0 = miss, 1 = reused)
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::BrIf(1));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(22));
    f.instruction(&W::GlobalGet(free_list_scan_steps_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(free_list_scan_steps_global_idx));
    f.instruction(&W::LocalGet(22));
    f.instruction(&W::LocalGet(1));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalSet(6));
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(5));
    f.instruction(&W::Br(0));
    f.instruction(&W::Else);
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::I32Load(mem32(0)));
    f.instruction(&W::LocalSet(24));
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(24));
    f.instruction(&W::GlobalSet(
        free_class_heads_base_global_idx + (GC_FREE_CLASS_COUNT as u32 - 1),
    ));
    f.instruction(&W::Else);
    f.instruction(&W::LocalGet(6));
    f.instruction(&W::LocalGet(24));
    f.instruction(&W::I32Store(mem32(0)));
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalSet(8));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::Br(2));
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Sub);
    f.instruction(&W::GlobalSet(free_list_count_global_idx));
    f.instruction(&W::End);
    f.instruction(&W::End);

    // free-list miss (または oversize miss) だけ class 境界まで予約する。
    // 再利用時は free-list node に保存した実容量をそのまま使う。
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    emit_free_class_capacity(&mut f, 1, 22);
    f.instruction(&W::End);

    // free-list first-fit search
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    // 旧 table は新しい class heads と併用しない。コードは ABI 差分を
    // 小さく保つため残すが、常に bump/class path へ進む。
    f.instruction(&W::Br(0));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(4));
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(4));
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32GeU);
    f.instruction(&W::BrIf(1));

    f.instruction(&W::GlobalGet(free_list_base_global_idx));
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
    f.instruction(&W::GlobalGet(free_list_base_global_idx));
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
    f.instruction(&W::End);

    // free-list miss -> bump allocate
    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Eqz);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(heap_ptr_global_idx));
    f.instruction(&W::LocalSet(2));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalGet(22));
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
    f.instruction(&W::LocalSet(20));
    f.instruction(&W::LocalGet(20));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Unreachable);
    f.instruction(&W::End);
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::GlobalSet(heap_ptr_global_idx));
    f.instruction(&W::LocalGet(2));
    f.instruction(&W::LocalSet(8));
    f.instruction(&W::End);

    // object table が満杯になったら Wasm memory の末尾へ倍増コピーする。
    // metadata を heap payload と同じ固定領域に置かず、既存 object address を動かさない。
    f.instruction(&W::GlobalGet(object_count_global_idx));
    f.instruction(&W::GlobalGet(object_table_capacity_global_idx));
    f.instruction(&W::I32GeU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(object_table_base_global_idx));
    f.instruction(&W::LocalSet(12));
    f.instruction(&W::GlobalGet(object_table_capacity_global_idx));
    f.instruction(&W::LocalSet(13));
    f.instruction(&W::LocalGet(13));
    f.instruction(&W::I32Const(2));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(14));
    f.instruction(&W::LocalGet(14));
    f.instruction(&W::I32Const(GC_OBJECT_SLOT_BYTES));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(17));
    f.instruction(&W::MemorySize(0));
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(15));
    f.instruction(&W::GlobalGet(heap_ptr_global_idx));
    f.instruction(&W::LocalSet(16));
    f.instruction(&W::LocalGet(16));
    f.instruction(&W::LocalGet(15));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(15));
    f.instruction(&W::LocalSet(16));
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(16));
    f.instruction(&W::I32Const(7));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(-8));
    f.instruction(&W::I32And);
    f.instruction(&W::LocalSet(16));
    f.instruction(&W::LocalGet(16));
    f.instruction(&W::LocalGet(17));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(18));
    f.instruction(&W::LocalGet(18));
    f.instruction(&W::LocalGet(15));
    f.instruction(&W::I32GtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(18));
    f.instruction(&W::LocalGet(15));
    f.instruction(&W::I32Sub);
    f.instruction(&W::I32Const(65535));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32DivU);
    f.instruction(&W::LocalSet(19));
    f.instruction(&W::Else);
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(19));
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(19));
    f.instruction(&W::MemoryGrow(0));
    f.instruction(&W::LocalSet(20));
    f.instruction(&W::LocalGet(20));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Unreachable);
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(16));
    f.instruction(&W::LocalGet(12));
    f.instruction(&W::LocalGet(13));
    f.instruction(&W::I32Const(GC_OBJECT_SLOT_BYTES));
    f.instruction(&W::I32Mul);
    f.instruction(&W::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&W::LocalGet(16));
    f.instruction(&W::GlobalSet(object_table_base_global_idx));
    f.instruction(&W::LocalGet(14));
    f.instruction(&W::GlobalSet(object_table_capacity_global_idx));
    f.instruction(&W::LocalGet(18));
    f.instruction(&W::GlobalSet(heap_ptr_global_idx));
    f.instruction(&W::End);

    // live object metadata を記録
    f.instruction(&W::GlobalGet(object_count_global_idx));
    f.instruction(&W::GlobalGet(object_table_capacity_global_idx));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(object_table_base_global_idx));
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
    f.instruction(&W::LocalGet(5));
    f.instruction(&W::LocalGet(22));
    f.instruction(&W::I32Store(mem32(12)));
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
    locals: GcMarkHelperLocals,
) {
    use wasm_encoder::{Instruction as W, MemArg};

    let CollectorGlobals {
        heap_ptr_global_idx,
        heap_start_global_idx,
        ..
    } = globals;
    let object_table_base_global_idx = globals.object_table_base_global_idx;
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

    f.instruction(&W::GlobalGet(object_table_base_global_idx));
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

fn emit_gc_collect_func(codes: &mut CodeSection, globals: CollectorGlobals) {
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
    const FREE_LIST_BASE_LOCAL: u32 = 22;
    const FREE_LIST_CAPACITY_LOCAL: u32 = 23;
    const FREE_LIST_NEW_CAPACITY_LOCAL: u32 = 24;
    const FREE_LIST_BYTES_LOCAL: u32 = 25;
    const MEMORY_END_LOCAL: u32 = 26;
    const NEW_BASE_LOCAL: u32 = 27;
    const NEW_END_LOCAL: u32 = 28;
    const GROW_PAGES_LOCAL: u32 = 29;
    const GROW_RESULT_LOCAL: u32 = 30;
    const OBJ_CAPACITY_LOCAL: u32 = 31;
    const CLASS_INDEX_LOCAL: u32 = 32;
    const NEXT_FREE_LOCAL: u32 = 33;

    let CollectorGlobals {
        heap_ptr_global_idx,
        heap_start_global_idx: _,
        root_stack_top_global_idx,
        root_stack_base_global_idx,
        object_count_global_idx,
        free_list_count_global_idx,
        free_list_base_global_idx,
        free_list_capacity_global_idx,
        object_table_base_global_idx,
        gc_collection_count_global_idx,
        gc_freed_count_global_idx,
        free_class_heads_base_global_idx,
    } = globals;
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

    let mut f = wasm_encoder::Function::new(vec![
        (19, ValType::I32),
        (3, ValType::I64),
        (9, ValType::I32),
        (3, ValType::I32),
    ]);

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
    f.instruction(&W::GlobalGet(object_table_base_global_idx));
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
    f.instruction(&W::GlobalGet(root_stack_base_global_idx));
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

    f.instruction(&W::GlobalGet(object_table_base_global_idx));
    f.instruction(&W::LocalGet(READ_IDX_LOCAL));
    f.instruction(&W::I32Const(4));
    f.instruction(&W::I32Shl);
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(8)));
    f.instruction(&W::LocalSet(MARK_STATE_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(4)));
    f.instruction(&W::LocalSet(OBJ_SIZE_LOCAL));
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(12)));
    f.instruction(&W::LocalSet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::LocalGet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_SIZE_LOCAL));
    f.instruction(&W::LocalSet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::End);

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

    f.instruction(&W::GlobalGet(object_table_base_global_idx));
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
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::I32Load(mem32(12)));
    f.instruction(&W::LocalSet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::LocalGet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(8));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(OBJ_SIZE_LOCAL));
    f.instruction(&W::LocalSet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::End);

    f.instruction(&W::LocalGet(MARK_STATE_LOCAL));
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(object_table_base_global_idx));
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
    f.instruction(&W::LocalGet(ENTRY_PTR_LOCAL));
    f.instruction(&W::LocalGet(OBJ_CAPACITY_LOCAL));
    f.instruction(&W::I32Store(mem32(12)));
    f.instruction(&W::LocalGet(WRITE_IDX_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(WRITE_IDX_LOCAL));
    f.instruction(&W::Else);
    emit_free_class_index(&mut f, OBJ_CAPACITY_LOCAL, CLASS_INDEX_LOCAL);
    emit_free_class_push(
        &mut f,
        CLASS_INDEX_LOCAL,
        OBJ_ADDR_LOCAL,
        OBJ_CAPACITY_LOCAL,
        NEXT_FREE_LOCAL,
        free_class_heads_base_global_idx,
    );
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::GlobalSet(free_list_count_global_idx));
    f.instruction(&W::LocalGet(FREED_THIS_CYCLE_LOCAL));
    f.instruction(&W::I32Const(1));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(FREED_THIS_CYCLE_LOCAL));
    // 旧 free-list table の grow/append 経路は後方互換用に残すが、
    // サイズクラス node を登録した後は実行しない。
    f.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Br(0));
    f.instruction(&W::GlobalGet(free_list_count_global_idx));
    f.instruction(&W::GlobalGet(free_list_capacity_global_idx));
    f.instruction(&W::I32GeU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::GlobalGet(free_list_base_global_idx));
    f.instruction(&W::LocalSet(FREE_LIST_BASE_LOCAL));
    f.instruction(&W::GlobalGet(free_list_capacity_global_idx));
    f.instruction(&W::LocalSet(FREE_LIST_CAPACITY_LOCAL));
    f.instruction(&W::LocalGet(FREE_LIST_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(2));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(FREE_LIST_NEW_CAPACITY_LOCAL));
    f.instruction(&W::LocalGet(FREE_LIST_NEW_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(GC_FREE_LIST_SLOT_BYTES));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(FREE_LIST_BYTES_LOCAL));
    f.instruction(&W::MemorySize(0));
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32Mul);
    f.instruction(&W::LocalSet(MEMORY_END_LOCAL));
    f.instruction(&W::GlobalGet(heap_ptr_global_idx));
    f.instruction(&W::LocalSet(NEW_BASE_LOCAL));
    f.instruction(&W::LocalGet(NEW_BASE_LOCAL));
    f.instruction(&W::LocalGet(MEMORY_END_LOCAL));
    f.instruction(&W::I32LtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(MEMORY_END_LOCAL));
    f.instruction(&W::LocalSet(NEW_BASE_LOCAL));
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(NEW_BASE_LOCAL));
    f.instruction(&W::I32Const(7));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(-8));
    f.instruction(&W::I32And);
    f.instruction(&W::LocalSet(NEW_BASE_LOCAL));
    f.instruction(&W::LocalGet(NEW_BASE_LOCAL));
    f.instruction(&W::LocalGet(FREE_LIST_BYTES_LOCAL));
    f.instruction(&W::I32Add);
    f.instruction(&W::LocalSet(NEW_END_LOCAL));
    f.instruction(&W::LocalGet(NEW_END_LOCAL));
    f.instruction(&W::LocalGet(MEMORY_END_LOCAL));
    f.instruction(&W::I32GtU);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::LocalGet(NEW_END_LOCAL));
    f.instruction(&W::LocalGet(MEMORY_END_LOCAL));
    f.instruction(&W::I32Sub);
    f.instruction(&W::I32Const(65535));
    f.instruction(&W::I32Add);
    f.instruction(&W::I32Const(65536));
    f.instruction(&W::I32DivU);
    f.instruction(&W::LocalSet(GROW_PAGES_LOCAL));
    f.instruction(&W::Else);
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(GROW_PAGES_LOCAL));
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(GROW_PAGES_LOCAL));
    f.instruction(&W::MemoryGrow(0));
    f.instruction(&W::LocalSet(GROW_RESULT_LOCAL));
    f.instruction(&W::LocalGet(GROW_RESULT_LOCAL));
    f.instruction(&W::I32Const(-1));
    f.instruction(&W::I32Eq);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::Unreachable);
    f.instruction(&W::End);
    f.instruction(&W::LocalGet(NEW_BASE_LOCAL));
    f.instruction(&W::LocalGet(FREE_LIST_BASE_LOCAL));
    f.instruction(&W::LocalGet(FREE_LIST_CAPACITY_LOCAL));
    f.instruction(&W::I32Const(GC_FREE_LIST_SLOT_BYTES));
    f.instruction(&W::I32Mul);
    f.instruction(&W::MemoryCopy {
        src_mem: 0,
        dst_mem: 0,
    });
    f.instruction(&W::LocalGet(NEW_BASE_LOCAL));
    f.instruction(&W::GlobalSet(free_list_base_global_idx));
    f.instruction(&W::LocalGet(FREE_LIST_NEW_CAPACITY_LOCAL));
    f.instruction(&W::GlobalSet(free_list_capacity_global_idx));
    f.instruction(&W::LocalGet(NEW_END_LOCAL));
    f.instruction(&W::GlobalSet(heap_ptr_global_idx));
    f.instruction(&W::End);
    f.instruction(&W::GlobalGet(free_list_base_global_idx));
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

    // fd_close の errno を保持し、close 失敗時は payload を公開しない。
    f.instruction(&W::LocalGet(3));
    f.instruction(&W::Call(fd_close_idx));
    f.instruction(&W::LocalSet(8));

    f.instruction(&W::LocalGet(8));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::I32Ne);
    f.instruction(&W::If(wasm_encoder::BlockType::Empty));
    f.instruction(&W::I32Const(0));
    f.instruction(&W::LocalSet(7));
    f.instruction(&W::End);

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
            GcTypeKind::Array(_) | GcTypeKind::PackedByteArray => None,
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
        GcTypeKind::Array(_) | GcTypeKind::PackedByteArray => Err(CodegenError::Error {
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
include!("wasi_tests.rs");
