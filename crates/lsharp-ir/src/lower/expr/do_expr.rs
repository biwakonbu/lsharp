use lsharp_syntax::ast::Expr;

use crate::Instruction;

use super::{FuncCtx, Lower, LowerError};

impl Lower {
    pub(super) fn lower_do(&mut self, ctx: &mut FuncCtx, exprs: &[Expr]) -> Result<(), LowerError> {
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
        Ok(())
    }
}
