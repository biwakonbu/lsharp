use lsharp_ir::{GcTypeDef, GcTypeKind, Instruction, Module};

use crate::codegen::CodegenError;

/// Struct lowering が利用する一時 local の範囲。
#[derive(Debug, Clone, Copy)]
pub(super) struct WasiStructScratch {
    pub(super) field_base: u32,
    pub(super) ptr_local: u32,
    pub(super) addr_local: u32,
}

pub(super) fn max_struct_field_count(module: &Module) -> u32 {
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

fn struct_field_count(gc_types: &[GcTypeDef], type_index: u32) -> Result<u32, CodegenError> {
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

pub(super) fn emit_wasi_struct_instruction(
    func: &mut wasm_encoder::Function,
    instruction: &Instruction,
    gc_types: &[GcTypeDef],
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
