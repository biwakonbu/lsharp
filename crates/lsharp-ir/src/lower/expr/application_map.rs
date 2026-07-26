use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::lower::{FuncCtx, Lower, LowerError};

impl Lower {
    pub(super) fn lower_app_map(
        &mut self,
        ctx: &mut FuncCtx,
        expr_span: Span,
        func: &Expr,
        args: &[Expr],
    ) -> Result<bool, LowerError> {
        if self.lower_app_map_lookup(ctx, expr_span, func, args)? {
            return Ok(true);
        }
        if self.lower_app_map_mutation(ctx, expr_span, func, args)? {
            return Ok(true);
        }
        if self.lower_app_map_allocation(ctx, expr_span, func, args)? {
            return Ok(true);
        }

        Ok(false)
    }
}
