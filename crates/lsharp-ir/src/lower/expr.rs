//! 式の lowering (lower_expr 関連)

use lsharp_syntax::ast::*;

use super::{FuncCtx, Lower, LowerBackend, LowerError};

mod ann_expr;
#[cfg(test)]
mod ann_expr_tests;
mod application;
mod application_calls;
mod application_map;
mod application_ref_vector;
mod application_scalar;
mod application_string_heap;
mod application_strings;
#[cfg(test)]
mod application_tests;
mod computation;
#[cfg(test)]
mod computation_tests;
mod do_expr;
#[cfg(test)]
mod do_expr_tests;
mod helpers;
#[cfg(test)]
mod helpers_tests;
mod if_expr;
#[cfg(test)]
mod if_expr_tests;
mod lambda;
#[cfg(test)]
mod lambda_tests;
mod let_expr;
#[cfg(test)]
mod let_expr_tests;
mod literal_expr;
#[cfg(test)]
mod literal_expr_tests;
mod match_expr;
#[cfg(test)]
mod match_expr_tests;
mod quote_expr;
#[cfg(test)]
mod quote_expr_tests;
mod record;
#[cfg(test)]
mod record_tests;
mod var_expr;
#[cfg(test)]
mod var_expr_tests;
mod wasmgc_lambda;
#[cfg(test)]
mod wasmgc_lambda_tests;

#[derive(Debug, Clone, Copy)]
struct WasmGcCapturedLambdaInfo {
    env_type_index: u32,
    call_ref_type_index: u32,
}

impl Lower {
    /// 式を IR 命令に変換（スタックマシン方式）
    pub(crate) fn lower_expr(&mut self, ctx: &mut FuncCtx, expr: &Expr) -> Result<(), LowerError> {
        match expr {
            Expr::Lit(expr_span, lit) => self.lower_lit(ctx, *expr_span, lit)?,

            Expr::Var(expr_span, name) => self.lower_var(ctx, *expr_span, name)?,

            Expr::If(_, cond, then, else_) => self.lower_if(ctx, cond, then, else_)?,

            Expr::Let(_, bindings, body) => {
                self.lower_let(ctx, bindings, body)?;
            }
            Expr::App(expr_span, func, args) => {
                self.lower_app(ctx, *expr_span, func, args)?;
            }
            Expr::Match(_, scrutinee, arms) => {
                self.lower_match_expr(ctx, scrutinee, arms)?;
            }

            Expr::Do(_, exprs) => {
                self.lower_do(ctx, exprs)?;
            }

            Expr::Lambda(_, params, body) => {
                self.lower_lambda(ctx, expr.span(), params, body)?;
            }
            Expr::Ann(_, expr, _) => self.lower_ann(ctx, expr)?,

            Expr::RecordLit(_, type_name, fields) => {
                self.lower_record_lit(ctx, type_name, fields)?;
            }
            Expr::FieldAccess(expr_span, expr, field_name) => {
                self.lower_field_access(ctx, *expr_span, expr, field_name)?;
            }
            Expr::RecordUpdate(_, base, update_fields) => {
                self.lower_record_update(ctx, base, update_fields)?;
            }
            Expr::Computation(span, builder_name, steps) => {
                self.lower_computation(ctx, *span, builder_name, steps)?;
            }
            Expr::Quote(expr_span, _)
            | Expr::Unquote(expr_span, _)
            | Expr::UnquoteSplice(expr_span, _) => self.lower_quote(*expr_span)?,
        }

        Ok(())
    }
}
