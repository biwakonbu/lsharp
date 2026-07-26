use std::collections::HashMap;

use lsharp_syntax::ast::*;
use lsharp_types::infer::ExprTypeKey;
use lsharp_types::types::{Type, TypeVarId};

use super::{FuncCtx, Lower, type_expr_to_name, type_to_name};

impl Lower {
    fn infer_cached_expr_type_name(&self, type_scope_key: &str, expr: &Expr) -> Option<String> {
        self.expr_type_results
            .get(&ExprTypeKey::new(type_scope_key, expr.span()))
            .and_then(type_to_name)
    }

    fn bind_type_var_name(
        &self,
        type_var_names: &mut HashMap<TypeVarId, String>,
        type_var: TypeVarId,
        actual_type_name: &str,
    ) -> bool {
        match type_var_names.get(&type_var) {
            Some(existing) => existing == actual_type_name,
            None => {
                type_var_names.insert(type_var, actual_type_name.to_string());
                true
            }
        }
    }

    fn collect_type_var_names_from_arg(
        &self,
        expected: &Type,
        actual_type_name: &str,
        type_var_names: &mut HashMap<TypeVarId, String>,
    ) -> bool {
        match expected {
            Type::Var(type_var) => {
                self.bind_type_var_name(type_var_names, *type_var, actual_type_name)
            }
            Type::Con(name) | Type::Record(name, _) | Type::App(name, _) => {
                name == actual_type_name
            }
            Type::Fun(_, _) => false,
        }
    }

    fn infer_type_name_with_type_var_names(
        &self,
        ty: &Type,
        type_var_names: &HashMap<TypeVarId, String>,
    ) -> Option<String> {
        match ty {
            Type::Var(type_var) => type_var_names.get(type_var).cloned(),
            _ => type_to_name(ty),
        }
    }

    fn infer_function_return_type_name_from_args(
        &self,
        local_type_names: &HashMap<String, String>,
        type_scope_key: &str,
        func_name: &str,
        args: &[Expr],
    ) -> Option<String> {
        let ty = self.type_results.get(func_name)?;
        let Type::Fun(params, ret) = ty else {
            return type_to_name(ty);
        };

        let mut type_var_names = HashMap::new();
        for (param_ty, arg) in params.iter().zip(args) {
            let Some(arg_type_name) =
                self.infer_expr_type_name_with_locals(local_type_names, type_scope_key, arg)
            else {
                continue;
            };
            if !self.collect_type_var_names_from_arg(param_ty, &arg_type_name, &mut type_var_names)
            {
                return None;
            }
        }

        self.infer_type_name_with_type_var_names(ret, &type_var_names)
            .or_else(|| type_to_name(ret))
    }

    fn infer_uniform_type_name(
        &self,
        mut type_names: impl Iterator<Item = Option<String>>,
    ) -> Option<String> {
        let first = type_names.next().flatten()?;
        if type_names.all(|type_name| type_name.as_deref() == Some(first.as_str())) {
            Some(first)
        } else {
            None
        }
    }

    fn infer_let_body_type_name(
        &self,
        local_type_names: &HashMap<String, String>,
        type_scope_key: &str,
        bindings: &[(Pattern, Expr)],
        body: &Expr,
    ) -> Option<String> {
        let mut local_type_names = local_type_names.clone();
        for (pattern, value) in bindings {
            let value_type =
                self.infer_expr_type_name_with_locals(&local_type_names, type_scope_key, value);
            if let (Pattern::Var(_, name), Some(type_name)) = (pattern, value_type) {
                local_type_names.insert(name.clone(), type_name);
            }
        }
        self.infer_expr_type_name_with_locals(&local_type_names, type_scope_key, body)
    }

    fn infer_expr_type_name_with_locals(
        &self,
        local_type_names: &HashMap<String, String>,
        type_scope_key: &str,
        expr: &Expr,
    ) -> Option<String> {
        if let Some(type_name) = self.infer_cached_expr_type_name(type_scope_key, expr) {
            return Some(type_name);
        }
        match expr {
            // リテラルから型を推定
            Expr::Lit(_, Literal::Int(_)) => Some("Int".to_string()),
            Expr::Lit(_, Literal::Float(_)) => Some("Float".to_string()),
            Expr::Lit(_, Literal::Bool(_)) => Some("Bool".to_string()),
            Expr::Lit(_, Literal::String(_)) => Some("String".to_string()),
            Expr::Lit(_, Literal::Unit) => Some("Unit".to_string()),
            // 変数の場合、型推論結果から型名を取得
            Expr::Var(_, name) => local_type_names
                .get(name)
                .cloned()
                .or_else(|| self.type_results.get(name).and_then(type_to_name)),
            // 型注釈がある場合
            Expr::Ann(_, _, type_expr) => type_expr_to_name(type_expr),
            // レコードリテラルの場合、型名が明示的
            Expr::RecordLit(_, type_name, _) => Some(type_name.clone()),
            Expr::RecordUpdate(_, base, _) => {
                self.infer_expr_type_name_with_locals(local_type_names, type_scope_key, base)
            }
            Expr::Lambda(_, _, _) => Some("Closure".to_string()),
            Expr::If(_, _, then_expr, else_expr) => self.infer_uniform_type_name(
                [
                    self.infer_expr_type_name_with_locals(
                        local_type_names,
                        type_scope_key,
                        then_expr,
                    ),
                    self.infer_expr_type_name_with_locals(
                        local_type_names,
                        type_scope_key,
                        else_expr,
                    ),
                ]
                .into_iter(),
            ),
            Expr::Match(_, _, arms) => self.infer_uniform_type_name(arms.iter().map(|arm| {
                self.infer_expr_type_name_with_locals(local_type_names, type_scope_key, &arm.body)
            })),
            Expr::Do(_, exprs) => exprs.last().and_then(|expr| {
                self.infer_expr_type_name_with_locals(local_type_names, type_scope_key, expr)
            }),
            Expr::Let(_, bindings, body) => {
                self.infer_let_body_type_name(local_type_names, type_scope_key, bindings, body)
            }
            // 関数呼び出しの場合、戻り値型を推定
            Expr::App(_, func, args) => {
                if let Expr::Var(_, func_name) = func.as_ref() {
                    if let Some(type_name) = self.infer_builtin_return_type_name(func_name) {
                        return Some(type_name);
                    }
                    self.infer_function_return_type_name_from_args(
                        local_type_names,
                        type_scope_key,
                        func_name,
                        args,
                    )
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn infer_builtin_return_type_name(&self, func_name: &str) -> Option<String> {
        match func_name {
            "string-concat" | "substring" | "int-to-string" | "read-file" | "command-line-arg"
            | "read-stdin" => Some("String".to_string()),
            "vector-new" | "vector-push" | "vector-set" => Some("Vector".to_string()),
            "map-new" | "map-insert" | "map-remove" => Some("Map".to_string()),
            "ref-new" => Some("Ref".to_string()),
            _ => None,
        }
    }

    /// 式の型名を推定する（静的ディスパッチ用の簡易推定）
    pub(crate) fn infer_expr_type_name(&self, expr: &Expr) -> Option<String> {
        let local_type_names = HashMap::new();
        self.infer_expr_type_name_with_locals(&local_type_names, "", expr)
    }

    pub(crate) fn infer_expr_type_name_with_ctx(
        &self,
        ctx: &FuncCtx,
        expr: &Expr,
    ) -> Option<String> {
        self.infer_expr_type_name_with_locals(&ctx.local_type_names, &ctx.type_scope_key, expr)
    }
}
