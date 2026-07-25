use lsharp_syntax::ast::{ComputationStep, Pattern};
use lsharp_syntax::span::Span;

use crate::Instruction;

use super::{FuncCtx, Lower, LowerBackend, LowerError};

impl Lower {
    pub(super) fn lower_computation(
        &mut self,
        ctx: &mut FuncCtx,
        span: Span,
        builder_name: &str,
        steps: &[ComputationStep],
    ) -> Result<(), LowerError> {
        if self.backend == LowerBackend::WasmGc
            && steps.iter().any(|step| {
                matches!(
                    step,
                    ComputationStep::LetBang(..) | ComputationStep::DoBang(..)
                )
            })
        {
            return Err(LowerError::Unsupported {
                msg:
                    "WasmGC backend の computation let!/do! は GC closure を使う bind が未対応です"
                        .to_string(),
                span: Some(span),
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
        Ok(())
    }
}
