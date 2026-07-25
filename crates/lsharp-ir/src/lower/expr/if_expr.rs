use lsharp_syntax::ast::Expr;

use crate::{Instruction, IrType};

use super::{FuncCtx, Lower, LowerError};

impl Lower {
    pub(super) fn lower_if(
        &mut self,
        ctx: &mut FuncCtx,
        cond: &Expr,
        then_branch: &Expr,
        else_branch: &Expr,
    ) -> Result<(), LowerError> {
        self.lower_expr(ctx, cond)?;
        // Bool (i64) -> i32 に変換
        ctx.emit(Instruction::I32WrapI64);
        // if-then-else
        ctx.emit(Instruction::If(IrType::I64));
        self.lower_expr(ctx, then_branch)?;
        ctx.emit(Instruction::Else);
        self.lower_expr(ctx, else_branch)?;
        ctx.emit(Instruction::End);
        Ok(())
    }
}
