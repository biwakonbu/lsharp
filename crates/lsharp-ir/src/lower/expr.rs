//! 式の lowering (lower_expr 関連)

use lsharp_syntax::ast::*;

use crate::{Instruction, IrType};

use super::{FuncCtx, Lower, LowerBackend, LowerError};

mod application;
mod application_calls;
mod application_map;
mod application_ref_vector;
mod application_scalar;
#[cfg(test)]
mod application_tests;
mod computation;
#[cfg(test)]
mod computation_tests;
mod do_expr;
#[cfg(test)]
mod do_expr_tests;
mod helpers;
#[cfg(test)]
mod helpers_tests;
mod lambda;
#[cfg(test)]
mod lambda_tests;
mod let_expr;
#[cfg(test)]
mod let_expr_tests;
mod match_expr;
#[cfg(test)]
mod match_expr_tests;
mod record;
#[cfg(test)]
mod record_tests;
mod wasmgc_lambda;
#[cfg(test)]
mod wasmgc_lambda_tests;

#[derive(Debug, Clone, Copy)]
struct WasmGcCapturedLambdaInfo {
    env_type_index: u32,
    call_ref_type_index: u32,
}

impl Lower {
    /// 式を IR 命令に変換（スタックマシン方式）
    pub(crate) fn lower_expr(&mut self, ctx: &mut FuncCtx, expr: &Expr) -> Result<(), LowerError> {
        match expr {
            Expr::Lit(expr_span, lit) => match lit {
                Literal::Int(n) => ctx.emit(Instruction::I64Const(*n)),
                Literal::Float(n) => ctx.emit(Instruction::F64Const(*n)),
                Literal::Bool(b) => ctx.emit(Instruction::I64Const(if *b { 1 } else { 0 })),
                Literal::String(s) => {
                    if self.backend == LowerBackend::WasmGc {
                        let type_index = self.string_array_type_index.ok_or_else(|| {
                            LowerError::Unsupported {
                                msg: "WasmGC String の GC array type が登録されていません"
                                    .to_string(),
                                span: Some(*expr_span),
                            }
                        })?;
                        for byte in s.as_bytes() {
                            ctx.emit(Instruction::I32Const(i32::from(*byte)));
                        }
                        ctx.emit(Instruction::ArrayNewFixed(
                            type_index,
                            s.len().try_into().map_err(|_| LowerError::Unsupported {
                                msg:
                                    "WasmGC String literal が array.new_fixed の長さを超えています"
                                        .to_string(),
                                span: Some(*expr_span),
                            })?,
                        ));
                        return Ok(());
                    }

                    // 文字列リテラル: データセクションにバイト列を格納し、
                    // ランタイムでヒープ上に String オブジェクト [tag=1, len, bytes] を確保
                    let bytes = s.as_bytes().to_vec();
                    let len = bytes.len() as u32;
                    let data_offset = self.string_offset;
                    let label = format!("$str{}", self.string_data.len());
                    self.string_data.push((label, bytes));
                    self.string_offset += len;

                    let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                        LowerError::UndefinedFunction {
                            name: "__alloc".to_string(),
                            span: Some(*expr_span),
                        }
                    })?;

                    // __alloc(8 + len) でヒープ領域を確保
                    ctx.emit(Instruction::I64Const((8 + len) as i64));
                    ctx.emit(Instruction::Call(alloc_idx));
                    // 戻り値 = ヒープオブジェクトのアドレス (i64)
                    let obj_local = ctx.alloc_local("_str_obj".to_string());
                    ctx.emit(Instruction::LocalSet(obj_local));

                    // tag = String を書き込み (obj + 0)
                    ctx.emit(Instruction::LocalGet(obj_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Const(super::HEAP_TAG_STRING));
                    ctx.emit(Instruction::I32Store { offset: 0 });

                    // len を書き込み (obj + 4)
                    ctx.emit(Instruction::LocalGet(obj_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Const(len as i32));
                    ctx.emit(Instruction::I32Store { offset: 4 });

                    if len > 0 {
                        // memory.copy(obj + 8, data_offset, len)
                        // dst: obj + 8
                        ctx.emit(Instruction::LocalGet(obj_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(8));
                        ctx.emit(Instruction::I32Add);
                        // src: data_offset (データセクション上のアドレス)
                        ctx.emit(Instruction::I32Const(data_offset as i32));
                        // len
                        ctx.emit(Instruction::I32Const(len as i32));
                        ctx.emit(Instruction::MemoryCopy);
                    }

                    // タグ付き String handle をスタックに積む
                    ctx.emit(Instruction::LocalGet(obj_local));
                    ctx.emit(Instruction::I64Const(1i64 << 63));
                    ctx.emit(Instruction::I64Add);
                }
                Literal::Unit => ctx.emit(Instruction::I64Const(0)),
            },

            Expr::Var(expr_span, name) => {
                if let Some(&idx) = ctx.locals_map.get(name) {
                    ctx.emit(Instruction::LocalGet(idx));
                } else if let Some(&func_idx) = self.func_indices.get(name) {
                    // 引数なし ADT コンストラクタ（または引数なし関数）を呼び出し
                    ctx.emit(Instruction::Call(func_idx));
                } else if let Some(&func_idx) = self.lifted_func_indices.get(name) {
                    // Lambda Lifting で生成された関数の呼び出し
                    ctx.emit(Instruction::Call(func_idx));
                } else {
                    return Err(LowerError::UndefinedFunction {
                        name: name.clone(),
                        span: Some(*expr_span),
                    });
                }
            }

            Expr::If(_, cond, then, else_) => {
                // 条件式
                self.lower_expr(ctx, cond)?;
                // Bool (i64) -> i32 に変換
                ctx.emit(Instruction::I32WrapI64);
                // if-then-else
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_expr(ctx, then)?;
                ctx.emit(Instruction::Else);
                self.lower_expr(ctx, else_)?;
                ctx.emit(Instruction::End);
            }

            Expr::Let(_, bindings, body) => {
                self.lower_let(ctx, bindings, body)?;
            }
            Expr::App(expr_span, func, args) => {
                self.lower_app(ctx, *expr_span, func, args)?;
            }
            Expr::Match(_, scrutinee, arms) => {
                self.lower_match_expr(ctx, scrutinee, arms)?;
            }

            Expr::Do(_, exprs) => {
                self.lower_do(ctx, exprs)?;
            }

            Expr::Lambda(_, params, body) => {
                self.lower_lambda(ctx, expr.span(), params, body)?;
            }
            Expr::Ann(_, expr, _) => {
                // 型注釈は無視して中身を変換
                self.lower_expr(ctx, expr)?;
            }

            Expr::RecordLit(_, type_name, fields) => {
                self.lower_record_lit(ctx, type_name, fields)?;
            }
            Expr::FieldAccess(expr_span, expr, field_name) => {
                self.lower_field_access(ctx, *expr_span, expr, field_name)?;
            }
            Expr::RecordUpdate(_, base, update_fields) => {
                self.lower_record_update(ctx, base, update_fields)?;
            }
            Expr::Computation(span, builder_name, steps) => {
                self.lower_computation(ctx, *span, builder_name, steps)?;
            }
            // P10-1: Quote/Unquote/UnquoteSplice はマクロ展開後には残らない
            Expr::Quote(expr_span, _)
            | Expr::Unquote(expr_span, _)
            | Expr::UnquoteSplice(expr_span, _) => {
                return Err(LowerError::Unsupported {
                    msg: "quote/unquote はマクロ展開後に使用できません".to_string(),
                    span: Some(*expr_span),
                });
            }
        }

        Ok(())
    }
}
