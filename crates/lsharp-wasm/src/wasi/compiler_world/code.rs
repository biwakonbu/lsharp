use super::*;

pub(super) fn emit_code_section(
    module: &Module,
    export_component_run: bool,
    indices: &WasiCodegenIndices,
    allocator_globals: AllocatorGlobals,
    collector_globals: CollectorGlobals,
    call_indirect_type_map: &HashMap<u32, u32>,
) -> Result<CodeSection, CodegenError> {
    let mut codes = CodeSection::new();
    print_i64::emit_print_i64_func(&mut codes);
    allocator::emit_alloc_func(&mut codes, allocator_globals);
    string_concat::emit_string_concat_func(&mut codes, indices.alloc_func_idx);
    string_eq::emit_string_eq_func(&mut codes);
    print_string::emit_print_string_func(&mut codes);
    int_to_string::emit_int_to_string_func(&mut codes, indices.alloc_func_idx);
    read_file::emit_read_file_func(
        &mut codes,
        indices.alloc_func_idx,
        indices.path_open_idx,
        indices.fd_read_idx,
        indices.fd_close_idx,
        indices.fd_filestat_get_idx,
    );
    write_file::emit_write_file_func(
        &mut codes,
        indices.path_open_idx,
        indices.fd_write_idx,
        indices.fd_close_idx,
    );
    file_exists::emit_file_exists_func(&mut codes, indices.path_open_idx, indices.fd_close_idx);
    argv::emit_command_line_args_func(&mut codes, indices.args_sizes_get_idx);
    argv::emit_command_line_arg_func(
        &mut codes,
        indices.alloc_func_idx,
        indices.args_get_idx,
        indices.args_sizes_get_idx,
    );
    stdin::emit_read_stdin_func(
        &mut codes,
        indices.alloc_func_idx,
        indices.string_concat_idx,
        indices.fd_read_idx,
    );
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
        indices.alloc_func_idx,
        indices.path_open_idx,
        indices.fd_write_idx,
        indices.fd_close_idx,
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
            indices.print_helper_idx,
            indices.alloc_func_idx,
            indices.string_concat_idx,
            indices.string_eq_idx,
            indices.print_string_idx,
            indices.proc_exit_helper_idx,
            indices.int_to_string_idx,
            indices.read_file_idx,
            indices.write_file_idx,
            Some(indices.write_file_bytes_idx),
            indices.file_exists_idx,
            indices.command_line_args_idx,
            indices.command_line_arg_idx,
            indices.read_stdin_idx,
            indices.fnv1a_hash_idx,
            indices.root_push_idx,
            indices.root_pop_idx,
            indices.root_set_idx,
            indices.user_func_base,
            call_indirect_type_map,
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
        f.instruction(&wasm_encoder::Instruction::Call(indices.gc_collect_idx));
        f.instruction(&wasm_encoder::Instruction::Drop);
        f.instruction(&wasm_encoder::Instruction::End);
        f.instruction(&wasm_encoder::Instruction::LocalGet(0));
        f.instruction(&wasm_encoder::Instruction::Call(indices.proc_exit_wasm_idx));
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
                indices.user_func_base + main_idx as u32,
            ));
            f.instruction(&wasm_encoder::Instruction::Drop);
            f.instruction(&wasm_encoder::Instruction::Call(indices.gc_collect_idx));
            f.instruction(&wasm_encoder::Instruction::Drop);
        }
        f.instruction(&wasm_encoder::Instruction::End);
        codes.function(&f);
    }

    if export_component_run {
        let mut f = wasm_encoder::Function::new(vec![]);
        f.instruction(&wasm_encoder::Instruction::Call(indices.start_func_idx));
        f.instruction(&wasm_encoder::Instruction::I32Const(0));
        f.instruction(&wasm_encoder::Instruction::End);
        codes.function(&f);
    }

    Ok(codes)
}
