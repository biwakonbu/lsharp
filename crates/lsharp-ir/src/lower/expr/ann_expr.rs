use lsharp_syntax::ast::Expr;

use super::{FuncCtx, Lower, LowerError};

impl Lower {
    pub(super) fn lower_ann(&mut self, ctx: &mut FuncCtx, expr: &Expr) -> Result<(), LowerError> {
        // 型注釈は無視して中身を変換
        self.lower_expr(ctx, expr)
    }
}
