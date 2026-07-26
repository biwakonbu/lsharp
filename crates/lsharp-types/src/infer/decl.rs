use super::*;
impl Infer {
    /// プログラム全体を型チェック
    pub fn infer_program(
        &mut self,
        program: &Program,
    ) -> Result<Vec<(String, TypeScheme)>, TypeError> {
        self.expr_type_results.clear();
        self.ambiguous_expr_type_keys.clear();
        self.current_expr_scope = None;

        let mut env = self.builtin_env();

        // 外部モジュールの型環境を注入
        for (name, scheme) in &self.external_types {
            env.insert(name.clone(), scheme.clone());
        }

        let mut results = Vec::new();

        // まず全ての型定義を処理してコンストラクタを環境に登録
        for decl in &program.decls {
            match decl {
                Decl::TypeDef {
                    name,
                    type_params,
                    variants,
                    ..
                } => {
                    self.register_type_def(&mut env, name, type_params, variants)?;
                }
                Decl::RecordDef {
                    name,
                    type_params,
                    fields,
                    ..
                } => {
                    self.register_record_def(&mut env, name, type_params, fields)?;
                }
                Decl::TypeAlias {
                    name,
                    params,
                    target,
                    span,
                    ..
                } => {
                    self.register_type_alias(name, params, target, *span)?;
                }
                Decl::TypeConstrained {
                    name,
                    base_type,
                    constraints,
                    span,
                    ..
                } => {
                    self.register_type_constrained(&mut env, name, base_type, constraints, *span)?;
                }
                Decl::TraitDef {
                    name,
                    type_param,
                    methods,
                    span,
                    ..
                } => {
                    self.register_trait_def(&mut env, name, type_param, methods, *span)?;
                }
                Decl::ImplDef {
                    trait_name,
                    type_name,
                    methods,
                    span,
                    ..
                } => {
                    self.register_impl_def(&mut env, trait_name, type_name, methods, *span)?;
                }
                Decl::ModuleDecl { name, body, .. } => {
                    self.module_env.name = Some(name.clone());
                    // ネストモジュールの本体宣言を修飾名で登録
                    self.register_nested_module_types(&mut env, name, body)?;
                }
                Decl::ImportDecl {
                    module,
                    alias,
                    only,
                    open,
                    ..
                } => {
                    self.module_env.imports.push(ModuleImport {
                        module: module.clone(),
                        alias: alias.clone(),
                        only: only.clone(),
                        open: *open,
                    });
                }
                Decl::ComputationBuilder {
                    name,
                    bind_fn,
                    return_fn,
                    ..
                } => {
                    self.computation_builders
                        .insert(name.clone(), (bind_fn.clone(), return_fn.clone()));
                }
                Decl::Private { inner, .. } => {
                    // Private 内の宣言も型登録する（内部名は同じ）
                    // 可視性情報はモジュール環境に記録
                    match inner.as_ref() {
                        Decl::Defn { name, .. } => {
                            self.module_env.privates.push(name.clone());
                        }
                        Decl::TypeDef { name, .. }
                        | Decl::RecordDef { name, .. }
                        | Decl::TypeAlias { name, .. }
                        | Decl::TypeConstrained { name, .. } => {
                            self.module_env.privates.push(name.clone());
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        // 次に全ての関数定義を型推論
        self.infer_decl_functions(&mut env, &mut results, &program.decls, None)?;

        // トレイト制約のチェック
        self.check_pending_constraints(&self.global_subst.clone())?;

        Ok(results)
    }

    /// ネストモジュールの型定義を修飾名で登録
    fn register_nested_module_types(
        &mut self,
        env: &mut TypeEnv,
        module_name: &str,
        body: &[Decl],
    ) -> Result<(), TypeError> {
        for decl in body {
            match decl {
                Decl::TypeDef {
                    name,
                    type_params,
                    variants,
                    ..
                } => {
                    let qualified = format!("{module_name}.{name}");
                    self.register_type_def(env, &qualified, type_params, variants)?;
                }
                Decl::RecordDef {
                    name,
                    type_params,
                    fields,
                    ..
                } => {
                    let qualified = format!("{module_name}.{name}");
                    self.register_record_def(env, &qualified, type_params, fields)?;
                }
                Decl::TypeAlias {
                    name,
                    params,
                    target,
                    span,
                    ..
                } => {
                    let qualified = format!("{module_name}.{name}");
                    self.register_type_alias(&qualified, params, target, *span)?;
                }
                Decl::TypeConstrained {
                    name,
                    base_type,
                    constraints,
                    span,
                    ..
                } => {
                    let qualified = format!("{module_name}.{name}");
                    self.register_type_constrained(env, &qualified, base_type, constraints, *span)?;
                }
                Decl::TraitDef {
                    name,
                    type_param,
                    methods,
                    span,
                    ..
                } => {
                    let qualified = format!("{module_name}.{name}");
                    self.register_trait_def(env, &qualified, type_param, methods, *span)?;
                }
                Decl::ImplDef {
                    trait_name,
                    type_name,
                    methods,
                    span,
                    ..
                } => {
                    self.register_impl_def(env, trait_name, type_name, methods, *span)?;
                }
                Decl::ModuleDecl {
                    name: inner_name,
                    body: inner_body,
                    ..
                } => {
                    // 再帰的にネストモジュールを処理（修飾名を連結）
                    let qualified = format!("{module_name}.{inner_name}");
                    self.register_nested_module_types(env, &qualified, inner_body)?;
                }
                Decl::ImportDecl {
                    module,
                    alias,
                    only,
                    open,
                    ..
                } => {
                    self.module_env.imports.push(ModuleImport {
                        module: module.clone(),
                        alias: alias.clone(),
                        only: only.clone(),
                        open: *open,
                    });
                }
                Decl::Private { inner, .. } => match inner.as_ref() {
                    Decl::Defn { name, .. } => {
                        let qualified = format!("{module_name}.{name}");
                        self.module_env.privates.push(qualified);
                    }
                    Decl::TypeDef { name, .. }
                    | Decl::RecordDef { name, .. }
                    | Decl::TypeAlias { name, .. }
                    | Decl::TypeConstrained { name, .. } => {
                        let qualified = format!("{module_name}.{name}");
                        self.module_env.privates.push(qualified);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(())
    }

    /// 宣言リストから関数定義を型推論（ネストモジュール対応、2パス方式で相互再帰対応）
    fn infer_decl_functions(
        &mut self,
        env: &mut TypeEnv,
        results: &mut Vec<(String, TypeScheme)>,
        decls: &[Decl],
        module_prefix: Option<&str>,
    ) -> Result<(), TypeError> {
        // パス1: 全 defn の名前に型変数を仮登録（前方参照を可能にする）
        let mut defn_infos: Vec<(String, &[Param], Option<&TypeExpr>, &Expr, Span, Type)> =
            Vec::new();
        for decl in decls {
            let actual_decl = match decl {
                Decl::Private { inner, .. } => inner.as_ref(),
                other => other,
            };
            match actual_decl {
                Decl::Defn {
                    name,
                    params,
                    return_ty,
                    body,
                    span,
                    ..
                } => {
                    let qualified_name = if let Some(prefix) = module_prefix {
                        format!("{prefix}.{name}")
                    } else {
                        name.clone()
                    };
                    let placeholder_ty = self.var_gen.fresh();
                    env.insert(
                        qualified_name.clone(),
                        TypeScheme::mono(placeholder_ty.clone()),
                    );
                    defn_infos.push((
                        qualified_name,
                        params.as_slice(),
                        return_ty.as_ref(),
                        body,
                        *span,
                        placeholder_ty,
                    ));
                }
                Decl::ModuleDecl { name, body, .. } if !body.is_empty() => {
                    let prefix = if let Some(outer) = module_prefix {
                        format!("{outer}.{name}")
                    } else {
                        name.clone()
                    };
                    self.infer_decl_functions(env, results, body, Some(&prefix))?;
                }
                _ => {}
            }
        }

        // パス2: 各 defn の body を本推論（仮登録された型変数を通じて前方参照が可能）
        // 逐次的に推論・generalize し、env を更新していく
        let pending_names: Vec<String> = defn_infos
            .iter()
            .map(|(qualified_name, _, _, _, _, _)| qualified_name.clone())
            .collect();
        for (index, (qualified_name, params, return_ty, body, span, placeholder_ty)) in
            defn_infos.into_iter().enumerate()
        {
            let (subst, ty) = self.infer_defn(DefnInferenceInput {
                env,
                name: &qualified_name,
                expr_scope: &qualified_name,
                params,
                return_ty,
                body,
                span,
            })?;
            // 仮登録型変数と推論結果の関数型を unify（循環参照の型を結びつける）
            let resolved_placeholder = placeholder_ty.apply_subst(&subst);
            let resolved_ty = ty.apply_subst(&subst);
            let s_extra = self.unify(&resolved_placeholder, &resolved_ty, span)?;
            let subst = subst.compose(&s_extra);

            let env_after = env.apply_subst(&subst);
            let final_ty = ty.apply_subst(&subst);
            // generalize 時に未確定の top-level 仮登録型を除外する。
            // import 解決では別 Infer で external_types を注入するため、
            // 残存 placeholder が env 側にいると under-generalize されてしまう。
            let mut env_for_gen = env_after.clone();
            for pending_name in pending_names.iter().skip(index) {
                env_for_gen.remove(pending_name);
            }
            let scheme = self.generalize(&env_for_gen, &final_ty);
            *env = env_after.extend(qualified_name.clone(), scheme.clone());
            results.push((qualified_name, scheme));
        }

        Ok(())
    }

    /// defn signature 内の lower-case 型変数を scope ごとに束縛する。
    fn collect_defn_type_var_names(
        &self,
        type_expr: &TypeExpr,
        names: &mut Vec<String>,
        seen: &mut HashSet<String>,
    ) {
        match type_expr {
            TypeExpr::Var(_, name) => {
                if !self.type_aliases.contains_key(name) && seen.insert(name.clone()) {
                    names.push(name.clone());
                }
            }
            TypeExpr::App(_, base, args) => {
                self.collect_defn_type_var_names(base, names, seen);
                for arg in args {
                    self.collect_defn_type_var_names(arg, names, seen);
                }
            }
            TypeExpr::Fun(_, params, ret) => {
                for param in params {
                    self.collect_defn_type_var_names(param, names, seen);
                }
                self.collect_defn_type_var_names(ret, names, seen);
            }
            TypeExpr::Record(_, fields) => {
                for (_, field_ty) in fields {
                    self.collect_defn_type_var_names(field_ty, names, seen);
                }
            }
            TypeExpr::Named(_, _) => {}
        }
    }

    fn defn_type_vars(
        &mut self,
        params: &[Param],
        return_ty: Option<&TypeExpr>,
    ) -> Vec<(String, TypeVarId)> {
        let mut names = Vec::new();
        let mut seen = HashSet::new();
        for param in params {
            if let Some(type_expr) = &param.ty {
                self.collect_defn_type_var_names(type_expr, &mut names, &mut seen);
            }
        }
        if let Some(type_expr) = return_ty {
            self.collect_defn_type_var_names(type_expr, &mut names, &mut seen);
        }
        names
            .into_iter()
            .map(|name| {
                let type_var = self.var_gen.fresh_id();
                (name, type_var)
            })
            .collect()
    }

    /// 関数定義の型推論
    pub(super) fn infer_defn(
        &mut self,
        input: DefnInferenceInput<'_>,
    ) -> Result<(Substitution, Type), TypeError> {
        let DefnInferenceInput {
            env,
            name,
            expr_scope,
            params,
            return_ty,
            body,
            span,
        } = input;
        let mut local_env = env.clone();
        let defn_type_vars = self.defn_type_vars(params, return_ty);

        // 再帰呼び出し用: 関数自身を型変数として環境に仮登録
        let self_ty = self.var_gen.fresh();
        local_env.insert(name.to_string(), TypeScheme::mono(self_ty.clone()));

        // パラメータの型変数を生成
        let mut param_types = Vec::new();
        for param in params {
            let ty = if let Some(type_expr) = &param.ty {
                self.resolve_type_expr(type_expr, &defn_type_vars)
            } else {
                self.var_gen.fresh()
            };
            local_env.insert(param.name.clone(), TypeScheme::mono(ty.clone()));
            param_types.push(ty);
        }

        // 本体を型推論
        let previous_scope = self.current_expr_scope.replace(expr_scope.to_string());
        let body_result = self.infer_expr(&local_env, body);
        self.current_expr_scope = previous_scope;
        let (subst, body_type) = body_result?;

        // 戻り値型注釈があれば統合
        let subst = if let Some(ret_ty_expr) = return_ty {
            let ret_ty = self.resolve_type_expr(ret_ty_expr, &defn_type_vars);
            let s2 = self.unify(&body_type.apply_subst(&subst), &ret_ty, span)?;
            subst.compose(&s2)
        } else {
            subst
        };

        // 関数型を構築
        let final_param_types: Vec<Type> =
            param_types.iter().map(|t| t.apply_subst(&subst)).collect();
        let final_ret_type = body_type.apply_subst(&subst);
        let func_type = Type::Fun(final_param_types, Box::new(final_ret_type));

        // 再帰呼び出し用の仮型と実際の関数型を統合
        let s_self = self.unify(&self_ty.apply_subst(&subst), &func_type, span)?;
        let subst = subst.compose(&s_self);
        let func_type = func_type.apply_subst(&s_self);

        Ok((subst, func_type))
    }
}
