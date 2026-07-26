use lsharp_ir::{GcTypeKind, Instruction, Module};
use wasm_encoder::{BlockType, Function, HeapType, MemArg};

use crate::codegen::CodegenError;

use super::{LOWER_RUNTIME_IMPORT_COUNT, PRINT_STRING_RUNTIME_INDEX, codegen_error, validation};

#[derive(Debug, Clone, Copy)]
pub(super) struct ComponentOutputLocals {
    pub(super) array: u32,
    pub(super) ptr: u32,
    pub(super) len: u32,
    pub(super) index: u32,
}

pub(super) struct WasmGcEmitOptions<'a> {
    pub(super) module: &'a Module,
    pub(super) function_count: usize,
    pub(super) import_count: u32,
    pub(super) print_string_import: bool,
    pub(super) component_output_import_index: Option<u32>,
    pub(super) output_locals: Option<ComponentOutputLocals>,
}

pub(super) fn emit_wasm_gc_instructions(
    function: &mut Function,
    instructions: &[Instruction],
    options: &WasmGcEmitOptions<'_>,
) -> Result<(), CodegenError> {
    use wasm_encoder::Instruction as W;

    crate::emit::emit_instructions_common_with_handler(
        function,
        instructions,
        |function, index| {
            if index == PRINT_STRING_RUNTIME_INDEX {
                if !options.print_string_import {
                    return Err(codegen_error(
                        "print-string import boundary が materialize されていません",
                    ));
                }
                if options.component_output_import_index.is_some() {
                    let import_index = options.component_output_import_index.ok_or_else(|| {
                        codegen_error(
                            "Component output の write import が materialize されていません",
                        )
                    })?;
                    let locals = options.output_locals.ok_or_else(|| {
                        codegen_error("Component output の linear-memory locals がありません")
                    })?;
                    emit_component_output_call(function, options.module, import_index, locals)?;
                } else {
                    function.instruction(&W::Call(options.module.imports.len() as u32));
                }
                return Ok(());
            }
            let local_index = index
                .checked_sub(LOWER_RUNTIME_IMPORT_COUNT)
                .ok_or_else(|| {
                    codegen_error(format!(
                        "WasmGC backend は runtime import 呼び出しを未対応です: Call({index})"
                    ))
                })?;
            if (local_index as usize) >= options.function_count {
                return Err(codegen_error(format!(
                    "ユーザー関数の呼び出しインデックスが範囲外です: Call({index})"
                )));
            }
            function.instruction(&W::Call(options.import_count + local_index));
            Ok(())
        },
        |function, instruction| {
            match instruction {
                Instruction::StructNew(type_index) => {
                    function.instruction(&W::StructNew(*type_index));
                }
                Instruction::StructGet(type_index, field_index) => {
                    function.instruction(&W::StructGet {
                        struct_type_index: *type_index,
                        field_index: *field_index,
                    });
                }
                Instruction::StructSet(type_index, field_index) => {
                    function.instruction(&W::StructSet {
                        struct_type_index: *type_index,
                        field_index: *field_index,
                    });
                }
                Instruction::RefCast(type_index) => {
                    function.instruction(&W::RefCastNullable(HeapType::Concrete(*type_index)));
                }
                Instruction::RefNull(type_index) => {
                    function.instruction(&W::RefNull(HeapType::Concrete(*type_index)));
                }
                Instruction::RefFunc(function_index) => {
                    function.instruction(&W::RefFunc(
                        *function_index + u32::from(options.print_string_import),
                    ));
                }
                Instruction::CallRef(type_index) => {
                    function.instruction(&W::CallRef(
                        *type_index + u32::from(options.print_string_import),
                    ));
                }
                Instruction::ArrayNewFixed(type_index, length) => {
                    function.instruction(&W::ArrayNewFixed {
                        array_type_index: *type_index,
                        array_size: *length,
                    });
                }
                Instruction::ArrayNewDefault(type_index) => {
                    function.instruction(&W::ArrayNewDefault(*type_index));
                }
                Instruction::ArrayGet(type_index) => {
                    let is_packed = matches!(
                        options
                            .module
                            .gc_types
                            .get(*type_index as usize)
                            .map(|gc_type| &gc_type.kind),
                        Some(GcTypeKind::PackedByteArray)
                    );
                    if is_packed {
                        function.instruction(&W::ArrayGetU(*type_index));
                    } else {
                        function.instruction(&W::ArrayGet(*type_index));
                    }
                }
                Instruction::ArraySet(type_index) => {
                    function.instruction(&W::ArraySet(*type_index));
                }
                Instruction::ArrayLen(_) => {
                    function.instruction(&W::ArrayLen);
                }
                _ => return Ok(false),
            }
            Ok(true)
        },
    )
}

fn emit_component_output_call(
    function: &mut Function,
    module: &Module,
    import_index: u32,
    locals: ComponentOutputLocals,
) -> Result<(), CodegenError> {
    use wasm_encoder::Instruction as W;

    let array_type_index = validation::string_array_type_index(module)?;

    // GC reference はこの同期的な copy/write 呼び出しの間だけ借用する。
    function.instruction(&W::LocalSet(locals.array));
    function.instruction(&W::I32Const(0));
    function.instruction(&W::LocalSet(locals.ptr));
    function.instruction(&W::LocalGet(locals.array));
    function.instruction(&W::ArrayLen);
    function.instruction(&W::LocalSet(locals.len));

    // 最初の store 前に memory を grow する。grow 失敗時の -1 は trap に変換する。
    function.instruction(&W::LocalGet(locals.len));
    function.instruction(&W::I32Const(65_535));
    function.instruction(&W::I32Add);
    function.instruction(&W::I32Const(16));
    function.instruction(&W::I32ShrU);
    function.instruction(&W::MemoryGrow(0));
    function.instruction(&W::I32Const(-1));
    function.instruction(&W::I32Eq);
    function.instruction(&W::If(BlockType::Empty));
    function.instruction(&W::Unreachable);
    function.instruction(&W::End);

    function.instruction(&W::I32Const(0));
    function.instruction(&W::LocalSet(locals.index));
    function.instruction(&W::Block(BlockType::Empty));
    function.instruction(&W::Loop(BlockType::Empty));
    function.instruction(&W::LocalGet(locals.index));
    function.instruction(&W::LocalGet(locals.len));
    function.instruction(&W::I32GeU);
    function.instruction(&W::BrIf(1));

    function.instruction(&W::LocalGet(locals.ptr));
    function.instruction(&W::LocalGet(locals.index));
    function.instruction(&W::I32Add);
    function.instruction(&W::LocalGet(locals.array));
    function.instruction(&W::LocalGet(locals.index));
    function.instruction(&W::ArrayGetU(array_type_index));
    function.instruction(&W::I32Store8(MemArg {
        offset: 0,
        align: 0,
        memory_index: 0,
    }));

    function.instruction(&W::LocalGet(locals.index));
    function.instruction(&W::I32Const(1));
    function.instruction(&W::I32Add);
    function.instruction(&W::LocalSet(locals.index));
    function.instruction(&W::Br(0));
    function.instruction(&W::End);
    function.instruction(&W::End);

    function.instruction(&W::LocalGet(locals.ptr));
    function.instruction(&W::LocalGet(locals.len));
    function.instruction(&W::Call(import_index));
    Ok(())
}
