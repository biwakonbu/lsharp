use lsharp_ir::Instruction;
use std::collections::HashMap;
use wasm_encoder::{CodeSection, Function, Instruction as W};

use super::{instructions::emit_instructions_wasi, structs::WasiStructScratch};

#[test]
fn instructions_module_emits_empty_instruction_body() {
    let mut function = Function::new(vec![]);
    emit_instructions_wasi(
        &mut function,
        &[],
        &[],
        WasiStructScratch {
            field_base: 0,
            ptr_local: 0,
            addr_local: 0,
        },
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        &HashMap::new(),
    )
    .expect("empty instruction list should emit successfully");
    function.instruction(&W::End);

    let mut codes = CodeSection::new();
    codes.function(&function);

    assert_eq!(codes.len(), 1);
}

#[test]
fn instructions_module_rejects_write_file_bytes_without_helper() {
    let mut function = Function::new(vec![]);
    let error = emit_instructions_wasi(
        &mut function,
        &[Instruction::WriteFileBytes],
        &[],
        WasiStructScratch {
            field_base: 0,
            ptr_local: 0,
            addr_local: 0,
        },
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        &HashMap::new(),
    )
    .expect_err("write-file-bytes must fail closed without a helper index");

    assert!(error.to_string().contains("write-file-bytes"));
}
