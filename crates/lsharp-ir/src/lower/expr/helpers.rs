use lsharp_syntax::ast::{Expr, Literal};
use lsharp_syntax::span::Span;

use crate::Instruction;
use crate::lower::{FuncCtx, Lower, LowerBackend, LowerError, is_heap_like_type_name};

impl Lower {
    /// 二項演算子の IR 命令を出力
    pub(crate) fn emit_binop(
        &mut self,
        ctx: &mut FuncCtx,
        op: &str,
        span: Span,
    ) -> Result<(), LowerError> {
        match op {
            "+" => ctx.emit(Instruction::I64Add),
            "-" => ctx.emit(Instruction::I64Sub),
            "*" => ctx.emit(Instruction::I64Mul),
            "/" => ctx.emit(Instruction::I64Div),
            "%" => ctx.emit(Instruction::I64Rem),
            "+." => ctx.emit(Instruction::F64Add),
            "-." => ctx.emit(Instruction::F64Sub),
            "*." => ctx.emit(Instruction::F64Mul),
            "/." => ctx.emit(Instruction::F64Div),
            "==" | "=" => {
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "!=" => {
                ctx.emit(Instruction::I64Ne);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "<" => {
                ctx.emit(Instruction::I64LtS);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            ">" => {
                ctx.emit(Instruction::I64GtS);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "<=" => {
                ctx.emit(Instruction::I64LeS);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            ">=" => {
                ctx.emit(Instruction::I64GeS);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "and" => {
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32And);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "or" => {
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Or);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            _ => {
                return Err(LowerError::Unsupported {
                    msg: format!("未知の二項演算子: {}", op),
                    span: Some(span),
                });
            }
        }
        Ok(())
    }

    /// 文字列キーの場合に FNV-1a ハッシュ呼び出しを挿入する
    fn emit_string_key_hash(&self, ctx: &mut FuncCtx, key_expr: &Expr) -> Result<(), LowerError> {
        let is_string_key = self
            .infer_expr_type_name_with_ctx(ctx, key_expr)
            .map(|t| t == "String")
            .unwrap_or(false);
        if is_string_key {
            let hash_idx = *self.func_indices.get("__fnv1a_hash").ok_or_else(|| {
                LowerError::UndefinedFunction {
                    name: "__fnv1a_hash".to_string(),
                    span: Some(key_expr.span()),
                }
            })?;
            ctx.emit(Instruction::Call(hash_idx));
        }
        Ok(())
    }

    pub(super) fn lower_map_key_to_local(
        &mut self,
        ctx: &mut FuncCtx,
        key_expr: &Expr,
        value_name: &str,
        slot_name: &str,
        key_local_name: &str,
    ) -> Result<(u32, bool), LowerError> {
        let key_is_rooted = self.should_root_user_call_argument(ctx, key_expr);
        if key_is_rooted {
            let key_value_local =
                self.lower_expr_to_rooted_local(ctx, key_expr, value_name, slot_name)?;
            ctx.emit(Instruction::LocalGet(key_value_local));
            self.emit_string_key_hash(ctx, key_expr)?;
            let key_local = ctx.alloc_local(key_local_name.to_string());
            ctx.emit(Instruction::LocalSet(key_local));
            Ok((key_local, true))
        } else {
            self.lower_expr(ctx, key_expr)?;
            self.emit_string_key_hash(ctx, key_expr)?;
            let key_local = ctx.alloc_local(key_local_name.to_string());
            ctx.emit(Instruction::LocalSet(key_local));
            Ok((key_local, false))
        }
    }

    pub(super) fn emit_root_push_local(
        &self,
        ctx: &mut FuncCtx,
        value_local: u32,
        slot_name: &str,
    ) -> Result<u32, LowerError> {
        let root_push_idx =
            *self
                .func_indices
                .get("root_push")
                .ok_or_else(|| LowerError::UndefinedFunction {
                    name: "root_push".to_string(),
                    span: None,
                })?;
        let slot_local = ctx.alloc_local(slot_name.to_string());
        ctx.emit(Instruction::LocalGet(value_local));
        ctx.emit(Instruction::Call(root_push_idx));
        ctx.emit(Instruction::LocalSet(slot_local));
        Ok(slot_local)
    }

    pub(super) fn lower_expr_to_rooted_local(
        &mut self,
        ctx: &mut FuncCtx,
        expr: &Expr,
        value_name: &str,
        slot_name: &str,
    ) -> Result<u32, LowerError> {
        self.lower_expr(ctx, expr)?;
        let value_local = ctx.alloc_local(value_name.to_string());
        ctx.emit(Instruction::LocalSet(value_local));
        self.emit_root_push_local(ctx, value_local, slot_name)?;
        Ok(value_local)
    }

    pub(super) fn emit_root_pop_drop(&self, ctx: &mut FuncCtx) -> Result<(), LowerError> {
        let root_pop_idx =
            *self
                .func_indices
                .get("root_pop")
                .ok_or_else(|| LowerError::UndefinedFunction {
                    name: "root_pop".to_string(),
                    span: None,
                })?;
        ctx.emit(Instruction::Call(root_pop_idx));
        ctx.emit(Instruction::Drop);
        Ok(())
    }

    pub(super) fn should_root_user_call_argument(&self, ctx: &FuncCtx, expr: &Expr) -> bool {
        if self.backend == LowerBackend::WasmGc {
            return false;
        }

        self.infer_expr_type_name_with_ctx(ctx, expr)
            .map(|type_name| is_heap_like_type_name(&type_name))
            .unwrap_or_else(|| self.should_conservatively_root_unknown_argument(expr))
    }

    fn should_conservatively_root_unknown_argument(&self, expr: &Expr) -> bool {
        !matches!(
            expr,
            Expr::Lit(
                _,
                Literal::Int(_) | Literal::Float(_) | Literal::Bool(_) | Literal::Unit
            )
        )
    }

    pub(super) fn emit_wasmgc_substring_range_guard(
        &self,
        ctx: &mut FuncCtx,
        source_local: u32,
        start_local: u32,
        end_local: u32,
        type_index: u32,
    ) {
        // invalid = start < 0 || end < 0 || start > end || end > source.length
        ctx.emit(Instruction::LocalGet(start_local));
        ctx.emit(Instruction::I64Const(0));
        ctx.emit(Instruction::I64LtS);
        ctx.emit(Instruction::LocalGet(end_local));
        ctx.emit(Instruction::I64Const(0));
        ctx.emit(Instruction::I64LtS);
        ctx.emit(Instruction::I32Or);
        ctx.emit(Instruction::LocalGet(start_local));
        ctx.emit(Instruction::LocalGet(end_local));
        ctx.emit(Instruction::I64GtS);
        ctx.emit(Instruction::I32Or);
        ctx.emit(Instruction::LocalGet(end_local));
        ctx.emit(Instruction::LocalGet(source_local));
        ctx.emit(Instruction::ArrayLen(type_index));
        ctx.emit(Instruction::I64ExtendI32U);
        ctx.emit(Instruction::I64GtS);
        ctx.emit(Instruction::I32Or);
        ctx.emit(Instruction::IfEmpty);
        ctx.emit(Instruction::Unreachable);
        ctx.emit(Instruction::End);
    }
}

pub(super) fn validate_wasmgc_substring_static_range(
    args: &[Expr],
    span: Span,
) -> Result<(), LowerError> {
    let [
        Expr::Lit(_, Literal::String(source)),
        Expr::Lit(_, Literal::Int(start)),
        Expr::Lit(_, Literal::Int(end)),
        ..,
    ] = args
    else {
        return Ok(());
    };

    let length = source.len() as i64;
    if *start < 0 || *end < 0 || *start > *end || *end > length {
        return Err(LowerError::Unsupported {
            msg: format!(
                "WasmGC substring の範囲が不正です: start={start}, end={end}, length={length}"
            ),
            span: Some(span),
        });
    }

    Ok(())
}
