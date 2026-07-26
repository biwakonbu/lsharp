use std::collections::HashMap;

use lsharp_ir::{GcTypeDef, Instruction};
use wasm_encoder::Function;

use crate::codegen::CodegenError;

use super::{IR_IMPORT_COUNT, structs};

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_instructions_wasi(
    func: &mut Function,
    instructions: &[Instruction],
    gc_types: &[GcTypeDef],
    scratch: structs::WasiStructScratch,
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
        .map(|instr| match instr {
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
        })
        .collect();

    crate::emit::emit_instructions_common_with_handler(
        func,
        &remapped,
        |function, index| {
            match index {
                0 => function.instruction(&W::Call(print_helper_idx)),
                1 => function.instruction(&W::Call(alloc_func_idx)),
                2 => function.instruction(&W::Call(string_concat_idx)),
                3 => function.instruction(&W::Call(string_eq_idx)),
                4 => function.instruction(&W::Call(print_string_idx)),
                5 => function.instruction(&W::Call(proc_exit_wasm_idx)),
                6 => function.instruction(&W::Call(int_to_string_idx)),
                7 => function.instruction(&W::Call(read_file_idx)),
                8 => function.instruction(&W::Call(write_file_idx)),
                9 => function.instruction(&W::Call(file_exists_idx)),
                10 => function.instruction(&W::Call(command_line_args_idx)),
                11 => function.instruction(&W::Call(command_line_arg_idx)),
                12 => function.instruction(&W::Call(read_stdin_idx)),
                13 => function.instruction(&W::Call(fnv1a_hash_idx)),
                14 => function.instruction(&W::Call(root_push_idx)),
                15 => function.instruction(&W::Call(root_pop_idx)),
                16 => function.instruction(&W::Call(root_set_idx)),
                _ => function.instruction(&W::Call(user_func_base + (index - IR_IMPORT_COUNT))),
            };
            Ok(())
        },
        |function, instruction| {
            if matches!(instruction, Instruction::WriteFileBytes) {
                let helper_idx = write_file_bytes_idx.ok_or_else(|| CodegenError::Error {
                    msg: "write-file-bytes はこの target では未対応です".to_string(),
                })?;
                function.instruction(&W::Call(helper_idx));
                return Ok(true);
            }

            structs::emit_wasi_struct_instruction(
                function,
                instruction,
                gc_types,
                alloc_func_idx,
                scratch,
            )
        },
    )
}
