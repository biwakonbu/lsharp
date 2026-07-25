use lsharp_syntax::ast::{Expr, Param};
use lsharp_syntax::span::Span;
use lsharp_types::{infer::ExprTypeKey, types::Type};

use super::WasmGcCapturedLambdaInfo;
use crate::lower::{FuncCtx, Lower, LowerError, is_builtin_binop, type_expr_to_name, type_to_name};
use crate::{Function, GcField, GcTypeDef, GcTypeKind, Instruction, IrType, closure};

impl Lower {
    pub(super) fn wasmgc_lambda_free_vars(&self, params: &[Param], body: &Expr) -> Vec<String> {
        let param_names: Vec<String> = params.iter().map(|param| param.name.clone()).collect();
        let mut free_vars: Vec<String> = closure::free_variables(&param_names, body)
            .into_iter()
            .filter(|name| {
                !is_builtin_binop(name)
                    && name != "not"
                    && name != "print"
                    && name != "__alloc"
                    && name != "proc-exit"
                    && !self.func_indices.contains_key(name)
            })
            .collect();
        free_vars.sort();
        free_vars
    }

    pub(super) fn lower_wasmgc_captured_lambda_value(
        &mut self,
        ctx: &mut FuncCtx,
        lambda_span: Span,
        params: &[Param],
        body: &Expr,
        free_var_list: &[String],
    ) -> Result<WasmGcCapturedLambdaInfo, LowerError> {
        let lambda_name = self.fresh_lambda_name();
        let lambda_type = self
            .expr_type_results
            .get(&ExprTypeKey::new(&ctx.type_scope_key, lambda_span))
            .cloned();
        let (param_types, param_type_names, inferred_result_type) = match lambda_type {
            Some(Type::Fun(inferred_params, ret)) if inferred_params.len() == params.len() => (
                inferred_params
                    .iter()
                    .map(|ty| self.ir_type_for_type(ty))
                    .collect::<Vec<_>>(),
                inferred_params.iter().map(type_to_name).collect::<Vec<_>>(),
                Some(self.ir_type_for_type(&ret)),
            ),
            _ => (
                params
                    .iter()
                    .map(|param| {
                        param
                            .ty
                            .as_ref()
                            .map(|ty| self.type_expr_to_ir(ty))
                            .unwrap_or(IrType::I64)
                    })
                    .collect::<Vec<_>>(),
                params
                    .iter()
                    .map(|param| param.ty.as_ref().and_then(type_expr_to_name))
                    .collect::<Vec<_>>(),
                None,
            ),
        };

        let mut capture_types = Vec::with_capacity(free_var_list.len());
        for name in free_var_list {
            let Some(&local_idx) = ctx.locals_map.get(name) else {
                return Err(LowerError::UndefinedFunction {
                    name: name.clone(),
                    span: Some(lambda_span),
                });
            };
            let Some(&local_type) = ctx.local_types.get(local_idx as usize) else {
                return Err(LowerError::Unsupported {
                    msg: format!("WasmGC captured local の型を解決できません: {name}"),
                    span: Some(lambda_span),
                });
            };
            capture_types.push(local_type);
        }

        let env_type_index = self.gc_types.len() as u32;
        let func_idx = self.next_func_idx + self.lifted_functions.len() as u32;
        let local_func_idx =
            func_idx
                .checked_sub(self.import_count)
                .ok_or_else(|| LowerError::Unsupported {
                    msg:
                        "WasmGC captured lambda の function index が runtime import 境界より前です"
                            .to_string(),
                    span: Some(lambda_span),
                })?;
        let call_ref_type_index = env_type_index + 1 + local_func_idx;
        let mut env_fields = vec![GcField {
            name: "function".to_string(),
            ty: IrType::TypedFuncRef(call_ref_type_index),
            mutable: false,
        }];
        env_fields.extend(capture_types.iter().enumerate().map(|(index, ty)| GcField {
            name: format!("capture_{index}"),
            ty: *ty,
            mutable: false,
        }));
        self.gc_types.push(GcTypeDef {
            name: format!("__closure_env_{lambda_name}"),
            kind: GcTypeKind::Struct(env_fields),
        });

        let mut lifted_ctx =
            FuncCtx::with_type_scope(lambda_name.clone(), ctx.type_scope_key.clone());
        for (param_idx, param) in params.iter().enumerate() {
            let idx = lifted_ctx.next_local;
            lifted_ctx.locals_map.insert(param.name.clone(), idx);
            if let Some(type_name) = param_type_names.get(param_idx).cloned().flatten() {
                lifted_ctx
                    .local_type_names
                    .insert(param.name.clone(), type_name);
            }
            lifted_ctx.param_count += 1;
            lifted_ctx.next_local += 1;
            lifted_ctx
                .local_types
                .push(param_types.get(param_idx).copied().unwrap_or(IrType::I64));
        }

        let env_param_idx = lifted_ctx.next_local;
        lifted_ctx
            .locals_map
            .insert("__closure_env".to_string(), env_param_idx);
        lifted_ctx.param_count += 1;
        lifted_ctx.next_local += 1;
        lifted_ctx.local_types.push(IrType::Ref(env_type_index));

        let mut prologue = Vec::with_capacity(free_var_list.len() * 4);
        for (capture_index, (name, capture_type)) in
            free_var_list.iter().zip(&capture_types).enumerate()
        {
            let capture_local = lifted_ctx.alloc_local_typed(name.clone(), *capture_type);
            prologue.push(Instruction::LocalGet(env_param_idx));
            prologue.push(Instruction::StructGet(
                env_type_index,
                capture_index as u32 + 1,
            ));
            prologue.push(Instruction::LocalSet(capture_local));
        }

        self.lower_expr(&mut lifted_ctx, body)?;
        let result_type = inferred_result_type
            .or_else(|| {
                self.infer_expr_type_name_with_ctx(&lifted_ctx, body)
                    .map(|name| self.ir_type_for_type_name(&name))
            })
            .unwrap_or(IrType::I64);
        let extra_local_count = (lifted_ctx.next_local - lifted_ctx.param_count) as usize;
        let extra_locals = lifted_ctx
            .local_types
            .get(lifted_ctx.param_count as usize..)
            .filter(|types| types.len() == extra_local_count)
            .map_or_else(
                || vec![IrType::I64; extra_local_count],
                |types| types.to_vec(),
            );
        let mut full_body = prologue;
        full_body.extend(lifted_ctx.instructions);
        let mut lifted_params = param_types;
        lifted_params.push(IrType::Ref(env_type_index));
        self.lifted_functions.push(Function {
            name: lambda_name,
            params: lifted_params,
            result: result_type,
            locals: extra_locals,
            body: full_body,
            is_export: false,
        });

        ctx.emit(Instruction::RefFunc(local_func_idx));
        for name in free_var_list {
            let local_idx =
                ctx.locals_map
                    .get(name)
                    .copied()
                    .ok_or_else(|| LowerError::UndefinedFunction {
                        name: name.clone(),
                        span: Some(lambda_span),
                    })?;
            ctx.emit(Instruction::LocalGet(local_idx));
        }
        ctx.emit(Instruction::StructNew(env_type_index));
        Ok(WasmGcCapturedLambdaInfo {
            env_type_index,
            call_ref_type_index,
        })
    }

    pub(super) fn wasmgc_env_call_ref_type(&self, env_type_index: u32) -> Option<u32> {
        let GcTypeKind::Struct(fields) = &self.gc_types.get(env_type_index as usize)?.kind else {
            return None;
        };
        match fields.first().map(|field| field.ty) {
            Some(IrType::TypedFuncRef(type_index)) => Some(type_index),
            _ => None,
        }
    }
}
