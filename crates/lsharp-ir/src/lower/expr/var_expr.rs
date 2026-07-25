use lsharp_syntax::span::Span;

use crate::Instruction;

use super::{FuncCtx, Lower, LowerError};

impl Lower {
    pub(super) fn lower_var(
        &mut self,
        ctx: &mut FuncCtx,
        expr_span: Span,
        name: &str,
    ) -> Result<(), LowerError> {
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
                name: name.to_string(),
                span: Some(expr_span),
            });
        }
        Ok(())
    }
}
