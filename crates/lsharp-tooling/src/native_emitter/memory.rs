//! Apple Silicon native emitter の struct / linear-memory instruction emission。

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use super::NativeFunctionEmitter;

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
use super::super::{
    native_emit_heap_alloc, native_emit_heap_base_plus_offset,
    native_emit_heap_base_plus_offset_and_memarg, native_emit_i64_const, native_emit_push,
};

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
impl<'a> NativeFunctionEmitter<'a> {
    pub(super) fn emit_struct_new(
        &mut self,
        asm: &mut String,
        type_index: u32,
    ) -> miette::Result<()> {
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

    pub(super) fn emit_struct_get(
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

    pub(super) fn emit_struct_set(
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

    pub(super) fn emit_i32_load(&mut self, asm: &mut String, offset: u32) -> miette::Result<()> {
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset_and_memarg(asm, "x9", "x11", offset);
        asm.push_str("    ldr w9, [x11]\n");
        asm.push_str("    uxtw x9, w9\n");
        native_emit_push(asm, "x9");
        self.stack_depth += 1;
        Ok(())
    }

    pub(super) fn emit_i32_load8_u(&mut self, asm: &mut String, offset: u32) -> miette::Result<()> {
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset_and_memarg(asm, "x9", "x11", offset);
        asm.push_str("    ldrb w9, [x11]\n");
        asm.push_str("    uxtw x9, w9\n");
        native_emit_push(asm, "x9");
        self.stack_depth += 1;
        Ok(())
    }

    pub(super) fn emit_i64_load(&mut self, asm: &mut String, offset: u32) -> miette::Result<()> {
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset_and_memarg(asm, "x9", "x11", offset);
        asm.push_str("    ldr x9, [x11]\n");
        native_emit_push(asm, "x9");
        self.stack_depth += 1;
        Ok(())
    }

    pub(super) fn emit_i32_store(&mut self, asm: &mut String, offset: u32) -> miette::Result<()> {
        self.pop(asm, "x10")?;
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset_and_memarg(asm, "x9", "x11", offset);
        asm.push_str("    str w10, [x11]\n");
        Ok(())
    }

    pub(super) fn emit_i32_store8(&mut self, asm: &mut String, offset: u32) -> miette::Result<()> {
        self.pop(asm, "x10")?;
        self.pop(asm, "x9")?;
        native_emit_heap_base_plus_offset_and_memarg(asm, "x9", "x11", offset);
        asm.push_str("    strb w10, [x11]\n");
        Ok(())
    }

    pub(super) fn emit_i64_store(&mut self, asm: &mut String, offset: u32) -> miette::Result<()> {
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
}
