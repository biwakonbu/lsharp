#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use super::*;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[path = "native_emitter/memory.rs"]
mod memory;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
struct NativeIfFrame {
    else_label: String,
    end_label: String,
    entry_depth: usize,
    then_depth: Option<usize>,
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
pub(super) struct NativeFunctionEmitter<'a> {
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
    pub(super) fn new(
        module: &'a Module,
        function_index: usize,
        function: &'a lsharp_ir::Function,
    ) -> Self {
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

    pub(super) fn emit(mut self, asm: &mut String) -> miette::Result<()> {
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
