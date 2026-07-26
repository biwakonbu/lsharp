//! WasmGC backend の pattern lowering

use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::{GcTypeKind, Instruction, IrType};

use super::{FuncCtx, Lower, LowerError};

type WasmGcPatternCheck = (u32, Pattern, Option<String>);

impl Lower {
    /// WasmGC レコードパターンを field 値の sequence として lowering する。
    ///
    /// `lower_wasmgc_pattern_sequence` は ADT の nested constructor/literal と同じく、
    /// field pattern が不一致なら同じ scrutinee の次の arm へ戻す。これにより、
    /// `[{Point x 42} ...] [_ ...]` が不一致を暗黙に受理したり trap で終わったりしない。
    #[allow(clippy::too_many_arguments)]
    pub(super) fn lower_wasmgc_record_pattern_arm(
        &mut self,
        ctx: &mut FuncCtx,
        scrut_local: u32,
        type_name: &str,
        field_pats: &[(String, Pattern)],
        arms: &[MatchArm],
        arm: &MatchArm,
        idx: usize,
    ) -> Result<(), LowerError> {
        let checks = self.collect_wasmgc_record_pattern_checks(
            ctx,
            scrut_local,
            type_name,
            field_pats,
            arm.span,
        )?;
        self.lower_wasmgc_pattern_sequence(ctx, checks, arms, idx, scrut_local)
    }

    fn collect_wasmgc_record_pattern_checks(
        &self,
        ctx: &mut FuncCtx,
        record_local: u32,
        type_name: &str,
        field_pats: &[(String, Pattern)],
        span: Span,
    ) -> Result<Vec<WasmGcPatternCheck>, LowerError> {
        let Some(&gc_type_idx) = self.record_type_indices.get(type_name) else {
            return Err(LowerError::Unsupported {
                msg: format!("WasmGC record type を解決できません: {type_name}"),
                span: Some(span),
            });
        };
        let Some(GcTypeKind::Struct(fields)) = self
            .gc_types
            .get(gc_type_idx as usize)
            .map(|type_def| &type_def.kind)
        else {
            return Err(LowerError::Unsupported {
                msg: format!("WasmGC record type が struct ではありません: {type_name}"),
                span: Some(span),
            });
        };

        let mut checks = Vec::with_capacity(field_pats.len());
        for (field_name, pattern) in field_pats {
            let Some(field_idx) = self.resolve_field_index(type_name, field_name) else {
                return Err(LowerError::Unsupported {
                    msg: format!("WasmGC record field を解決できません: {type_name}.{field_name}"),
                    span: Some(span),
                });
            };
            let field_type = fields
                .get(field_idx as usize)
                .map(|field| field.ty)
                .unwrap_or(IrType::I64);
            ctx.emit(Instruction::LocalGet(record_local));
            ctx.emit(Instruction::StructGet(gc_type_idx, field_idx));
            let field_local = ctx.alloc_local_typed(
                format!("_wasmgc_record_pattern_field_{field_name}"),
                field_type,
            );
            ctx.emit(Instruction::LocalSet(field_local));
            let field_type_name = match field_type {
                IrType::Ref(type_index) => self
                    .gc_types
                    .get(type_index as usize)
                    .map(|type_def| type_def.name.clone()),
                _ => None,
            };
            checks.push((field_local, pattern.clone(), field_type_name));
        }
        Ok(checks)
    }

    pub(super) fn lower_wasmgc_constructor_arm(
        &mut self,
        ctx: &mut FuncCtx,
        scrut_local: u32,
        arms: &[MatchArm],
        idx: usize,
    ) -> Result<(), LowerError> {
        let arm = &arms[idx];
        let Pattern::Constructor(_, name, sub_pats) = &arm.pattern else {
            return Err(LowerError::Unsupported {
                msg: "WasmGC ADT pattern の内部形式が不正です".to_string(),
                span: Some(arm.span),
            });
        };

        let Some(&(gc_type_idx, tag)) = self.adt_variant_indices.get(name) else {
            return Err(LowerError::Unsupported {
                msg: format!("WasmGC ADT variant を解決できません: {name}"),
                span: Some(arm.span),
            });
        };

        ctx.emit(Instruction::LocalGet(scrut_local));
        ctx.emit(Instruction::StructGet(gc_type_idx, 0));
        ctx.emit(Instruction::I64Const(tag as i64));
        ctx.emit(Instruction::I64Eq);
        ctx.emit(Instruction::If(IrType::I64));

        let field_types = self
            .adt_variant_field_types
            .get(name)
            .cloned()
            .unwrap_or_default();
        let field_type_names = self
            .adt_variant_field_type_names
            .get(name)
            .cloned()
            .unwrap_or_default();
        let field_offsets = self
            .adt_variant_field_offsets
            .get(name)
            .cloned()
            .unwrap_or_default();
        let mut checks = Vec::with_capacity(sub_pats.len());
        for (field_idx, pattern) in sub_pats.iter().enumerate() {
            ctx.emit(Instruction::LocalGet(scrut_local));
            let slot_idx = field_offsets
                .get(field_idx)
                .copied()
                .unwrap_or(field_idx as u32);
            ctx.emit(Instruction::StructGet(gc_type_idx, slot_idx + 1));
            let field_type = field_types.get(field_idx).copied().unwrap_or(IrType::I64);
            let field_local =
                ctx.alloc_local_typed(format!("_wasmgc_pattern_field_{field_idx}"), field_type);
            ctx.emit(Instruction::LocalSet(field_local));
            checks.push((
                field_local,
                pattern.clone(),
                field_type_names.get(field_idx).cloned().flatten(),
            ));
        }
        self.lower_wasmgc_pattern_sequence(ctx, checks, arms, idx, scrut_local)?;

        ctx.emit(Instruction::Else);
        if idx == arms.len() - 1 {
            ctx.emit(Instruction::Unreachable);
        } else {
            self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
        }
        ctx.emit(Instruction::End);
        Ok(())
    }

    fn lower_wasmgc_pattern_sequence(
        &mut self,
        ctx: &mut FuncCtx,
        checks: Vec<WasmGcPatternCheck>,
        arms: &[MatchArm],
        idx: usize,
        scrut_local: u32,
    ) -> Result<(), LowerError> {
        let arm = &arms[idx];
        let Some((value_local, pattern, value_type_name)) = checks.first() else {
            return self.lower_arm_body_with_guard(ctx, scrut_local, arms, arm, idx);
        };
        let rest = checks.iter().skip(1).cloned().collect::<Vec<_>>();

        match pattern {
            Pattern::Var(_, name) => {
                ctx.emit(Instruction::LocalGet(*value_local));
                let value_type = ctx
                    .local_types
                    .get(*value_local as usize)
                    .copied()
                    .unwrap_or(IrType::I64);
                let var_local = ctx.alloc_local_typed(name.clone(), value_type);
                ctx.emit(Instruction::LocalSet(var_local));
                if let Some(type_name) = value_type_name {
                    ctx.local_type_names.insert(name.clone(), type_name.clone());
                }
                self.lower_wasmgc_pattern_sequence(ctx, rest, arms, idx, scrut_local)
            }
            Pattern::Wildcard(_) => {
                self.lower_wasmgc_pattern_sequence(ctx, rest, arms, idx, scrut_local)
            }
            Pattern::Constructor(_, name, sub_pats) => {
                let Some(&(gc_type_idx, tag)) = self.adt_variant_indices.get(name) else {
                    return Err(LowerError::Unsupported {
                        msg: format!("WasmGC ADT variant を解決できません: {name}"),
                        span: Some(arm.span),
                    });
                };
                ctx.emit(Instruction::LocalGet(*value_local));
                ctx.emit(Instruction::StructGet(gc_type_idx, 0));
                ctx.emit(Instruction::I64Const(tag as i64));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));

                let field_types = self
                    .adt_variant_field_types
                    .get(name)
                    .cloned()
                    .unwrap_or_default();
                let field_type_names = self
                    .adt_variant_field_type_names
                    .get(name)
                    .cloned()
                    .unwrap_or_default();
                let field_offsets = self
                    .adt_variant_field_offsets
                    .get(name)
                    .cloned()
                    .unwrap_or_default();
                let mut nested_checks = Vec::with_capacity(sub_pats.len() + rest.len());
                for (field_idx, nested_pattern) in sub_pats.iter().enumerate() {
                    ctx.emit(Instruction::LocalGet(*value_local));
                    let slot_idx = field_offsets
                        .get(field_idx)
                        .copied()
                        .unwrap_or(field_idx as u32);
                    ctx.emit(Instruction::StructGet(gc_type_idx, slot_idx + 1));
                    let field_type = field_types.get(field_idx).copied().unwrap_or(IrType::I64);
                    let field_local = ctx
                        .alloc_local_typed(format!("_wasmgc_nested_field_{field_idx}"), field_type);
                    ctx.emit(Instruction::LocalSet(field_local));
                    nested_checks.push((
                        field_local,
                        nested_pattern.clone(),
                        field_type_names.get(field_idx).cloned().flatten(),
                    ));
                }
                nested_checks.extend(rest);
                self.lower_wasmgc_pattern_sequence(ctx, nested_checks, arms, idx, scrut_local)?;

                ctx.emit(Instruction::Else);
                if idx == arms.len() - 1 {
                    ctx.emit(Instruction::Unreachable);
                } else {
                    self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                }
                ctx.emit(Instruction::End);
                Ok(())
            }
            Pattern::Lit(_, literal) => {
                let Some(expected) = (match literal {
                    Literal::Int(value) => Some(*value),
                    Literal::Bool(value) => Some(i64::from(*value)),
                    Literal::Unit => Some(0),
                    Literal::Float(_) | Literal::String(_) => None,
                }) else {
                    return Err(LowerError::Unsupported {
                        msg: "WasmGC ADT の nested/literal pattern".to_string(),
                        span: Some(arm.span),
                    });
                };
                ctx.emit(Instruction::LocalGet(*value_local));
                ctx.emit(Instruction::I64Const(expected));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_wasmgc_pattern_sequence(ctx, rest, arms, idx, scrut_local)?;
                ctx.emit(Instruction::Else);
                if idx == arms.len() - 1 {
                    ctx.emit(Instruction::Unreachable);
                } else {
                    self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                }
                ctx.emit(Instruction::End);
                Ok(())
            }
            Pattern::RecordPat(_, type_name, field_pats) => {
                let mut nested_checks = self.collect_wasmgc_record_pattern_checks(
                    ctx,
                    *value_local,
                    type_name,
                    field_pats,
                    arm.span,
                )?;
                nested_checks.extend(rest);
                self.lower_wasmgc_pattern_sequence(ctx, nested_checks, arms, idx, scrut_local)
            }
        }
    }
}
