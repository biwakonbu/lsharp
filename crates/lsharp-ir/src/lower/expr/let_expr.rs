use lsharp_syntax::ast::{Expr, Pattern};

use crate::{Instruction, IrType};

use super::{FuncCtx, Lower, LowerBackend, LowerError};

impl Lower {
    pub(super) fn lower_let(
        &mut self,
        ctx: &mut FuncCtx,
        bindings: &[(Pattern, Expr)],
        body: &Expr,
    ) -> Result<(), LowerError> {
        let mut scoped_bindings = Vec::new();
        let result = (|| -> Result<(), LowerError> {
            for (pat, val) in bindings {
                let inferred_type_name = self.infer_expr_type_name_with_ctx(ctx, val);
                let is_captured_lambda = if self.backend == LowerBackend::WasmGc
                    && let Expr::Lambda(lambda_span, params, body) = val
                {
                    let free_var_list = self.wasmgc_lambda_free_vars(params, body);
                    if free_var_list.is_empty() {
                        false
                    } else {
                        self.lower_wasmgc_captured_lambda_value(
                            ctx,
                            *lambda_span,
                            params,
                            body,
                            &free_var_list,
                        )?;
                        true
                    }
                } else {
                    false
                };
                if !is_captured_lambda {
                    self.lower_expr(ctx, val)?;
                }
                let lambda_func_index = if self.backend == LowerBackend::WasmGc
                    && matches!(val, Expr::Lambda(_, _, _))
                {
                    match ctx.instructions.last() {
                        Some(Instruction::RefFunc(function_index)) => Some(*function_index),
                        _ => None,
                    }
                } else {
                    None
                };
                let lambda_func_type_index =
                    lambda_func_index.map(|index| self.gc_types.len() as u32 + index);
                let lambda_env_type_index = if self.backend == LowerBackend::WasmGc
                    && matches!(val, Expr::Lambda(_, _, _))
                {
                    match ctx.instructions.last() {
                        Some(Instruction::StructNew(type_index)) => Some(*type_index),
                        _ => None,
                    }
                } else {
                    None
                };
                match pat {
                    Pattern::Var(_, name) => {
                        let previous_local = ctx.locals_map.get(name).copied();
                        let previous_type = ctx.local_type_names.get(name).cloned();
                        let binding_ir_type = lambda_env_type_index
                            .map(IrType::Ref)
                            .or_else(|| lambda_func_type_index.map(IrType::TypedFuncRef))
                            .or_else(|| {
                                inferred_type_name
                                    .as_deref()
                                    .map(|type_name| self.ir_type_for_type_name(type_name))
                                    .filter(|ty| {
                                        self.backend == LowerBackend::WasmGc
                                            && matches!(ty, IrType::Ref(_))
                                    })
                            })
                            .unwrap_or(IrType::I64);
                        let idx = ctx.alloc_scoped_local_typed(name.clone(), binding_ir_type);
                        if let Some(type_name) = inferred_type_name {
                            ctx.local_type_names.insert(name.clone(), type_name);
                        } else {
                            ctx.local_type_names.remove(name);
                        }
                        scoped_bindings.push((name.clone(), previous_local, previous_type));
                        ctx.emit(Instruction::LocalSet(idx));
                    }
                    Pattern::Wildcard(_) => {
                        ctx.emit(Instruction::Drop);
                    }
                    _ => {
                        // MVP: 複雑なパターンは未サポート
                        let idx = ctx.alloc_local("_pat".to_string());
                        ctx.emit(Instruction::LocalSet(idx));
                    }
                }
            }
            self.lower_expr(ctx, body)
        })();
        for (name, previous_local, previous_type) in scoped_bindings.into_iter().rev() {
            ctx.restore_local_binding(name, previous_local, previous_type);
        }
        result?;
        Ok(())
    }
}
