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
mod helpers;
#[cfg(test)]
mod helpers_tests;
mod lambda;
#[cfg(test)]
mod lambda_tests;
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
                let mut scoped_bindings = Vec::new();
                let result = (|| -> Result<(), LowerError> {
                    for (pat, val) in bindings {
                        let inferred_type_name = self.infer_expr_type_name_with_ctx(ctx, val);
                        let is_captured_lambda = if self.backend == super::LowerBackend::WasmGc
                            && let Expr::Lambda(lambda_span, params, body) = val
                        {
                            let free_var_list = self.wasmgc_lambda_free_vars(params, body);
                            if free_var_list.is_empty() {
                                false
                            } else {
                                self.lower_wasmgc_captured_lambda_value(
                                    ctx,
                                    *lambda_span,
                                    params,
                                    body,
                                    &free_var_list,
                                )?;
                                true
                            }
                        } else {
                            false
                        };
                        if !is_captured_lambda {
                            self.lower_expr(ctx, val)?;
                        }
                        let lambda_func_index = if self.backend == super::LowerBackend::WasmGc
                            && matches!(val, Expr::Lambda(_, _, _))
                        {
                            match ctx.instructions.last() {
                                Some(Instruction::RefFunc(function_index)) => Some(*function_index),
                                _ => None,
                            }
                        } else {
                            None
                        };
                        let lambda_func_type_index =
                            lambda_func_index.map(|index| self.gc_types.len() as u32 + index);
                        let lambda_env_type_index = if self.backend == super::LowerBackend::WasmGc
                            && matches!(val, Expr::Lambda(_, _, _))
                        {
                            match ctx.instructions.last() {
                                Some(Instruction::StructNew(type_index)) => Some(*type_index),
                                _ => None,
                            }
                        } else {
                            None
                        };
                        match pat {
                            Pattern::Var(_, name) => {
                                let previous_local = ctx.locals_map.get(name).copied();
                                let previous_type = ctx.local_type_names.get(name).cloned();
                                let binding_ir_type = lambda_env_type_index
                                    .map(IrType::Ref)
                                    .or_else(|| lambda_func_type_index.map(IrType::TypedFuncRef))
                                    .or_else(|| {
                                        inferred_type_name
                                            .as_deref()
                                            .map(|type_name| self.ir_type_for_type_name(type_name))
                                            .filter(|ty| {
                                                self.backend == super::LowerBackend::WasmGc
                                                    && matches!(ty, IrType::Ref(_))
                                            })
                                    })
                                    .unwrap_or(IrType::I64);
                                let idx =
                                    ctx.alloc_scoped_local_typed(name.clone(), binding_ir_type);
                                if let Some(type_name) = inferred_type_name {
                                    ctx.local_type_names.insert(name.clone(), type_name);
                                } else {
                                    ctx.local_type_names.remove(name);
                                }
                                scoped_bindings.push((name.clone(), previous_local, previous_type));
                                ctx.emit(Instruction::LocalSet(idx));
                            }
                            Pattern::Wildcard(_) => {
                                ctx.emit(Instruction::Drop);
                            }
                            _ => {
                                // MVP: 複雑なパターンは未サポート
                                let idx = ctx.alloc_local("_pat".to_string());
                                ctx.emit(Instruction::LocalSet(idx));
                            }
                        }
                    }
                    self.lower_expr(ctx, body)
                })();
                for (name, previous_local, previous_type) in scoped_bindings.into_iter().rev() {
                    ctx.restore_local_binding(name, previous_local, previous_type);
                }
                result?;
            }
            Expr::App(expr_span, func, args) => {
                self.lower_app(ctx, *expr_span, func, args)?;
            }
            Expr::Match(_, scrutinee, arms) => {
                self.lower_match_expr(ctx, scrutinee, arms)?;
            }

            Expr::Do(_, exprs) => {
                for (i, expr) in exprs.iter().enumerate() {
                    self.lower_expr(ctx, expr)?;
                    // 最後の式以外は結果を捨てる
                    if i < exprs.len() - 1 {
                        ctx.emit(Instruction::Drop);
                    }
                }
                if exprs.is_empty() {
                    ctx.emit(Instruction::I64Const(0)); // unit
                }
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
                if self.backend == LowerBackend::WasmGc
                    && steps.iter().any(|step| {
                        matches!(
                            step,
                            ComputationStep::LetBang(..) | ComputationStep::DoBang(..)
                        )
                    })
                {
                    return Err(LowerError::Unsupported {
                        msg: "WasmGC backend の computation let!/do! は GC closure を使う bind が未対応です"
                            .to_string(),
                        span: Some(*span),
                    });
                }

                // Computation Expression: bind/return 関数呼び出しに脱糖
                let builder_info = self.computation_builders.get(builder_name).cloned();

                for (i, step) in steps.iter().enumerate() {
                    match step {
                        ComputationStep::LetBang(_, pat, expr) => {
                            // let! x = expr -> bind(expr, fn [x] rest)
                            // MVP: bind 関数を呼び出す（簡易版: 式を評価してローカルに格納）
                            self.lower_expr(ctx, expr)?;
                            if let Some((ref bind_fn, _)) = builder_info
                                && let Some(&idx) = self.func_indices.get(bind_fn.as_str())
                            {
                                // bind 関数の第1引数（モナド値）は既にスタック上
                                // 残りのステップは後続で評価される
                                // MVP: 式の結果をそのまま変数に束縛
                                let _ = idx; // 将来的に bind 呼び出しに使用
                            }
                            // パターン変数をローカルに格納
                            if let Pattern::Var(_, var_name) = pat {
                                let var_local = ctx.alloc_local(var_name.clone());
                                ctx.emit(Instruction::LocalSet(var_local));
                            }
                        }
                        ComputationStep::DoBang(_, expr) => {
                            // do! expr -> bind(expr, fn [_] rest)
                            self.lower_expr(ctx, expr)?;
                            // 結果を捨てる（最後のステップでなければ）
                            if i < steps.len() - 1 {
                                ctx.emit(Instruction::Drop);
                            }
                        }
                        ComputationStep::Return(_, expr) => {
                            // return expr -> return_fn(expr)
                            self.lower_expr(ctx, expr)?;
                            if let Some((_, ref return_fn)) = builder_info
                                && let Some(&idx) = self.func_indices.get(return_fn.as_str())
                            {
                                ctx.emit(Instruction::Call(idx));
                            }
                        }
                        ComputationStep::Expr(expr) => {
                            self.lower_expr(ctx, expr)?;
                            // 中間式の結果を捨てる（最後のステップでなければ）
                            if i < steps.len() - 1 {
                                ctx.emit(Instruction::Drop);
                            }
                        }
                    }
                }
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
