//! WasmGC バックエンドの capability probe と Stage 1 IR emitter。
//!
//! Stage 0 では `wasm-encoder` で GC struct と `struct.new` / `struct.get` を含む
//! self-contained module を生成し、Wasmtime が実行できることを固定する。Stage 1 では
//! 同じ型・命令契約を L# IR の `Module` から生成する。

use lsharp_ir::{GcTypeKind, Instruction, IrType, Module};
use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, FieldType, Function, FunctionSection,
    HeapType, ImportSection, RefType, StorageType, TypeSection, ValType,
};

use crate::codegen::CodegenError;

/// Lower が予約する runtime import の論理インデックス数。
///
/// WasmGC backend は現時点で runtime import を materialize しないため、lower が
/// `Call(17 + user_index)` として保持するユーザー関数呼び出しだけを core module の
/// ローカル関数 index へ変換する。
const LOWER_RUNTIME_IMPORT_COUNT: u32 = 17;

/// L# IR を WasmGC core module へ変換する。
///
/// Stage 1 では GC 型定義と `StructNew` / `StructGet` / `StructSet` / `RefCast`、
/// および linear-memory に依存しない基本命令を扱う。WASI や文字列の linear-memory
/// ABI は後続 stage の責務であり、未対応命令は i64 に黙ってフォールバックせず診断する。
pub fn emit_wasm_wasmgc(module: &Module) -> Result<Vec<u8>, CodegenError> {
    validate_module(module)?;

    let mut wasm_module = wasm_encoder::Module::new();
    let mut types = TypeSection::new();

    for gc_type in &module.gc_types {
        match &gc_type.kind {
            GcTypeKind::Struct(fields) => {
                let fields = fields
                    .iter()
                    .map(|field| FieldType {
                        element_type: StorageType::Val(wasm_gc_valtype(field.ty)),
                        mutable: field.mutable,
                    })
                    .collect::<Vec<_>>();
                types.ty().struct_(fields);
            }
            GcTypeKind::Array(element_type) => {
                types
                    .ty()
                    .array(&StorageType::Val(wasm_gc_valtype(*element_type)), true);
            }
        }
    }

    let mut import_type_indices = Vec::with_capacity(module.imports.len());
    for import in &module.imports {
        import_type_indices.push(types.len());
        types.ty().function(
            import.params.iter().copied().map(wasm_gc_valtype),
            [wasm_gc_valtype(import.result)],
        );
    }

    let mut function_type_indices = Vec::with_capacity(module.functions.len());
    for function in &module.functions {
        function_type_indices.push(types.len());
        types.ty().function(
            function.params.iter().copied().map(wasm_gc_valtype),
            [wasm_gc_valtype(function.result)],
        );
    }
    wasm_module.section(&types);

    let mut imports = ImportSection::new();
    for (import, &type_index) in module.imports.iter().zip(&import_type_indices) {
        imports.import(
            &import.module,
            &import.name,
            EntityType::Function(type_index),
        );
    }
    if !module.imports.is_empty() {
        wasm_module.section(&imports);
    }

    let mut functions = FunctionSection::new();
    for type_index in function_type_indices {
        functions.function(type_index);
    }
    if !module.functions.is_empty() {
        wasm_module.section(&functions);
    }

    let mut exports = ExportSection::new();
    let import_count = module.imports.len() as u32;
    for (index, function) in module.functions.iter().enumerate() {
        if function.is_export {
            exports.export(
                &function.name,
                ExportKind::Func,
                import_count + index as u32,
            );
        }
    }
    if !module.functions.is_empty() {
        wasm_module.section(&exports);
    }

    let mut code = CodeSection::new();
    for function in &module.functions {
        let locals = function
            .locals
            .iter()
            .copied()
            .map(|ty| (1, wasm_gc_valtype(ty)))
            .collect::<Vec<_>>();
        let mut wasm_function = Function::new(locals);
        emit_wasm_gc_instructions(&mut wasm_function, &function.body, module.functions.len())?;
        wasm_function.instruction(&wasm_encoder::Instruction::End);
        code.function(&wasm_function);
    }
    if !module.functions.is_empty() {
        wasm_module.section(&code);
    }

    Ok(wasm_module.finish())
}

fn wasm_gc_valtype(ty: IrType) -> ValType {
    match ty {
        IrType::I64 => ValType::I64,
        IrType::F64 => ValType::F64,
        IrType::I32 => ValType::I32,
        IrType::Ref(index) => ValType::Ref(RefType {
            nullable: true,
            heap_type: HeapType::Concrete(index),
        }),
        IrType::FuncRef => ValType::FUNCREF,
    }
}

fn validate_module(module: &Module) -> Result<(), CodegenError> {
    for gc_type in &module.gc_types {
        match &gc_type.kind {
            GcTypeKind::Struct(fields) => {
                for field in fields {
                    validate_gc_ref(field.ty, module.gc_types.len(), "struct field")?;
                }
            }
            GcTypeKind::Array(element_type) => {
                validate_gc_ref(*element_type, module.gc_types.len(), "array element")?;
            }
        }
    }

    for import in &module.imports {
        for &param in &import.params {
            validate_gc_ref(param, module.gc_types.len(), "import parameter")?;
        }
        validate_gc_ref(import.result, module.gc_types.len(), "import result")?;
    }

    for function in &module.functions {
        validate_gc_ref(function.result, module.gc_types.len(), "function result")?;
        for &param in &function.params {
            validate_gc_ref(param, module.gc_types.len(), "function parameter")?;
        }
        for &local in &function.locals {
            validate_gc_ref(local, module.gc_types.len(), "function local")?;
        }
        for instruction in &function.body {
            match instruction {
                Instruction::Call(function_index) => {
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
                        "GC 参照を結果に持つ制御ブロックは Stage 1 では未対応です",
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
                | Instruction::RefFunc(_)
                | Instruction::CallRef(_)
                | Instruction::CallIndirect(_)
                | Instruction::FuncIdx(_) => {
                    return Err(codegen_error(format!(
                        "WasmGC Stage 1 backend が未対応の命令です: {instruction}"
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

fn validate_gc_type_index(
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

fn emit_wasm_gc_instructions(
    function: &mut Function,
    instructions: &[Instruction],
    function_count: usize,
) -> Result<(), CodegenError> {
    use wasm_encoder::Instruction as W;

    crate::emit::emit_instructions_common_with_handler(
        function,
        instructions,
        |function, index| {
            let local_index = index
                .checked_sub(LOWER_RUNTIME_IMPORT_COUNT)
                .ok_or_else(|| {
                    codegen_error(format!(
                        "WasmGC backend は runtime import 呼び出しを未対応です: Call({index})"
                    ))
                })?;
            if (local_index as usize) >= function_count {
                return Err(codegen_error(format!(
                    "ユーザー関数の呼び出しインデックスが範囲外です: Call({index})"
                )));
            }
            function.instruction(&W::Call(local_index));
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
                _ => return Ok(false),
            }
            Ok(true)
        },
    )
}

fn codegen_error(message: impl Into<String>) -> CodegenError {
    CodegenError::Error {
        msg: message.into(),
    }
}

/// WasmGC の struct 命令を含む最小の self-contained module を生成する。
///
/// `read-field` は immutable な `struct { i64 }` を生成し、フィールド 0 を読み出して
/// `42` を返す。型セクションの 0 番目は GC struct、1 番目は関数型である。
pub fn emit_minimal_struct_probe() -> Vec<u8> {
    let mut module = wasm_encoder::Module::new();

    let mut types = TypeSection::new();
    types.ty().struct_([FieldType {
        element_type: StorageType::Val(ValType::I64),
        mutable: false,
    }]);
    types.ty().function([], [ValType::I64]);
    module.section(&types);

    let mut functions = FunctionSection::new();
    functions.function(1);
    module.section(&functions);

    let mut exports = ExportSection::new();
    exports.export("read-field", ExportKind::Func, 0);
    module.section(&exports);

    let mut code = CodeSection::new();
    let mut function = Function::new([]);
    function.instruction(&wasm_encoder::Instruction::I64Const(42));
    function.instruction(&wasm_encoder::Instruction::StructNew(0));
    function.instruction(&wasm_encoder::Instruction::StructGet {
        struct_type_index: 0,
        field_index: 0,
    });
    function.instruction(&wasm_encoder::Instruction::End);
    code.function(&function);
    module.section(&code);

    module.finish()
}
