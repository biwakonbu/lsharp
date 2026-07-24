//! WasmGC バックエンドの capability probe と Stage 1/2 IR emitter。
//!
//! Stage 0 では `wasm-encoder` で GC struct と `struct.new` / `struct.get` を含む
//! self-contained module を生成し、Wasmtime が実行できることを固定する。Stage 1/2 では
//! 同じ型・命令契約を L# IR の `Module` から生成する。

use std::borrow::Cow;

use lsharp_ir::{GcTypeDef, GcTypeKind, Instruction, IrType, Module};
use wasm_encoder::{
    ArrayType, CodeSection, CompositeInnerType, CompositeType, ElementSection, Elements,
    EntityType, ExportKind, ExportSection, FieldType, FuncType, Function, FunctionSection,
    HeapType, ImportSection, MemorySection, MemoryType, RefType, StorageType, StructType, SubType,
    TypeSection, ValType,
};

use crate::codegen::CodegenError;

/// Lower が予約する runtime import の論理インデックス数。
///
/// WasmGC backend は runtime import を論理 index から必要な external boundary だけ materialize
/// する。`Call(17 + user_index)` はユーザー関数、`Call(4)` は `print-string` を表す。
const LOWER_RUNTIME_IMPORT_COUNT: u32 = 17;
const PRINT_STRING_RUNTIME_INDEX: u32 = 4;
const COMPONENT_OUTPUT_MODULE: &str = "lsharp:wasmgc-output/stdout@0.1.0";
const COMPONENT_OUTPUT_NAME: &str = "write";

/// L# IR を WasmGC core module へ変換する。
///
/// Stage 1/2 では GC 型定義、struct 命令、scalar String array の
/// `ArrayNewFixed` / `ArrayNewDefault` / `ArrayGet` / `ArraySet` / `ArrayLen`、
/// および linear-memory に依存しない基本命令を扱う。WASI や文字列の linear-memory ABI は
/// 後続 stage の責務であり、`print-string` 以外の未対応 runtime import は i64 に黙って
/// フォールバックせず診断する。
pub fn emit_wasm_wasmgc(module: &Module) -> Result<Vec<u8>, CodegenError> {
    emit_wasm_wasmgc_internal(module, false, false)
}

/// L# IR を Component Model の output `list<u8>` canonical ABI を持つ
/// WasmGC core module へ変換する。
///
/// `print-string` の packed GC array は module 内で一時的に linear memory へコピーされ、
/// `(ptr, len) -> ()` の WIT canonical import へ渡される。memory は呼び出し中だけ借用され、
/// host が write error を返した場合は trap として実行を終了する。
pub fn emit_wasm_wasmgc_component_output(module: &Module) -> Result<Vec<u8>, CodegenError> {
    emit_wasm_wasmgc_internal(module, true, false)
}

/// L# IR を `wasi:cli/run` export と Component Model の output `list<u8>` canonical ABI を
/// 持つ WasmGC core module へ変換する。
///
/// custom `wasmgc-output` world を command Component へ昇格するため、module 内の `main` を
/// 呼び出して i32 の成功 exit code を返す canonical run wrapper を追加する。WASI の未使用
/// interface を core module へ暗黙に import することはしない。
pub fn emit_wasm_wasmgc_component_cli(module: &Module) -> Result<Vec<u8>, CodegenError> {
    emit_wasm_wasmgc_internal(module, true, true)
}

fn emit_wasm_wasmgc_internal(
    module: &Module,
    component_output: bool,
    component_cli: bool,
) -> Result<Vec<u8>, CodegenError> {
    let print_string_import = module_uses_print_string(module);
    if component_output && !print_string_import {
        return Err(codegen_error(
            "Component output backend は print-string を使用する module が必要です",
        ));
    }
    if component_cli
        && !module
            .functions
            .iter()
            .any(|function| function.name == "main")
    {
        return Err(codegen_error(
            "WasmGC CLI Component backend は main 関数を必要とします",
        ));
    }
    validate_module(module, print_string_import)?;

    let mut wasm_module = wasm_encoder::Module::new();
    let mut types = TypeSection::new();
    let mut import_type_indices = Vec::with_capacity(module.imports.len());
    let mut function_type_indices = Vec::with_capacity(module.functions.len());
    let mut print_string_type_index = None;
    let mut component_output_type_index = None;
    let mut component_cli_run_type_index = None;
    let synthetic_import_offset = u32::from(print_string_import);

    if module_uses_typed_funcref(module) {
        let mut recursive_types = Vec::new();
        for gc_type in &module.gc_types {
            recursive_types.push(wasm_gc_gc_subtype(gc_type, synthetic_import_offset));
        }
        for import in &module.imports {
            import_type_indices.push(recursive_types.len() as u32);
            recursive_types.push(wasm_gc_function_subtype(
                &import.params,
                &[import.result],
                synthetic_import_offset,
            ));
        }
        if print_string_import {
            let string_type_index = string_array_type_index(module)?;
            let type_index = recursive_types.len() as u32;
            recursive_types.push(wasm_gc_function_subtype(
                &[IrType::Ref(string_type_index)],
                &[],
                synthetic_import_offset,
            ));
            print_string_type_index = Some((type_index, string_type_index));
        }
        if component_output {
            let type_index = recursive_types.len() as u32;
            recursive_types.push(wasm_gc_function_subtype(
                &[IrType::I32, IrType::I32],
                &[],
                synthetic_import_offset,
            ));
            component_output_type_index = Some(type_index);
        }
        for function in &module.functions {
            function_type_indices.push(recursive_types.len() as u32);
            recursive_types.push(wasm_gc_function_subtype(
                &function.params,
                &[function.result],
                synthetic_import_offset,
            ));
        }
        if component_cli {
            let type_index = recursive_types.len() as u32;
            recursive_types.push(wasm_gc_function_subtype(
                &[],
                &[IrType::I32],
                synthetic_import_offset,
            ));
            component_cli_run_type_index = Some(type_index);
        }
        types.ty().rec(recursive_types);
    } else {
        for gc_type in &module.gc_types {
            match &gc_type.kind {
                GcTypeKind::Struct(fields) => {
                    let fields = fields
                        .iter()
                        .map(|field| FieldType {
                            element_type: StorageType::Val(wasm_gc_valtype(
                                field.ty,
                                synthetic_import_offset,
                            )),
                            mutable: field.mutable,
                        })
                        .collect::<Vec<_>>();
                    types.ty().struct_(fields);
                }
                GcTypeKind::Array(element_type) => {
                    types.ty().array(
                        &StorageType::Val(wasm_gc_valtype(*element_type, synthetic_import_offset)),
                        true,
                    );
                }
                GcTypeKind::PackedByteArray => {
                    types.ty().array(&StorageType::I8, true);
                }
            }
        }
        for import in &module.imports {
            import_type_indices.push(types.len());
            types.ty().function(
                import
                    .params
                    .iter()
                    .copied()
                    .map(|ty| wasm_gc_valtype(ty, synthetic_import_offset)),
                [wasm_gc_valtype(import.result, synthetic_import_offset)],
            );
        }
        if print_string_import {
            let string_type_index = string_array_type_index(module)?;
            let type_index = types.len();
            types.ty().function(
                [ValType::Ref(RefType {
                    nullable: true,
                    heap_type: HeapType::Concrete(string_type_index),
                })],
                [],
            );
            print_string_type_index = Some((type_index, string_type_index));
        }
        if component_output {
            let type_index = types.len();
            types.ty().function([ValType::I32, ValType::I32], []);
            component_output_type_index = Some(type_index);
        }
        for function in &module.functions {
            function_type_indices.push(types.len());
            types.ty().function(
                function
                    .params
                    .iter()
                    .copied()
                    .map(|ty| wasm_gc_valtype(ty, synthetic_import_offset)),
                [wasm_gc_valtype(function.result, synthetic_import_offset)],
            );
        }
        if component_cli {
            let type_index = types.len();
            types.ty().function([], [ValType::I32]);
            component_cli_run_type_index = Some(type_index);
        }
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
    if let Some(type_index) = component_output_type_index {
        imports.import(
            COMPONENT_OUTPUT_MODULE,
            COMPONENT_OUTPUT_NAME,
            EntityType::Function(type_index),
        );
    } else if let Some((type_index, _)) = print_string_type_index {
        imports.import("env", "print-string", EntityType::Function(type_index));
    }
    if !module.imports.is_empty() || print_string_import {
        wasm_module.section(&imports);
    }

    let mut functions = FunctionSection::new();
    for type_index in function_type_indices {
        functions.function(type_index);
    }
    if let Some(type_index) = component_cli_run_type_index {
        functions.function(type_index);
    }
    if !module.functions.is_empty() || component_cli {
        wasm_module.section(&functions);
    }

    if component_output {
        let mut memories = MemorySection::new();
        memories.memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        });
        wasm_module.section(&memories);
    }

    let mut exports = ExportSection::new();
    let import_count = module.imports.len() as u32 + u32::from(print_string_import);
    if component_output {
        exports.export("memory", ExportKind::Memory, 0);
    }
    for (index, function) in module.functions.iter().enumerate() {
        if function.is_export {
            exports.export(
                &function.name,
                ExportKind::Func,
                import_count + index as u32,
            );
        }
    }
    if component_cli {
        let run_index = import_count + module.functions.len() as u32;
        exports.export("wasi:cli/run@0.2.3#run", ExportKind::Func, run_index);
    }
    if !module.functions.is_empty() || component_cli {
        wasm_module.section(&exports);
    }

    let referenced_funcrefs = module
        .functions
        .iter()
        .flat_map(|function| function.body.iter())
        .filter_map(|instruction| match instruction {
            Instruction::RefFunc(index) => Some(*index),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !referenced_funcrefs.is_empty() {
        let synthetic_import_offset = u32::from(print_string_import);
        let referenced_funcrefs = referenced_funcrefs
            .into_iter()
            .map(|index| index + synthetic_import_offset)
            .collect::<Vec<_>>();
        let mut elements = ElementSection::new();
        elements.declared(Elements::Functions(Cow::Owned(referenced_funcrefs)));
        wasm_module.section(&elements);
    }

    let mut code = CodeSection::new();
    for function in &module.functions {
        let mut locals = function
            .locals
            .iter()
            .copied()
            .map(|ty| (1, wasm_gc_valtype(ty, u32::from(print_string_import))))
            .collect::<Vec<_>>();
        let output_locals = if component_output {
            let array_type_index = string_array_type_index(module)?;
            let base = function.params.len() as u32 + function.locals.len() as u32;
            locals.push((
                1,
                ValType::Ref(RefType {
                    nullable: true,
                    heap_type: HeapType::Concrete(array_type_index),
                }),
            ));
            locals.push((3, ValType::I32));
            Some(ComponentOutputLocals {
                array: base,
                ptr: base + 1,
                len: base + 2,
                index: base + 3,
            })
        } else {
            None
        };
        let mut wasm_function = Function::new(locals);
        let emit_options = WasmGcEmitOptions {
            module,
            function_count: module.functions.len(),
            import_count,
            print_string_import,
            component_output_import_index: component_output_type_index
                .map(|_| module.imports.len() as u32),
            output_locals,
        };
        emit_wasm_gc_instructions(&mut wasm_function, &function.body, &emit_options)?;
        wasm_function.instruction(&wasm_encoder::Instruction::End);
        code.function(&wasm_function);
    }
    if component_cli {
        let main_index = module
            .functions
            .iter()
            .rposition(|function| function.name == "main")
            .ok_or_else(|| codegen_error("WasmGC CLI Component の main 関数がありません"))?;
        let mut run_function = Function::new(vec![]);
        run_function.instruction(&wasm_encoder::Instruction::Call(
            import_count + main_index as u32,
        ));
        run_function.instruction(&wasm_encoder::Instruction::Drop);
        run_function.instruction(&wasm_encoder::Instruction::I32Const(0));
        run_function.instruction(&wasm_encoder::Instruction::End);
        code.function(&run_function);
    }
    if !module.functions.is_empty() || component_cli {
        wasm_module.section(&code);
    }

    Ok(wasm_module.finish())
}

#[derive(Debug, Clone, Copy)]
struct ComponentOutputLocals {
    array: u32,
    ptr: u32,
    len: u32,
    index: u32,
}

struct WasmGcEmitOptions<'a> {
    module: &'a Module,
    function_count: usize,
    import_count: u32,
    print_string_import: bool,
    component_output_import_index: Option<u32>,
    output_locals: Option<ComponentOutputLocals>,
}

fn wasm_gc_valtype(ty: IrType, synthetic_import_offset: u32) -> ValType {
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

fn wasm_gc_gc_subtype(gc_type: &GcTypeDef, synthetic_import_offset: u32) -> SubType {
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

fn wasm_gc_function_subtype(
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

fn module_uses_typed_funcref(module: &Module) -> bool {
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

fn module_uses_print_string(module: &Module) -> bool {
    module.functions.iter().any(|function| {
        function
            .body
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Call(PRINT_STRING_RUNTIME_INDEX)))
    })
}

fn string_array_type_index(module: &Module) -> Result<u32, CodegenError> {
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

fn validate_module(module: &Module, print_string_import: bool) -> Result<(), CodegenError> {
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

    let array_type_index = string_array_type_index(module)?;

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
    function.instruction(&W::If(wasm_encoder::BlockType::Empty));
    function.instruction(&W::Unreachable);
    function.instruction(&W::End);

    function.instruction(&W::I32Const(0));
    function.instruction(&W::LocalSet(locals.index));
    function.instruction(&W::Block(wasm_encoder::BlockType::Empty));
    function.instruction(&W::Loop(wasm_encoder::BlockType::Empty));
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
    function.instruction(&W::I32Store8(wasm_encoder::MemArg {
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
