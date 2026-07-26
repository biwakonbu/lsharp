use super::*;

/// HTTP handler world 向けの Wasm Component を生成する。
pub(super) fn emit_wasm_http_handler_p2(module: &Module) -> Result<Vec<u8>, CodegenError> {
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
    super::allocator::emit_alloc_func(&mut codes, allocator_globals);
    string_concat::emit_string_concat_func(&mut codes, alloc_func_idx);
    string_eq::emit_string_eq_func(&mut codes);
    emit_trap_i64_to_unit_func(&mut codes);
    emit_trap_i32_to_unit_func(&mut codes);
    int_to_string::emit_int_to_string_func(&mut codes, alloc_func_idx);
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
    super::gc_collect::emit_gc_collect_func(&mut codes, collector_globals);

    let struct_scratch_fields = structs::max_struct_field_count(module);
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
