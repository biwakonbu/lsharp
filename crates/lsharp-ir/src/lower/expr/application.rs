use lsharp_syntax::ast::Expr;
use lsharp_syntax::span::Span;

use crate::lower::{FuncCtx, Lower, LowerBackend, LowerError};
use crate::{Instruction, IrType};

impl Lower {
    pub(super) fn lower_app(
        &mut self,
        ctx: &mut FuncCtx,
        expr_span: Span,
        func: &Expr,
        args: &[Expr],
    ) -> Result<(), LowerError> {
        if self.backend == LowerBackend::WasmGc
            && let Expr::Lambda(lambda_span, params, body) = func
        {
            let free_var_list = self.wasmgc_lambda_free_vars(params, body);
            if !free_var_list.is_empty() {
                let info = self.lower_wasmgc_captured_lambda_value(
                    ctx,
                    *lambda_span,
                    params,
                    body,
                    &free_var_list,
                )?;
                let env_local = ctx.alloc_local_typed(
                    "_wasmgc_closure_env".to_string(),
                    IrType::Ref(info.env_type_index),
                );
                ctx.emit(Instruction::LocalSet(env_local));
                for arg in args {
                    self.lower_expr(ctx, arg)?;
                }
                ctx.emit(Instruction::LocalGet(env_local));
                ctx.emit(Instruction::LocalGet(env_local));
                ctx.emit(Instruction::StructGet(info.env_type_index, 0));
                ctx.emit(Instruction::CallRef(info.call_ref_type_index));
                return Ok(());
            }
            // WasmGC の non-capturing lambda は、引数を先に積んでから
            // `ref.func` を積み、lambda の user function type を指定した
            // typed `call_ref` へ接続する。captured lambda は lower_expr 側の
            // 明示拒否境界を通るため、linear-memory closure へ戻らない。
            for arg in args {
                self.lower_expr(ctx, arg)?;
            }
            self.lower_expr(ctx, func)?;
            let function_index = match ctx.instructions.last() {
                Some(Instruction::RefFunc(index)) => *index,
                _ => {
                    return Err(LowerError::Unsupported {
                        msg: "WasmGC lambda call の ref.func が生成されませんでした".to_string(),
                        span: Some(expr_span),
                    });
                }
            };
            // WasmGC emitter の type section は GC type → import function type →
            // user function type の順序で構築される。lowerer が生成する module
            // は import を持たないため、ここでは GC type 数を user function index
            // に加えれば lambda の function type index になる。
            ctx.emit(Instruction::CallRef(
                self.gc_types.len() as u32 + function_index,
            ));
            return Ok(());
        }
        let func_expr = func;
        if self.lower_app_scalar(ctx, expr_span, func_expr, args)?
            || self.lower_app_ref_vector(ctx, expr_span, func_expr, args)?
            || self.lower_app_map(ctx, expr_span, func_expr, args)?
            || self.lower_app_calls(ctx, expr_span, func_expr, args)?
        {
            return Ok(());
        }
        Err(LowerError::Unsupported {
            msg: "間接的な関数呼び出し".to_string(),
            span: Some(expr_span),
        })
    }
}
