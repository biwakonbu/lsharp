use super::*;

pub(super) fn emit_wasm_wasi_with_options(
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
    print_i64::emit_print_i64_func(&mut codes);
    allocator::emit_alloc_func(&mut codes, allocator_globals);
    string_concat::emit_string_concat_func(&mut codes, alloc_func_idx);
    string_eq::emit_string_eq_func(&mut codes);
    print_string::emit_print_string_func(&mut codes);
    int_to_string::emit_int_to_string_func(&mut codes, alloc_func_idx);
    read_file::emit_read_file_func(
        &mut codes,
        alloc_func_idx,
        path_open_idx,
        fd_read_idx,
        fd_close_idx,
        fd_filestat_get_idx,
    );
    write_file::emit_write_file_func(&mut codes, path_open_idx, fd_write_idx, fd_close_idx);
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
    gc_collect::emit_gc_collect_func(&mut codes, collector_globals);

    let struct_scratch_fields = structs::max_struct_field_count(module);
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
        let scratch = structs::WasiStructScratch {
            field_base: scratch_base,
            ptr_local: scratch_base + struct_scratch_fields,
            addr_local: scratch_base + struct_scratch_fields + 1,
        };
        let mut f = wasm_encoder::Function::new(locals);
        instructions::emit_instructions_wasi(
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
