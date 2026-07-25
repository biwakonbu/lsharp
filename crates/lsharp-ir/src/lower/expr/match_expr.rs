use lsharp_syntax::ast::{Expr, MatchArm};

use crate::{Instruction, IrType};

use super::{FuncCtx, Lower, LowerError};

impl Lower {
    pub(super) fn lower_match_expr(
        &mut self,
        ctx: &mut FuncCtx,
        scrutinee: &Expr,
        arms: &[MatchArm],
    ) -> Result<(), LowerError> {
        // scrutinee を評価してローカルに保存
        self.lower_expr(ctx, scrutinee)?;
        let scrut_type_name = self.infer_expr_type_name_with_ctx(ctx, scrutinee);
        let scrut_ir_type = scrut_type_name
            .as_deref()
            .map(|name| self.ir_type_for_type_name(name))
            .unwrap_or(IrType::I64);
        let scrut_local = ctx.alloc_local_typed("_match".to_string(), scrut_ir_type);
        if let Some(type_name) = scrut_type_name {
            ctx.local_type_names.insert("_match".to_string(), type_name);
        }
        ctx.emit(Instruction::LocalSet(scrut_local));

        // ネストした if-else チェインで変換
        self.lower_match_arms(ctx, scrut_local, arms, 0)?;
        Ok(())
    }
}
