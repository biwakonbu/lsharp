//! WASI 対応の Wasm コード生成
//!
//! wasmtime で直接実行可能な Wasm バイナリを生成する。
//! print 関数を WASI の fd_write で実装し、_start エントリポイントを生成。

use lsharp_ir::{Instruction, Module};
use std::{collections::HashMap, path::PathBuf};
use wasm_encoder::{
    CodeSection, DataSection, ElementSection, Elements, EntityType, ExportKind, ExportSection,
    FunctionSection, GlobalSection, GlobalType, ImportSection, MemorySection, MemoryType,
    TableSection, TableType, TypeSection, ValType,
};

use crate::codegen::CodegenError;

mod allocator;
#[cfg(test)]
mod allocator_tests;
mod argv;
#[cfg(test)]
mod argv_tests;
mod compiler_world;
#[cfg(test)]
mod compiler_world_tests;
mod file_exists;
#[cfg(test)]
mod file_exists_tests;
mod free_list;
#[cfg(test)]
mod free_list_tests;
mod gc_collect;
#[cfg(test)]
mod gc_collect_tests;
mod gc_mark;
#[cfg(test)]
mod gc_mark_tests;
mod hash;
#[cfg(test)]
mod hash_tests;
mod http_handler;
mod instructions;
#[cfg(test)]
mod instructions_tests;
mod int_to_string;
#[cfg(test)]
mod int_to_string_tests;
mod print_i64;
#[cfg(test)]
mod print_i64_tests;
mod print_string;
#[cfg(test)]
mod print_string_tests;
mod read_file;
#[cfg(test)]
mod read_file_tests;
mod root;
#[cfg(test)]
mod root_tests;
mod stdin;
#[cfg(test)]
mod stdin_tests;
mod string_concat;
#[cfg(test)]
mod string_concat_tests;
mod string_eq;
#[cfg(test)]
mod string_eq_tests;
mod structs;
#[cfg(test)]
mod structs_tests;
mod write_file;
mod write_file_bytes;
#[cfg(test)]
mod write_file_bytes_tests;
#[cfg(test)]
mod write_file_tests;

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
    compiler_world::emit_wasm_wasi_with_options(module, false)
}

/// Preview2/component 化向けの Wasm Component を生成する。
pub fn emit_wasm_wasi_p2(module: &Module) -> Result<Vec<u8>, CodegenError> {
    if is_http_handler_module(module) {
        return emit_wasm_http_handler_p2(module);
    }

    let core_wasm = compiler_world::emit_wasm_wasi_with_options(module, true)?;
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
    http_handler::emit_wasm_http_handler_p2(module)
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

#[cfg(test)]
include!("wasi_tests.rs");
