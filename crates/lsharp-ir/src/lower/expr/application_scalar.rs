use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::Instruction;
use crate::lower::{FuncCtx, Lower, LowerError, is_builtin_binop};

impl Lower {
    pub(super) fn lower_app_scalar(
        &mut self,
        ctx: &mut FuncCtx,
        expr_span: Span,
        func: &Expr,
        args: &[Expr],
    ) -> Result<bool, LowerError> {
        if self.lower_app_string_scalar(ctx, expr_span, func, args)? {
            return Ok(true);
        }
        if self.lower_app_string_heap(ctx, expr_span, func, args)? {
            return Ok(true);
        }

        match func {
            Expr::Var(_, op) if (op == "and" || op == "or") && args.len() == 2 => {
                // 左オペランド: i64 -> i32
                self.lower_expr(ctx, &args[0])?;
                ctx.emit(Instruction::I32WrapI64);
                // 右オペランド: i64 -> i32
                self.lower_expr(ctx, &args[1])?;
                ctx.emit(Instruction::I32WrapI64);
                // i32 レベルで and/or
                if op == "and" {
                    ctx.emit(Instruction::I32And);
                } else {
                    ctx.emit(Instruction::I32Or);
                }
                // 結果を i64 に拡張
                ctx.emit(Instruction::I64ExtendI32S);
                Ok(true)
            }
            // 組み込み二項演算子
            Expr::Var(_, op) if is_builtin_binop(op) && args.len() == 2 => {
                self.lower_expr(ctx, &args[0])?;
                self.lower_expr(ctx, &args[1])?;
                self.emit_binop(ctx, op, expr_span)?;
                Ok(true)
            }
            // not 演算子
            Expr::Var(_, op) if op == "not" && args.len() == 1 => {
                self.lower_expr(ctx, &args[0])?;
                ctx.emit(Instruction::I64Const(0));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::I64ExtendI32S);
                Ok(true)
            }
            // print 関数 (多相: 引数型に応じて print-int / print-string を呼び分け)
            Expr::Var(_, name) if name == "print" => {
                if let Some(arg) = args.first() {
                    // 引数の型を推定して適切な print 関数を選択
                    let is_string = self
                        .infer_expr_type_name(arg)
                        .map(|t| t == "String")
                        .unwrap_or(false);
                    self.lower_expr(ctx, arg)?;
                    if is_string {
                        // 文字列の場合: print-string (改行なし) + 改行出力
                        let idx = *self.func_indices.get("print-string").ok_or_else(|| {
                            LowerError::UndefinedFunction {
                                name: "print-string".to_string(),
                                span: Some(expr_span),
                            }
                        })?;
                        ctx.emit(Instruction::Call(idx));
                    } else {
                        // 整数の場合: print (改行付き)
                        let idx = *self.func_indices.get("print").ok_or_else(|| {
                            LowerError::UndefinedFunction {
                                name: "print".to_string(),
                                span: Some(expr_span),
                            }
                        })?;
                        ctx.emit(Instruction::Call(idx));
                    }
                }
                // print は Unit を返す
                ctx.emit(Instruction::I64Const(0));
                Ok(true)
            }
            // print-string: 文字列を出力 (改行なし)
            Expr::Var(_, name) if name == "print-string" => {
                if let Some(arg) = args.first() {
                    self.lower_expr(ctx, arg)?;
                }
                let idx = *self.func_indices.get("print-string").ok_or_else(|| {
                    LowerError::UndefinedFunction {
                        name: "print-string".to_string(),
                        span: Some(expr_span),
                    }
                })?;
                ctx.emit(Instruction::Call(idx));
                // print-string は Unit を返す
                ctx.emit(Instruction::I64Const(0));
                Ok(true)
            }
            // proc-exit: プロセス終了 (Int -> Unit)
            Expr::Var(_, name) if name == "proc-exit" => {
                if let Some(arg) = args.first() {
                    self.lower_expr(ctx, arg)?;
                }
                // i64 -> i32 に変換して proc_exit を呼ぶ
                ctx.emit(Instruction::I32WrapI64);
                let idx = *self.func_indices.get("proc-exit").ok_or_else(|| {
                    LowerError::UndefinedFunction {
                        name: "proc-exit".to_string(),
                        span: Some(expr_span),
                    }
                })?;
                ctx.emit(Instruction::Call(idx));
                // proc-exit は Unit を返す（実際にはここに到達しないが型整合のため）
                ctx.emit(Instruction::I64Const(0));
                Ok(true)
            }
            // __alloc 関数 (Bump Allocator)
            Expr::Var(_, name) if name == "__alloc" => {
                if let Some(arg) = args.first() {
                    self.lower_expr(ctx, arg)?;
                }
                let idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                    LowerError::UndefinedFunction {
                        name: "__alloc".to_string(),
                        span: Some(expr_span),
                    }
                })?;
                ctx.emit(Instruction::Call(idx));
                Ok(true)
            }
            // ref-new: ヒープに Ref Cell を確保して値を格納
            // レイアウト: [tag=7: i32, _pad: i32, value: i64]
            // 合計 16 バイト
            _ => Ok(false),
        }
    }
}
