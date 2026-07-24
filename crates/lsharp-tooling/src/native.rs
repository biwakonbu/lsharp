use std::path::Path;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use std::path::PathBuf;

use lsharp_ir::Module;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(crate) fn compile_native_executable(module: &Module, output_path: &Path) -> miette::Result<()> {
    let asm = native_module_assembly(module)?;
    let asm_path = native_temp_asm_path(output_path)?;
    let temporary_output_path = native_temp_output_path(output_path)?;

    std::fs::write(&asm_path, asm).map_err(|e| miette::miette!("{}: {}", asm_path.display(), e))?;
    let linker_output = match std::process::Command::new("cc")
        .arg(&asm_path)
        .arg("-o")
        .arg(&temporary_output_path)
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            let _ = std::fs::remove_file(&asm_path);
            let _ = std::fs::remove_file(&temporary_output_path);
            return Err(miette::miette!("native linker 起動失敗: {error}"));
        }
    };
    let _ = std::fs::remove_file(&asm_path);

    if !linker_output.status.success() {
        let _ = std::fs::remove_file(&temporary_output_path);
        let stderr = String::from_utf8_lossy(&linker_output.stderr);
        return Err(miette::miette!(
            "native linker が失敗しました: status={}; stderr={stderr}",
            linker_output.status
        ));
    }

    if let Err(error) = lsharp_wasm::component_adapter::sync_artifact_file(&temporary_output_path) {
        let _ = std::fs::remove_file(&temporary_output_path);
        return Err(miette::miette!(
            "native artifact file の同期に失敗しました ({}): {error}",
            temporary_output_path.display()
        ));
    }

    std::fs::rename(&temporary_output_path, output_path).map_err(|error| {
        let _ = std::fs::remove_file(&temporary_output_path);
        miette::miette!(
            "native artifact の atomic replacement に失敗しました ({}): {error}",
            output_path.display()
        )
    })?;

    lsharp_wasm::component_adapter::sync_artifact_parent(output_path).map_err(|error| {
        miette::miette!(
            "native artifact parent directory の同期に失敗しました ({}): {error}",
            output_path.display()
        )
    })?;

    Ok(())
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
pub(crate) fn compile_native_executable(
    _module: &Module,
    output_path: &Path,
) -> miette::Result<()> {
    Err(miette::miette!(
        "native backend は未サポートです。この host の Rust driver native path は aarch64-apple-darwin のみ生成できます: {}",
        output_path.display()
    ))
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_main_index(module: &Module) -> miette::Result<usize> {
    module
        .functions
        .iter()
        .position(|func| func.name == "main" && func.is_export)
        .or_else(|| module.functions.iter().position(|func| func.name == "main"))
        .ok_or_else(|| miette::miette!("native backend には main 関数が必要です"))
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const NATIVE_IR_IMPORT_COUNT: u32 = 17;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_module_assembly(module: &Module) -> miette::Result<String> {
    let reachable_functions = native_reachable_function_indexes(module)?;
    let mut asm = String::from(
        ".section __TEXT,__cstring\n\
         L_lsharp_fmt_i64:\n\
             .asciz \"%lld\\n\"\n\
         .data\n\
         .p2align 3\n\
         _lsharp_heap_next:\n\
             .quad 0\n\
         .zerofill __DATA,__bss,_lsharp_heap,1048576,4\n\
         .text\n",
    );
    for function_index in reachable_functions {
        let function = &module.functions[function_index];
        NativeFunctionEmitter::new(module, function_index, function).emit(&mut asm)?;
    }
    Ok(asm)
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_reachable_function_indexes(module: &Module) -> miette::Result<Vec<usize>> {
    let main_index = native_main_index(module)?;
    let mut reachable = vec![false; module.functions.len()];
    let mut pending = vec![main_index];

    while let Some(function_index) = pending.pop() {
        if reachable[function_index] {
            continue;
        }
        reachable[function_index] = true;

        for instruction in &module.functions[function_index].body {
            let lsharp_ir::Instruction::Call(index) = instruction else {
                continue;
            };
            if *index < NATIVE_IR_IMPORT_COUNT {
                continue;
            }
            let callee_index = (*index - NATIVE_IR_IMPORT_COUNT) as usize;
            if callee_index >= module.functions.len() {
                continue;
            }
            pending.push(callee_index);
        }
    }

    Ok(reachable
        .into_iter()
        .enumerate()
        .filter_map(|(index, is_reachable)| is_reachable.then_some(index))
        .collect())
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_function_label(module: &Module, function_index: usize) -> String {
    let function = &module.functions[function_index];
    if function.name == "main" && function.is_export {
        "_main".to_string()
    } else {
        format!(
            "_lsharp_fn_{function_index}_{}",
            native_sanitize_symbol(&function.name)
        )
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_sanitize_symbol(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
struct NativeIfFrame {
    else_label: String,
    end_label: String,
    entry_depth: usize,
    then_depth: Option<usize>,
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
struct NativeFunctionEmitter<'a> {
    module: &'a Module,
    function_index: usize,
    function: &'a lsharp_ir::Function,
    stack_depth: usize,
    label_counter: usize,
    if_stack: Vec<NativeIfFrame>,
    return_label: String,
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
impl<'a> NativeFunctionEmitter<'a> {
    fn new(module: &'a Module, function_index: usize, function: &'a lsharp_ir::Function) -> Self {
        let return_label = format!(
            "L_lsharp_return_{function_index}_{}",
            native_sanitize_symbol(&function.name)
        );
        Self {
            module,
            function_index,
            function,
            stack_depth: 0,
            label_counter: 0,
            if_stack: Vec::new(),
            return_label,
        }
    }

    fn emit(mut self, asm: &mut String) -> miette::Result<()> {
        if self.function.params.len() > 8 {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装は 8 引数までの関数のみ対応しています: {}",
                self.function.name
            ));
        }
        if !matches!(
            self.function.result,
            lsharp_ir::IrType::I64 | lsharp_ir::IrType::I32
        ) {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装は整数を返す関数のみ対応しています: {}",
                self.function.name
            ));
        }

        let label = native_function_label(self.module, self.function_index);
        asm.push_str(&format!(
            ".globl {label}\n\
             .p2align 2\n\
             {label}:\n"
        ));
        asm.push_str("    stp x29, x30, [sp, #-16]!\n");
        asm.push_str("    mov x29, sp\n");
        let frame_size = native_frame_size(self.function);
        if frame_size > 0 {
            asm.push_str(&format!("    sub sp, sp, #{frame_size}\n"));
        }
        for param_idx in 0..self.function.params.len() {
            asm.push_str(&format!(
                "    str x{param_idx}, [x29, #{}]\n",
                native_local_offset(param_idx as u32)
            ));
        }

        let body = self.function.body.clone();
        for instruction in &body {
            self.emit_instruction(asm, instruction)?;
        }
        if !self.if_stack.is_empty() {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装で if/end の対応が閉じていません: {}",
                self.function.name
            ));
        }
        if self.stack_depth != 1 {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装は関数終了時に戻り値が1つ必要です: {} stack_depth={}",
                self.function.name,
                self.stack_depth
            ));
        }

        native_emit_pop(asm, "x0");
        self.emit_epilogue(asm);
        Ok(())
    }

    fn emit_instruction(
        &mut self,
        asm: &mut String,
        instruction: &lsharp_ir::Instruction,
    ) -> miette::Result<()> {
        match instruction {
            lsharp_ir::Instruction::I64Const(value) => {
                native_emit_i64_const(asm, "x9", *value);
                native_emit_push(asm, "x9");
                self.stack_depth += 1;
            }
            lsharp_ir::Instruction::I32Const(value) => {
                native_emit_i64_const(asm, "x9", i64::from(*value));
                native_emit_push(asm, "x9");
                self.stack_depth += 1;
            }
            lsharp_ir::Instruction::LocalGet(index) => {
                self.ensure_local(*index)?;
                asm.push_str(&format!(
                    "    ldr x9, [x29, #{}]\n",
                    native_local_offset(*index)
                ));
                native_emit_push(asm, "x9");
                self.stack_depth += 1;
            }
            lsharp_ir::Instruction::LocalSet(index) => {
                self.ensure_local(*index)?;
                self.pop(asm, "x9")?;
                asm.push_str(&format!(
                    "    str x9, [x29, #{}]\n",
                    native_local_offset(*index)
                ));
            }
            lsharp_ir::Instruction::LocalTee(index) => {
                self.ensure_local(*index)?;
                self.pop(asm, "x9")?;
                asm.push_str(&format!(
                    "    str x9, [x29, #{}]\n",
                    native_local_offset(*index)
                ));
                native_emit_push(asm, "x9");
                self.stack_depth += 1;
            }
            lsharp_ir::Instruction::I64Add | lsharp_ir::Instruction::I32Add => {
                self.emit_binary_op(asm, "add x9, x9, x10")?;
            }
            lsharp_ir::Instruction::I64Sub | lsharp_ir::Instruction::I32Sub => {
                self.emit_binary_op(asm, "sub x9, x9, x10")?;
            }
            lsharp_ir::Instruction::I64Mul | lsharp_ir::Instruction::I32Mul => {
                self.emit_binary_op(asm, "mul x9, x9, x10")?;
            }
            lsharp_ir::Instruction::I64Div => {
                self.emit_binary_op(asm, "sdiv x9, x9, x10")?;
            }
            lsharp_ir::Instruction::I64Rem => {
                self.emit_binary_op(asm, "sdiv x11, x9, x10\n    msub x9, x11, x10, x9")?;
            }
            lsharp_ir::Instruction::I64Eq => self.emit_compare(asm, "eq")?,
            lsharp_ir::Instruction::I64Ne => self.emit_compare(asm, "ne")?,
            lsharp_ir::Instruction::I64LtS => self.emit_compare(asm, "lt")?,
            lsharp_ir::Instruction::I64GtS => self.emit_compare(asm, "gt")?,
            lsharp_ir::Instruction::I64LeS => self.emit_compare(asm, "le")?,
            lsharp_ir::Instruction::I64GeS => self.emit_compare(asm, "ge")?,
            lsharp_ir::Instruction::I32GtU => self.emit_compare(asm, "hi")?,
            lsharp_ir::Instruction::I32GeU => self.emit_compare(asm, "hs")?,
            lsharp_ir::Instruction::I32Eqz => {
                self.pop(asm, "x9")?;
                asm.push_str("    cmp x9, #0\n");
                asm.push_str("    cset x9, eq\n");
                native_emit_push(asm, "x9");
                self.stack_depth += 1;
            }
            lsharp_ir::Instruction::I32And | lsharp_ir::Instruction::I64And => {
                self.emit_binary_op(asm, "and x9, x9, x10")?;
            }
            lsharp_ir::Instruction::I32Or | lsharp_ir::Instruction::I64Or => {
                self.emit_binary_op(asm, "orr x9, x9, x10")?;
            }
            lsharp_ir::Instruction::I64Xor => {
                self.emit_binary_op(asm, "eor x9, x9, x10")?;
            }
            lsharp_ir::Instruction::I64Shl | lsharp_ir::Instruction::I32Shl => {
                self.emit_binary_op(asm, "lsl x9, x9, x10")?;
            }
            lsharp_ir::Instruction::I64ShrU | lsharp_ir::Instruction::I32ShrU => {
                self.emit_binary_op(asm, "lsr x9, x9, x10")?;
            }
            lsharp_ir::Instruction::I64ExtendI32S
            | lsharp_ir::Instruction::I64ExtendI32U
            | lsharp_ir::Instruction::I32WrapI64 => {}
            lsharp_ir::Instruction::Call(index) => self.emit_call(asm, *index)?,
            lsharp_ir::Instruction::CallImport(index) => self.emit_call(asm, *index)?,
            lsharp_ir::Instruction::If(_) | lsharp_ir::Instruction::IfEmpty => {
                self.emit_if_start(asm)?;
            }
            lsharp_ir::Instruction::Else => self.emit_else(asm)?,
            lsharp_ir::Instruction::End => self.emit_end(asm)?,
            lsharp_ir::Instruction::Drop => {
                self.pop(asm, "x9")?;
            }
            lsharp_ir::Instruction::Return => {
                self.pop(asm, "x0")?;
                asm.push_str(&format!("    b {}\n", self.return_label));
            }
            lsharp_ir::Instruction::StructNew(type_index) => {
                self.emit_struct_new(asm, *type_index)?;
            }
            lsharp_ir::Instruction::StructGet(type_index, field_index) => {
                self.emit_struct_get(asm, *type_index, *field_index)?;
            }
            lsharp_ir::Instruction::StructSet(type_index, field_index) => {
                self.emit_struct_set(asm, *type_index, *field_index)?;
            }
            lsharp_ir::Instruction::I32Load { offset } => {
                self.emit_i32_load(asm, *offset)?;
            }
            lsharp_ir::Instruction::I32Load8U { offset } => {
                self.emit_i32_load8_u(asm, *offset)?;
            }
            lsharp_ir::Instruction::I32Store { offset } => {
                self.emit_i32_store(asm, *offset)?;
            }
            lsharp_ir::Instruction::I32Store8 { offset } => {
                self.emit_i32_store8(asm, *offset)?;
            }
            lsharp_ir::Instruction::I64Load { offset } => {
                self.emit_i64_load(asm, *offset)?;
            }
            lsharp_ir::Instruction::I64Store { offset } => {
                self.emit_i64_store(asm, *offset)?;
            }
            lsharp_ir::Instruction::Unreachable => {
                asm.push_str("    brk #1\n");
                native_emit_i64_const(asm, "x9", 0);
                native_emit_push(asm, "x9");
                self.stack_depth += 1;
            }
            unsupported => {
                return Err(miette::miette!(
                    "native backend の Apple Silicon 実装は未対応 IR を含む関数をまだ生成できません: {} {unsupported}",
                    self.function.name
                ));
            }
        }
        Ok(())
    }

    fn emit_binary_op(&mut self, asm: &mut String, operation: &str) -> miette::Result<()> {
        self.pop(asm, "x10")?;
        self.pop(asm, "x9")?;
        asm.push_str("    ");
        asm.push_str(operation);
        asm.push('\n');
        native_emit_push(asm, "x9");
        self.stack_depth += 1;
        Ok(())
    }

    fn emit_compare(&mut self, asm: &mut String, condition: &str) -> miette::Result<()> {
        self.pop(asm, "x10")?;
        self.pop(asm, "x9")?;
        asm.push_str("    cmp x9, x10\n");
        asm.push_str(&format!("    cset x9, {condition}\n"));
        native_emit_push(asm, "x9");
        self.stack_depth += 1;
        Ok(())
    }

    fn emit_call(&mut self, asm: &mut String, index: u32) -> miette::Result<()> {
        match index {
            0 => {
                self.pop(asm, "x9")?;
                asm.push_str("    adrp x0, L_lsharp_fmt_i64@PAGE\n");
                asm.push_str("    add x0, x0, L_lsharp_fmt_i64@PAGEOFF\n");
                asm.push_str("    sub sp, sp, #16\n");
                asm.push_str("    str x9, [sp]\n");
                asm.push_str("    bl _printf\n");
                asm.push_str("    add sp, sp, #16\n");
                return Ok(());
            }
            1 => {
                self.pop(asm, "x9")?;
                native_emit_heap_alloc(asm);
                native_emit_push(asm, "x9");
                self.stack_depth += 1;
                return Ok(());
            }
            5 => {
                self.pop(asm, "x0")?;
                asm.push_str("    bl _exit\n");
                return Ok(());
            }
            14 => {
                self.pop(asm, "x9")?;
                native_emit_i64_const(asm, "x9", 0);
                native_emit_push(asm, "x9");
                self.stack_depth += 1;
                return Ok(());
            }
            15 => {
                native_emit_i64_const(asm, "x9", 0);
                native_emit_push(asm, "x9");
                self.stack_depth += 1;
                return Ok(());
            }
            16 => {
                self.pop(asm, "x10")?;
                self.pop(asm, "x9")?;
                native_emit_push(asm, "x9");
                self.stack_depth += 1;
                return Ok(());
            }
            _ => {}
        }
        if index < NATIVE_IR_IMPORT_COUNT {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装は import/runtime call をまだ生成できません: call {index}"
            ));
        }
        let function_index = (index - NATIVE_IR_IMPORT_COUNT) as usize;
        let Some(callee) = self.module.functions.get(function_index) else {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装で call index が関数範囲外です: call {index}"
            ));
        };
        if callee.params.len() > 8 {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装は 8 引数までの呼び出しのみ対応しています: {}",
                callee.name
            ));
        }
        for arg_index in (0..callee.params.len()).rev() {
            self.pop(asm, &format!("x{arg_index}"))?;
        }
        let label = native_function_label(self.module, function_index);
        asm.push_str(&format!("    bl {label}\n"));
        native_emit_push(asm, "x0");
        self.stack_depth += 1;
        Ok(())
    }

    fn emit_struct_new(&mut self, asm: &mut String, type_index: u32) -> miette::Result<()> {
        let field_count = self.native_struct_field_count(type_index)?;
        native_emit_i64_const(asm, "x9", (field_count * 8) as i64);
        native_emit_heap_alloc(asm);
        asm.push_str("    mov x13, x9\n");
        native_emit_heap_base_plus_offset(asm, "x13", "x11");
        for field_index in (0..field_count).rev() {
            self.pop(asm, "x10")?;
            asm.push_str(&format!("    str x10, [x11, #{}]\n", field_index * 8));
        }
        native_emit_push(asm, "x13");
        self.stack_depth += 1;
        Ok(())
    }

    fn emit_struct_get(
        &mut self,
        asm: &mut String,
        type_index: u32,
        field_index: u32,
    ) -> miette::Result<()> {
        let field_count = self.native_struct_field_count(type_index)?;
        if field_index as usize >= field_count {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装で struct field index が範囲外です: type={type_index} field={field_index}"
            ));
        }
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset(asm, "x9", "x11");
        asm.push_str(&format!("    ldr x9, [x11, #{}]\n", field_index * 8));
        native_emit_push(asm, "x9");
        self.stack_depth += 1;
        Ok(())
    }

    fn emit_struct_set(
        &mut self,
        asm: &mut String,
        type_index: u32,
        field_index: u32,
    ) -> miette::Result<()> {
        let field_count = self.native_struct_field_count(type_index)?;
        if field_index as usize >= field_count {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装で struct field index が範囲外です: type={type_index} field={field_index}"
            ));
        }
        self.pop(asm, "x10")?;
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset(asm, "x9", "x11");
        asm.push_str(&format!("    str x10, [x11, #{}]\n", field_index * 8));
        Ok(())
    }

    fn emit_i32_load(&mut self, asm: &mut String, offset: u32) -> miette::Result<()> {
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset_and_memarg(asm, "x9", "x11", offset);
        asm.push_str("    ldr w9, [x11]\n");
        asm.push_str("    uxtw x9, w9\n");
        native_emit_push(asm, "x9");
        self.stack_depth += 1;
        Ok(())
    }

    fn emit_i32_load8_u(&mut self, asm: &mut String, offset: u32) -> miette::Result<()> {
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset_and_memarg(asm, "x9", "x11", offset);
        asm.push_str("    ldrb w9, [x11]\n");
        asm.push_str("    uxtw x9, w9\n");
        native_emit_push(asm, "x9");
        self.stack_depth += 1;
        Ok(())
    }

    fn emit_i64_load(&mut self, asm: &mut String, offset: u32) -> miette::Result<()> {
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset_and_memarg(asm, "x9", "x11", offset);
        asm.push_str("    ldr x9, [x11]\n");
        native_emit_push(asm, "x9");
        self.stack_depth += 1;
        Ok(())
    }

    fn emit_i32_store(&mut self, asm: &mut String, offset: u32) -> miette::Result<()> {
        self.pop(asm, "x10")?;
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset_and_memarg(asm, "x9", "x11", offset);
        asm.push_str("    str w10, [x11]\n");
        Ok(())
    }

    fn emit_i32_store8(&mut self, asm: &mut String, offset: u32) -> miette::Result<()> {
        self.pop(asm, "x10")?;
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset_and_memarg(asm, "x9", "x11", offset);
        asm.push_str("    strb w10, [x11]\n");
        Ok(())
    }

    fn emit_i64_store(&mut self, asm: &mut String, offset: u32) -> miette::Result<()> {
        self.pop(asm, "x10")?;
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset_and_memarg(asm, "x9", "x11", offset);
        asm.push_str("    str x10, [x11]\n");
        Ok(())
    }

    fn native_struct_field_count(&self, type_index: u32) -> miette::Result<usize> {
        let Some(gc_type) = self.module.gc_types.get(type_index as usize) else {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装で struct type index が範囲外です: type={type_index}"
            ));
        };
        match &gc_type.kind {
            lsharp_ir::GcTypeKind::Struct(fields) => Ok(fields.len()),
            lsharp_ir::GcTypeKind::Array(_) | lsharp_ir::GcTypeKind::PackedByteArray => {
                Err(miette::miette!(
                    "native backend の Apple Silicon 実装は array GC type をまだ生成できません: {}",
                    gc_type.name
                ))
            }
        }
    }

    fn emit_if_start(&mut self, asm: &mut String) -> miette::Result<()> {
        let else_label = self.next_label("else");
        let end_label = self.next_label("endif");
        self.pop(asm, "x9")?;
        let entry_depth = self.stack_depth;
        asm.push_str("    cmp x9, #0\n");
        asm.push_str(&format!("    b.eq {else_label}\n"));
        self.if_stack.push(NativeIfFrame {
            else_label,
            end_label,
            entry_depth,
            then_depth: None,
        });
        Ok(())
    }

    fn emit_else(&mut self, asm: &mut String) -> miette::Result<()> {
        let Some(frame) = self.if_stack.last_mut() else {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装で else に対応する if がありません"
            ));
        };
        frame.then_depth = Some(self.stack_depth);
        asm.push_str(&format!("    b {}\n", frame.end_label));
        asm.push_str(&format!("{}:\n", frame.else_label));
        self.stack_depth = frame.entry_depth;
        Ok(())
    }

    fn emit_end(&mut self, asm: &mut String) -> miette::Result<()> {
        let Some(frame) = self.if_stack.pop() else {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装で end に対応する block がありません"
            ));
        };
        if let Some(then_depth) = frame.then_depth {
            if self.stack_depth != then_depth {
                return Err(miette::miette!(
                    "native backend の Apple Silicon 実装で if 分岐の stack depth が一致しません: then={then_depth} else={}",
                    self.stack_depth
                ));
            }
            asm.push_str(&format!("{}:\n", frame.end_label));
        } else {
            asm.push_str(&format!("{}:\n", frame.else_label));
            asm.push_str(&format!("{}:\n", frame.end_label));
            self.stack_depth = frame.entry_depth;
        }
        Ok(())
    }

    fn pop(&mut self, asm: &mut String, register: &str) -> miette::Result<()> {
        if self.stack_depth == 0 {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装でスタック値が不足しています: {}",
                self.function.name
            ));
        }
        native_emit_pop(asm, register);
        self.stack_depth -= 1;
        Ok(())
    }

    fn emit_epilogue(&self, asm: &mut String) {
        asm.push_str(&format!("{}:\n", self.return_label));
        asm.push_str("    mov sp, x29\n");
        asm.push_str("    ldp x29, x30, [sp], #16\n");
        asm.push_str("    ret\n");
    }

    fn ensure_local(&self, index: u32) -> miette::Result<()> {
        let local_count = self.function.params.len() + self.function.locals.len();
        if index as usize >= local_count {
            return Err(miette::miette!(
                "native backend の Apple Silicon 実装で local index が範囲外です: {} local={index} local_count={local_count}",
                self.function.name
            ));
        }
        Ok(())
    }

    fn next_label(&mut self, kind: &str) -> String {
        let label = format!(
            "L_lsharp_{}_{}_{}_{kind}",
            self.function_index,
            native_sanitize_symbol(&self.function.name),
            self.label_counter
        );
        self.label_counter += 1;
        label
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_frame_size(function: &lsharp_ir::Function) -> usize {
    let slots = function.params.len() + function.locals.len();
    let bytes = slots * 8;
    bytes.div_ceil(16) * 16
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_local_offset(index: u32) -> i32 {
    -8 * (index as i32 + 1)
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_emit_i64_const(asm: &mut String, register: &str, value: i64) {
    let raw = value as u64;
    let chunks = [
        raw & 0xffff,
        (raw >> 16) & 0xffff,
        (raw >> 32) & 0xffff,
        (raw >> 48) & 0xffff,
    ];
    asm.push_str(&format!("    movz {register}, #{}\n", chunks[0]));
    for (idx, chunk) in chunks.iter().enumerate().skip(1) {
        if *chunk != 0 {
            asm.push_str(&format!(
                "    movk {register}, #{chunk}, lsl #{}\n",
                idx * 16
            ));
        }
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_emit_heap_alloc(asm: &mut String) {
    asm.push_str("    add x9, x9, #7\n");
    asm.push_str("    lsr x9, x9, #3\n");
    asm.push_str("    lsl x9, x9, #3\n");
    asm.push_str("    adrp x10, _lsharp_heap_next@PAGE\n");
    asm.push_str("    add x10, x10, _lsharp_heap_next@PAGEOFF\n");
    asm.push_str("    ldr x11, [x10]\n");
    asm.push_str("    add x12, x11, x9\n");
    asm.push_str("    str x12, [x10]\n");
    asm.push_str("    mov x9, x11\n");
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_emit_heap_base_plus_offset(asm: &mut String, offset_register: &str, target: &str) {
    let wide_register = offset_register;
    let narrow_register = offset_register.replacen('x', "w", 1);
    asm.push_str(&format!("    uxtw {wide_register}, {narrow_register}\n"));
    asm.push_str(&format!("    adrp {target}, _lsharp_heap@PAGE\n"));
    asm.push_str(&format!(
        "    add {target}, {target}, _lsharp_heap@PAGEOFF\n"
    ));
    asm.push_str(&format!("    add {target}, {target}, {wide_register}\n"));
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_emit_heap_base_plus_offset_and_memarg(
    asm: &mut String,
    offset_register: &str,
    target: &str,
    memarg_offset: u32,
) {
    native_emit_heap_base_plus_offset(asm, offset_register, target);
    if memarg_offset > 0 {
        native_emit_i64_const(asm, "x12", i64::from(memarg_offset));
        asm.push_str(&format!("    add {target}, {target}, x12\n"));
    }
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_emit_push(asm: &mut String, register: &str) {
    asm.push_str("    sub sp, sp, #16\n");
    asm.push_str(&format!("    str {register}, [sp]\n"));
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_emit_pop(asm: &mut String, register: &str) {
    asm.push_str(&format!("    ldr {register}, [sp]\n"));
    asm.push_str("    add sp, sp, #16\n");
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_temp_asm_path(output_path: &Path) -> miette::Result<PathBuf> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| miette::miette!("native temp path 作成失敗: {e}"))?
        .as_nanos();
    let output_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("a.out");

    Ok(std::env::temp_dir().join(format!(
        "lsharp-native-{}-{timestamp}-{output_name}.s",
        std::process::id()
    )))
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn native_temp_output_path(output_path: &Path) -> miette::Result<PathBuf> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| miette::miette!("native output temp path 作成失敗: {e}"))?
        .as_nanos();
    let parent = output_path.parent().unwrap_or_else(|| Path::new("."));
    let output_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("a.out");

    Ok(parent.join(format!(
        ".{output_name}.tmp-{}-{timestamp}",
        std::process::id()
    )))
}

#[cfg(all(test, target_os = "macos", target_arch = "aarch64"))]
mod tests {
    use super::*;

    #[test]
    fn native_output_temp_path_is_a_unique_sibling() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_native_output_temp_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock は unix epoch より後であるべき")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("native output test directory を作成できる");
        let output_path = dir.join("demo");
        let temporary_path =
            native_temp_output_path(&output_path).expect("native output の一時 path を作成できる");

        assert_eq!(temporary_path.parent(), output_path.parent());
        let name = temporary_path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("native temporary output name を取得できる");
        assert!(name.starts_with(".demo.tmp-"));
        assert_ne!(temporary_path, output_path);
        assert!(!temporary_path.exists());

        std::fs::remove_dir_all(&dir).expect("native output test directory を削除できる");
    }

    #[test]
    fn native_link_failure_cleans_temporary_output_before_returning() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_native_atomic_failure_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock は unix epoch より後であるべき")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("native failure test directory を作成できる");
        let output_path = dir.join("destination");
        std::fs::create_dir(&output_path)
            .expect("rename failure 用 destination directory を作成できる");
        let module = Module {
            functions: vec![lsharp_ir::Function {
                name: "main".to_string(),
                params: vec![],
                result: lsharp_ir::IrType::I64,
                locals: vec![],
                body: vec![lsharp_ir::Instruction::I64Const(0)],
                is_export: true,
            }],
            gc_types: vec![],
            imports: vec![],
            globals: vec![],
            string_data: vec![],
        };

        let error = compile_native_executable(&module, &output_path)
            .expect_err("directory destination への atomic replacement は失敗するべき");
        assert!(error.to_string().contains("atomic replacement"));
        assert!(output_path.is_dir(), "失敗時も既存 destination を壊さない");
        let temporary_outputs = std::fs::read_dir(&dir)
            .expect("native failure test directory を列挙できる")
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with(".destination.tmp-"))
            })
            .count();
        assert_eq!(
            temporary_outputs, 0,
            "link failure 後に temporary executable を残さない"
        );
        std::fs::remove_dir_all(&dir).expect("native failure test directory を削除できる");
    }
}
