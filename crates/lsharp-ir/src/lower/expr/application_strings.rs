use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::lower::{FuncCtx, Lower, LowerBackend, LowerError};
use crate::{Instruction, IrType};

impl Lower {
    pub(super) fn lower_app_string_scalar(
        &mut self,
        ctx: &mut FuncCtx,
        expr_span: Span,
        func: &Expr,
        args: &[Expr],
    ) -> Result<bool, LowerError> {
        let Expr::Var(_, name) = func else {
            return Ok(false);
        };

        match name.as_str() {
            // string-length: ヒープ上 String オブジェクトの len フィールドを取得
            // String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
            "string-length" => {
                if let Some(arg) = args.first() {
                    self.lower_expr(ctx, arg)?;
                }
                if self.backend == LowerBackend::WasmGc {
                    let type_index =
                        self.string_array_type_index
                            .ok_or_else(|| LowerError::Unsupported {
                                msg: "WasmGC String の GC array type が登録されていません"
                                    .to_string(),
                                span: Some(expr_span),
                            })?;
                    ctx.emit(Instruction::ArrayLen(type_index));
                    ctx.emit(Instruction::I64ExtendI32U);
                    return Ok(true);
                }
                // s はスタックトップ (i64) = ヒープオブジェクトのアドレス
                // len = i32.load(s + 4)
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Load { offset: 4 });
                ctx.emit(Instruction::I64ExtendI32U);
                Ok(true)
            }
            // string-eq: 2 つの文字列を比較
            "string-eq" => {
                if args.len() >= 2 {
                    if self.backend == LowerBackend::WasmGc {
                        let type_index = self.string_array_type_index.ok_or_else(|| {
                            LowerError::Unsupported {
                                msg: "WasmGC String の GC array type が登録されていません"
                                    .to_string(),
                                span: Some(expr_span),
                            }
                        })?;
                        let lhs_local = ctx
                            .alloc_local_typed("_str_eq_lhs".to_string(), IrType::Ref(type_index));
                        let rhs_local = ctx
                            .alloc_local_typed("_str_eq_rhs".to_string(), IrType::Ref(type_index));
                        let lhs_len_local =
                            ctx.alloc_local_typed("_str_eq_lhs_len".to_string(), IrType::I32);
                        let rhs_len_local =
                            ctx.alloc_local_typed("_str_eq_rhs_len".to_string(), IrType::I32);
                        let index_local =
                            ctx.alloc_local_typed("_str_eq_index".to_string(), IrType::I32);
                        let result_local =
                            ctx.alloc_local_typed("_str_eq_result".to_string(), IrType::I64);

                        self.lower_expr(ctx, &args[0])?;
                        ctx.emit(Instruction::LocalSet(lhs_local));
                        self.lower_expr(ctx, &args[1])?;
                        ctx.emit(Instruction::LocalSet(rhs_local));

                        ctx.emit(Instruction::LocalGet(lhs_local));
                        ctx.emit(Instruction::ArrayLen(type_index));
                        ctx.emit(Instruction::LocalSet(lhs_len_local));
                        ctx.emit(Instruction::LocalGet(rhs_local));
                        ctx.emit(Instruction::ArrayLen(type_index));
                        ctx.emit(Instruction::LocalSet(rhs_len_local));
                        ctx.emit(Instruction::I64Const(1));
                        ctx.emit(Instruction::LocalSet(result_local));

                        // 長さが異なる場合は要素を読まずに false とする。
                        ctx.emit(Instruction::LocalGet(lhs_len_local));
                        ctx.emit(Instruction::I64ExtendI32U);
                        ctx.emit(Instruction::LocalGet(rhs_len_local));
                        ctx.emit(Instruction::I64ExtendI32U);
                        ctx.emit(Instruction::I64Ne);
                        ctx.emit(Instruction::IfEmpty);
                        ctx.emit(Instruction::I64Const(0));
                        ctx.emit(Instruction::LocalSet(result_local));
                        ctx.emit(Instruction::Else);

                        ctx.emit(Instruction::I32Const(0));
                        ctx.emit(Instruction::LocalSet(index_local));
                        ctx.emit(Instruction::BlockEmpty);
                        ctx.emit(Instruction::LoopEmpty);
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::LocalGet(lhs_len_local));
                        ctx.emit(Instruction::I32GeU);
                        ctx.emit(Instruction::BrIf(1));

                        ctx.emit(Instruction::LocalGet(lhs_local));
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::ArrayGet(type_index));
                        ctx.emit(Instruction::I64ExtendI32U);
                        ctx.emit(Instruction::LocalGet(rhs_local));
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::ArrayGet(type_index));
                        ctx.emit(Instruction::I64ExtendI32U);
                        ctx.emit(Instruction::I64Ne);
                        ctx.emit(Instruction::IfEmpty);
                        ctx.emit(Instruction::I64Const(0));
                        ctx.emit(Instruction::LocalSet(result_local));
                        ctx.emit(Instruction::Br(2));
                        ctx.emit(Instruction::End);

                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::I32Const(1));
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::LocalSet(index_local));
                        ctx.emit(Instruction::Br(0));
                        ctx.emit(Instruction::End);
                        ctx.emit(Instruction::End);
                        ctx.emit(Instruction::End);
                        ctx.emit(Instruction::LocalGet(result_local));
                        return Ok(true);
                    }
                    self.lower_expr(ctx, &args[0])?;
                    self.lower_expr(ctx, &args[1])?;
                }
                let idx = *self.func_indices.get("__string_eq").ok_or_else(|| {
                    LowerError::UndefinedFunction {
                        name: "__string_eq".to_string(),
                        span: Some(expr_span),
                    }
                })?;
                ctx.emit(Instruction::Call(idx));
                Ok(true)
            }
            // int-to-string: 整数を文字列に変換
            "int-to-string" => {
                if let Some(arg) = args.first() {
                    self.lower_expr(ctx, arg)?;
                }
                let idx = *self.func_indices.get("__int_to_string").ok_or_else(|| {
                    LowerError::UndefinedFunction {
                        name: "__int_to_string".to_string(),
                        span: Some(expr_span),
                    }
                })?;
                ctx.emit(Instruction::Call(idx));
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
