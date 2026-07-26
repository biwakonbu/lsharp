use super::*;

pub(super) fn wasm_gc_valtype(ty: IrType, synthetic_import_offset: u32) -> ValType {
    match ty {
        IrType::I64 => ValType::I64,
        IrType::F64 => ValType::F64,
        IrType::I32 => ValType::I32,
        IrType::Ref(index) => ValType::Ref(RefType {
            nullable: true,
            heap_type: HeapType::Concrete(index),
        }),
        IrType::FuncRef => ValType::FUNCREF,
        IrType::TypedFuncRef(index) => ValType::Ref(RefType {
            nullable: true,
            heap_type: HeapType::Concrete(index + synthetic_import_offset),
        }),
    }
}

fn wasm_gc_subtype(inner: CompositeInnerType) -> SubType {
    SubType {
        is_final: true,
        supertype_idx: None,
        composite_type: CompositeType {
            inner,
            shared: false,
            descriptor: None,
            describes: None,
        },
    }
}

pub(super) fn wasm_gc_gc_subtype(gc_type: &GcTypeDef, synthetic_import_offset: u32) -> SubType {
    match &gc_type.kind {
        GcTypeKind::Struct(fields) => wasm_gc_subtype(CompositeInnerType::Struct(StructType {
            fields: fields
                .iter()
                .map(|field| FieldType {
                    element_type: StorageType::Val(wasm_gc_valtype(
                        field.ty,
                        synthetic_import_offset,
                    )),
                    mutable: field.mutable,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        })),
        GcTypeKind::Array(element_type) => {
            wasm_gc_subtype(CompositeInnerType::Array(ArrayType(FieldType {
                element_type: StorageType::Val(wasm_gc_valtype(
                    *element_type,
                    synthetic_import_offset,
                )),
                mutable: true,
            })))
        }
        GcTypeKind::PackedByteArray => {
            wasm_gc_subtype(CompositeInnerType::Array(ArrayType(FieldType {
                element_type: StorageType::I8,
                mutable: true,
            })))
        }
    }
}

pub(super) fn wasm_gc_function_subtype(
    params: &[IrType],
    results: &[IrType],
    synthetic_import_offset: u32,
) -> SubType {
    wasm_gc_subtype(CompositeInnerType::Func(FuncType::new(
        params
            .iter()
            .copied()
            .map(|ty| wasm_gc_valtype(ty, synthetic_import_offset)),
        results
            .iter()
            .copied()
            .map(|ty| wasm_gc_valtype(ty, synthetic_import_offset)),
    )))
}

pub(super) fn module_uses_typed_funcref(module: &Module) -> bool {
    let uses_typed_funcref = |ty: IrType| matches!(ty, IrType::TypedFuncRef(_));
    module.gc_types.iter().any(|gc_type| match &gc_type.kind {
        GcTypeKind::Struct(fields) => fields.iter().any(|field| uses_typed_funcref(field.ty)),
        GcTypeKind::Array(element_type) => uses_typed_funcref(*element_type),
        GcTypeKind::PackedByteArray => false,
    }) || module.imports.iter().any(|import| {
        import.params.iter().copied().any(uses_typed_funcref) || uses_typed_funcref(import.result)
    }) || module.functions.iter().any(|function| {
        function.params.iter().copied().any(uses_typed_funcref)
            || uses_typed_funcref(function.result)
            || function.locals.iter().copied().any(uses_typed_funcref)
    })
}

pub(super) fn module_uses_print_string(module: &Module) -> bool {
    module.functions.iter().any(|function| {
        function
            .body
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Call(PRINT_STRING_RUNTIME_INDEX)))
    })
}

pub(super) fn string_array_type_index(module: &Module) -> Result<u32, CodegenError> {
    module
        .gc_types
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, gc_type)| {
            matches!(&gc_type.kind, GcTypeKind::PackedByteArray).then_some(index as u32)
        })
        .ok_or_else(|| codegen_error("print-string の StringBytes GC array type がありません"))
}

pub(super) fn validate_module(
    module: &Module,
    print_string_import: bool,
) -> Result<(), CodegenError> {
    let function_type_start = module.gc_types.len() + module.imports.len();
    let function_type_count = module.functions.len();
    if print_string_import {
        let string_type_index = string_array_type_index(module)?;
        validate_gc_type_index(
            string_type_index,
            module.gc_types.len(),
            "print-string parameter",
        )?;
    }

    for gc_type in &module.gc_types {
        match &gc_type.kind {
            GcTypeKind::Struct(fields) => {
                for field in fields {
                    validate_ir_type(
                        field.ty,
                        module.gc_types.len(),
                        function_type_start,
                        function_type_count,
                        "struct field",
                    )?;
                }
            }
            GcTypeKind::Array(element_type) => {
                validate_ir_type(
                    *element_type,
                    module.gc_types.len(),
                    function_type_start,
                    function_type_count,
                    "array element",
                )?;
            }
            GcTypeKind::PackedByteArray => {}
        }
    }

    for import in &module.imports {
        for &param in &import.params {
            validate_ir_type(
                param,
                module.gc_types.len(),
                function_type_start,
                function_type_count,
                "import parameter",
            )?;
        }
        validate_ir_type(
            import.result,
            module.gc_types.len(),
            function_type_start,
            function_type_count,
            "import result",
        )?;
    }

    for function in &module.functions {
        validate_ir_type(
            function.result,
            module.gc_types.len(),
            function_type_start,
            function_type_count,
            "function result",
        )?;
        for &param in &function.params {
            validate_ir_type(
                param,
                module.gc_types.len(),
                function_type_start,
                function_type_count,
                "function parameter",
            )?;
        }
        for &local in &function.locals {
            validate_ir_type(
                local,
                module.gc_types.len(),
                function_type_start,
                function_type_count,
                "function local",
            )?;
        }
        for instruction in &function.body {
            match instruction {
                Instruction::Call(function_index) => {
                    if *function_index == PRINT_STRING_RUNTIME_INDEX {
                        if !print_string_import {
                            return Err(codegen_error(
                                "print-string import boundary が materialize されていません",
                            ));
                        }
                        continue;
                    }
                    let Some(local_index) = function_index.checked_sub(LOWER_RUNTIME_IMPORT_COUNT)
                    else {
                        return Err(codegen_error(format!(
                            "WasmGC backend は runtime import 呼び出しを未対応です: Call({function_index})"
                        )));
                    };
                    if (local_index as usize) >= module.functions.len() {
                        return Err(codegen_error(format!(
                            "ユーザー関数の呼び出しインデックスが範囲外です: Call({function_index})"
                        )));
                    }
                }
                Instruction::CallImport(function_index) => {
                    return Err(codegen_error(format!(
                        "WasmGC backend は CallImport を未対応です: {function_index}"
                    )));
                }
                Instruction::StructNew(type_index) => {
                    validate_gc_type_index(*type_index, module.gc_types.len(), instruction)?;
                    if !matches!(
                        module
                            .gc_types
                            .get(*type_index as usize)
                            .map(|gc_type| &gc_type.kind),
                        Some(GcTypeKind::Struct(_))
                    ) {
                        return Err(codegen_error(format!(
                            "struct.new の GC 型インデックスが struct ではありません: {type_index}"
                        )));
                    }
                }
                Instruction::RefCast(type_index) => {
                    validate_gc_type_index(*type_index, module.gc_types.len(), instruction)?;
                }
                Instruction::RefNull(type_index) => {
                    validate_gc_type_index(*type_index, module.gc_types.len(), instruction)?;
                }
                Instruction::RefFunc(function_index) => {
                    let function_count = module.imports.len() + module.functions.len();
                    if (*function_index as usize) >= function_count {
                        return Err(codegen_error(format!(
                            "ref.func の関数インデックスが範囲外です: {function_index}"
                        )));
                    }
                }
                Instruction::CallRef(type_index) => {
                    validate_function_type_index(
                        *type_index,
                        function_type_start,
                        function_type_count,
                        "call_ref",
                    )?;
                }
                Instruction::ArrayNewFixed(type_index, _)
                | Instruction::ArrayNewDefault(type_index)
                | Instruction::ArrayGet(type_index)
                | Instruction::ArraySet(type_index)
                | Instruction::ArrayLen(type_index) => {
                    let Some(GcTypeKind::Array(_) | GcTypeKind::PackedByteArray) = module
                        .gc_types
                        .get(*type_index as usize)
                        .map(|gc_type| &gc_type.kind)
                    else {
                        return Err(codegen_error(format!(
                            "array 命令の型インデックスが array ではありません: {type_index}"
                        )));
                    };
                }
                Instruction::StructGet(type_index, field_index)
                | Instruction::StructSet(type_index, field_index) => {
                    let Some(GcTypeKind::Struct(fields)) = module
                        .gc_types
                        .get(*type_index as usize)
                        .map(|gc_type| &gc_type.kind)
                    else {
                        return Err(codegen_error(format!(
                            "struct 命令の型インデックスが struct ではありません: {type_index}"
                        )));
                    };
                    if fields.get(*field_index as usize).is_none() {
                        return Err(codegen_error(format!(
                            "struct 命令のフィールドインデックスが範囲外です: type={type_index}, field={field_index}"
                        )));
                    }
                    if matches!(instruction, Instruction::StructSet(_, _))
                        && !fields[*field_index as usize].mutable
                    {
                        return Err(codegen_error(format!(
                            "struct.set のフィールドが immutable です: type={type_index}, field={field_index}"
                        )));
                    }
                }
                Instruction::If(IrType::Ref(_))
                | Instruction::Block(IrType::Ref(_))
                | Instruction::Loop(IrType::Ref(_)) => {
                    return Err(codegen_error(
                        "GC 参照を結果に持つ制御ブロックは WasmGC backend では未対応です",
                    ));
                }
                Instruction::StringConst(_)
                | Instruction::WriteFileBytes
                | Instruction::I32Load { .. }
                | Instruction::I32Store { .. }
                | Instruction::I32Load8U { .. }
                | Instruction::I32Store8 { .. }
                | Instruction::I64Load { .. }
                | Instruction::I64Store { .. }
                | Instruction::MemoryGrow
                | Instruction::MemorySize
                | Instruction::MemoryCopy
                | Instruction::MemoryFill
                | Instruction::GlobalGet(_)
                | Instruction::GlobalSet(_)
                | Instruction::CallIndirect(_)
                | Instruction::FuncIdx(_) => {
                    return Err(codegen_error(format!(
                        "WasmGC backend が未対応の命令です: {instruction}"
                    )));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn validate_gc_ref(ty: IrType, gc_type_count: usize, context: &str) -> Result<(), CodegenError> {
    if let IrType::Ref(index) = ty {
        validate_gc_type_index(index, gc_type_count, context)?;
    }
    Ok(())
}

fn validate_ir_type(
    ty: IrType,
    gc_type_count: usize,
    function_type_start: usize,
    function_type_count: usize,
    context: &str,
) -> Result<(), CodegenError> {
    validate_gc_ref(ty, gc_type_count, context)?;
    if let IrType::TypedFuncRef(index) = ty {
        validate_function_type_index(index, function_type_start, function_type_count, context)?;
    }
    Ok(())
}

fn validate_function_type_index(
    index: u32,
    function_type_start: usize,
    function_type_count: usize,
    context: impl std::fmt::Display,
) -> Result<(), CodegenError> {
    let function_type_end = function_type_start + function_type_count;
    if (index as usize) < function_type_start || (index as usize) >= function_type_end {
        return Err(codegen_error(format!(
            "{context} の関数型インデックスが範囲外です: {index}"
        )));
    }
    Ok(())
}

pub(super) fn validate_gc_type_index(
    index: u32,
    gc_type_count: usize,
    context: impl std::fmt::Display,
) -> Result<(), CodegenError> {
    if (index as usize) >= gc_type_count {
        return Err(codegen_error(format!(
            "{context} の GC 型インデックスが範囲外です: {index}"
        )));
    }
    Ok(())
}
