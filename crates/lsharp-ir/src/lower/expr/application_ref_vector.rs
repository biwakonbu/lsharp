use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::lower::{FuncCtx, Lower, LowerError};

impl Lower {
    pub(super) fn lower_app_ref_vector(
        &mut self,
        ctx: &mut FuncCtx,
        expr_span: Span,
        func: &Expr,
        args: &[Expr],
    ) -> Result<bool, LowerError> {
        if self.lower_app_ref(ctx, expr_span, func, args)? {
            return Ok(true);
        }
        self.lower_app_vector(ctx, expr_span, func, args)
    }
}
