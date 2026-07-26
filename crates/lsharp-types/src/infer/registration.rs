use super::*;

impl Infer {
    /// 型定義のコンストラクタを環境に登録
    pub(super) fn register_type_def(
        &mut self,
        env: &mut TypeEnv,
        type_name: &str,
        type_params: &[String],
        variants: &[Variant],
    ) -> Result<(), TypeError> {
        let param_vars: Vec<(String, TypeVarId)> = type_params
            .iter()
            .map(|p| (p.clone(), self.var_gen.fresh_id()))
            .collect();

        let result_type = if param_vars.is_empty() {
            Type::Con(type_name.to_string())
        } else {
            Type::App(
                type_name.to_string(),
                param_vars.iter().map(|(_, id)| Type::Var(*id)).collect(),
            )
        };

        let bound_vars: Vec<TypeVarId> = param_vars.iter().map(|(_, id)| *id).collect();

        // Kind を推論して登録
        let kind = if type_params.is_empty() {
            Kind::star()
        } else {
            // n 引数の型コンストラクタ: * -> * -> ... -> *
            type_params.iter().rev().fold(Kind::star(), |acc, _| {
                Kind::Arrow(Box::new(Kind::Star), Box::new(acc))
            })
        };
        self.kind_env.insert(type_name.to_string(), kind);

        for variant in variants {
            // GADT: バリアント別の戻り型がある場合はそれを使用
            let variant_ret_type = if let Some(ref ret_type_expr) = variant.return_type {
                let gadt_ret = self.resolve_type_expr(ret_type_expr, &param_vars);
                // GADT 戻り型を記録（パターンマッチでの型絞り込みに使用）
                self.gadt_return_types
                    .insert(variant.name.clone(), gadt_ret.clone());
                gadt_ret
            } else {
                result_type.clone()
            };

            let ctor_type = if variant.fields.is_empty() {
                variant_ret_type
            } else {
                let field_types: Vec<Type> = variant
                    .fields
                    .iter()
                    .map(|f| self.resolve_type_expr(f, &param_vars))
                    .collect();
                Type::Fun(field_types, Box::new(variant_ret_type))
            };

            let scheme = TypeScheme {
                vars: bound_vars.clone(),
                constraints: Vec::new(),
                ty: ctor_type,
            };
            env.insert(variant.name.clone(), scheme);
        }

        Ok(())
    }

    /// レコード型定義を登録
    pub(super) fn register_record_def(
        &mut self,
        env: &mut TypeEnv,
        type_name: &str,
        type_params: &[String],
        fields: &[(String, TypeExpr)],
    ) -> Result<(), TypeError> {
        let param_vars: Vec<(String, TypeVarId)> = type_params
            .iter()
            .map(|p| (p.clone(), self.var_gen.fresh_id()))
            .collect();

        let bound_vars: Vec<TypeVarId> = param_vars.iter().map(|(_, id)| *id).collect();

        let record_fields: Vec<(String, Type)> = fields
            .iter()
            .map(|(name, ty_expr)| (name.clone(), self.resolve_type_expr(ty_expr, &param_vars)))
            .collect();

        let record_type = Type::Record(type_name.to_string(), record_fields.clone());

        // Kind を推論して登録
        let kind = if type_params.is_empty() {
            Kind::star()
        } else {
            type_params.iter().rev().fold(Kind::star(), |acc, _| {
                Kind::Arrow(Box::new(Kind::Star), Box::new(acc))
            })
        };
        self.kind_env.insert(type_name.to_string(), kind);

        self.record_registry.insert(
            type_name.to_string(),
            RecordInfo {
                name: type_name.to_string(),
                type_params: bound_vars.clone(),
                fields: record_fields.clone(),
            },
        );

        // コンストラクタを環境に登録
        let field_types: Vec<Type> = record_fields.iter().map(|(_, t)| t.clone()).collect();
        let ctor_type = if field_types.is_empty() {
            record_type.clone()
        } else {
            Type::Fun(field_types, Box::new(record_type.clone()))
        };

        let ctor_scheme = TypeScheme {
            vars: bound_vars.clone(),
            constraints: Vec::new(),
            ty: ctor_type,
        };
        env.insert(type_name.to_string(), ctor_scheme);

        // フィールドアクセサを登録
        for (field_name, field_type) in &record_fields {
            let accessor_name = format!("{type_name}.{field_name}");
            let accessor_type = Type::Fun(vec![record_type.clone()], Box::new(field_type.clone()));
            let accessor_scheme = TypeScheme {
                vars: bound_vars.clone(),
                constraints: Vec::new(),
                ty: accessor_type,
            };
            env.insert(accessor_name, accessor_scheme);
        }

        Ok(())
    }

    /// 型エイリアスを登録
    pub(super) fn register_type_alias(
        &mut self,
        name: &str,
        params: &[String],
        target: &TypeExpr,
        span: Span,
    ) -> Result<(), TypeError> {
        // 再帰エイリアスの検出: ターゲット型にエイリアス名自体が含まれないか
        if self.type_alias_contains_self(name, target) {
            return Err(TypeError::RecursiveAlias {
                name: name.to_string(),
                span,
            });
        }

        let param_vars: Vec<(String, TypeVarId)> = params
            .iter()
            .map(|p| (p.clone(), self.var_gen.fresh_id()))
            .collect();

        let resolved = self.resolve_type_expr(target, &param_vars);
        self.type_aliases
            .insert(name.to_string(), (params.to_vec(), resolved));

        Ok(())
    }

    /// 型エイリアスのターゲット型に自身が含まれるかチェック（再帰検出）
    fn type_alias_contains_self(&self, alias_name: &str, target: &TypeExpr) -> bool {
        match target {
            TypeExpr::Named(_, name) | TypeExpr::Var(_, name) => name == alias_name,
            TypeExpr::App(_, base, args) => {
                self.type_alias_contains_self(alias_name, base)
                    || args
                        .iter()
                        .any(|a| self.type_alias_contains_self(alias_name, a))
            }
            TypeExpr::Fun(_, params, ret) => {
                params
                    .iter()
                    .any(|p| self.type_alias_contains_self(alias_name, p))
                    || self.type_alias_contains_self(alias_name, ret)
            }
            TypeExpr::Record(_, fields) => fields
                .iter()
                .any(|(_, t)| self.type_alias_contains_self(alias_name, t)),
        }
    }

    /// 制約付き型を登録
    pub(super) fn register_type_constrained(
        &mut self,
        env: &mut TypeEnv,
        name: &str,
        base_type: &TypeExpr,
        constraints: &[Constraint],
        _span: Span,
    ) -> Result<(), TypeError> {
        let resolved_base = self.resolve_type_expr(base_type, &[]);

        // 制約を ConstraintDef に変換
        let constraint_defs: Vec<ConstraintDef> = constraints
            .iter()
            .filter_map(|c| self.constraint_to_def(c))
            .collect();

        self.constrained_types.insert(
            name.to_string(),
            ConstrainedTypeInfo {
                name: name.to_string(),
                base_type: resolved_base.clone(),
                constraints: constraint_defs,
            },
        );

        // 制約付き型はベース型のエイリアスとして扱う（型推論時は透過）
        self.type_aliases
            .insert(name.to_string(), (Vec::new(), resolved_base));

        // スマートコンストラクタ Name.new : BaseType -> Name を登録
        let new_type = Type::Fun(
            vec![self.resolve_type_expr(base_type, &[])],
            Box::new(Type::Con(name.to_string())),
        );
        env.insert(format!("{name}.new"), TypeScheme::mono(new_type));

        // Name.value : Name -> BaseType を登録
        let value_type = Type::Fun(
            vec![Type::Con(name.to_string())],
            Box::new(self.resolve_type_expr(base_type, &[])),
        );
        env.insert(format!("{name}.value"), TypeScheme::mono(value_type));

        // Name.valid? : BaseType -> Bool を登録
        let valid_type = Type::Fun(
            vec![self.resolve_type_expr(base_type, &[])],
            Box::new(Type::bool()),
        );
        env.insert(format!("{name}.valid?"), TypeScheme::mono(valid_type));

        Ok(())
    }

    /// AST の Constraint を ConstraintDef に変換
    fn constraint_to_def(&self, constraint: &Constraint) -> Option<ConstraintDef> {
        match constraint {
            Constraint::Gte(expr) => {
                if let Expr::Lit(_, Literal::Int(n)) = expr {
                    Some(ConstraintDef::Gte(*n))
                } else {
                    None
                }
            }
            Constraint::Lte(expr) => {
                if let Expr::Lit(_, Literal::Int(n)) = expr {
                    Some(ConstraintDef::Lte(*n))
                } else {
                    None
                }
            }
            Constraint::Range(lo, hi) => {
                if let (Expr::Lit(_, Literal::Int(l)), Expr::Lit(_, Literal::Int(h))) = (lo, hi) {
                    Some(ConstraintDef::Range(*l, *h))
                } else {
                    None
                }
            }
            Constraint::Matches(pattern) => Some(ConstraintDef::Matches(pattern.clone())),
            Constraint::MinLength(expr) => {
                if let Expr::Lit(_, Literal::Int(n)) = expr {
                    Some(ConstraintDef::MinLength(*n as usize))
                } else {
                    None
                }
            }
            Constraint::MaxLength(expr) => {
                if let Expr::Lit(_, Literal::Int(n)) = expr {
                    Some(ConstraintDef::MaxLength(*n as usize))
                } else {
                    None
                }
            }
            Constraint::OneOf(exprs) => {
                let values: Vec<i64> = exprs
                    .iter()
                    .filter_map(|e| {
                        if let Expr::Lit(_, Literal::Int(n)) = e {
                            Some(*n)
                        } else {
                            None
                        }
                    })
                    .collect();
                if values.len() == exprs.len() {
                    Some(ConstraintDef::OneOf(values))
                } else {
                    None
                }
            }
            Constraint::Satisfies(fn_name) => Some(ConstraintDef::Satisfies(fn_name.clone())),
        }
    }

    /// トレイト定義を登録
    pub(super) fn register_trait_def(
        &mut self,
        env: &mut TypeEnv,
        name: &str,
        type_param: &str,
        methods: &[TraitMethod],
        _span: Span,
    ) -> Result<(), TypeError> {
        let type_var = self.var_gen.fresh_id();

        let mut trait_methods = Vec::new();
        for method in methods {
            let param_vars = vec![(type_param.to_string(), type_var)];
            let mut param_types = Vec::new();
            for param in &method.params {
                let ty = if let Some(type_expr) = &param.ty {
                    self.resolve_type_expr(type_expr, &param_vars)
                } else if param.name == "self" {
                    Type::Var(type_var)
                } else {
                    self.var_gen.fresh()
                };
                param_types.push(ty);
            }

            let ret_ty = if let Some(ret_expr) = &method.return_ty {
                self.resolve_type_expr(ret_expr, &param_vars)
            } else {
                self.var_gen.fresh()
            };

            let method_type = Type::Fun(param_types, Box::new(ret_ty));
            let method_scheme = TypeScheme {
                vars: vec![type_var],
                constraints: vec![TraitConstraint {
                    trait_name: name.to_string(),
                    type_var,
                }],
                ty: method_type.clone(),
            };

            trait_methods.push((method.name.clone(), method_scheme.clone()));
            env.insert(method.name.clone(), method_scheme);

            // デフォルト実装がある場合はキャッシュに保存
            if let Some(ref default_body) = method.default_impl {
                self.default_impls.insert(
                    (name.to_string(), method.name.clone()),
                    (
                        method.params.clone(),
                        method.return_ty.clone(),
                        default_body.clone(),
                    ),
                );
            }
        }

        self.trait_registry.insert(
            name.to_string(),
            TraitInfo {
                name: name.to_string(),
                type_param: type_var,
                methods: trait_methods,
            },
        );

        Ok(())
    }

    /// impl 定義を登録
    pub(super) fn register_impl_def(
        &mut self,
        env: &mut TypeEnv,
        trait_name: &str,
        type_name: &str,
        methods: &[Decl],
        _span: Span,
    ) -> Result<(), TypeError> {
        // Kind 整合性チェック: トレイトが要求する Kind と実装型の Kind を比較
        if let Some(trait_kind) = self.kind_env.get(trait_name).cloned() {
            let type_kind = self
                .kind_env
                .get(type_name)
                .cloned()
                .unwrap_or(Kind::star());
            if !kinds_compatible(&trait_kind, &type_kind) {
                return Err(TypeError::KindMismatch {
                    type_name: type_name.to_string(),
                    trait_name: trait_name.to_string(),
                    expected_kind: trait_kind,
                    actual_kind: type_kind,
                    span: _span,
                });
            }
        }

        let mut method_types = Vec::new();

        for method_decl in methods {
            if let Decl::Defn {
                name,
                params,
                return_ty,
                body,
                span,
                ..
            } = method_decl
            {
                // impl メソッドの型推論
                let specialized_name = format!("{trait_name}::{name}${type_name}");
                let (subst, ty) = self.infer_defn(DefnInferenceInput {
                    env,
                    name,
                    expr_scope: &specialized_name,
                    params,
                    return_ty: return_ty.as_ref(),
                    body,
                    span: *span,
                })?;
                let final_ty = ty.apply_subst(&subst);

                // 特化された型を環境に登録
                // TraitName::method_name$TypeName のような内部名を使用
                let scheme = self.generalize(env, &final_ty);
                env.insert(specialized_name, scheme);

                method_types.push((name.clone(), final_ty));
            }
        }

        // デフォルト実装のフォールバック:
        // impl に定義されていないメソッドがトレイトにデフォルト実装を持つ場合、
        // デフォルト実装を使用する
        let trait_method_names: Vec<String> = self
            .trait_registry
            .get(trait_name)
            .map(|info| info.methods.iter().map(|(n, _)| n.clone()).collect())
            .unwrap_or_default();

        let impl_method_names: Vec<String> = method_types.iter().map(|(n, _)| n.clone()).collect();

        for trait_method_name in &trait_method_names {
            if !impl_method_names.contains(trait_method_name) {
                // デフォルト実装をキャッシュから取得
                let key = (trait_name.to_string(), trait_method_name.clone());
                if let Some((default_params, default_ret_ty, default_body)) =
                    self.default_impls.get(&key).cloned()
                {
                    // デフォルト実装を型推論
                    let dummy_span = Span { start: 0, end: 0 };
                    let specialized_name = format!("{trait_name}::{trait_method_name}${type_name}");
                    let result = self.infer_defn(DefnInferenceInput {
                        env,
                        name: trait_method_name,
                        expr_scope: &specialized_name,
                        params: &default_params,
                        return_ty: default_ret_ty.as_ref(),
                        body: &default_body,
                        span: dummy_span,
                    });

                    if let Ok((subst, ty)) = result {
                        let final_ty = ty.apply_subst(&subst);
                        let scheme = self.generalize(env, &final_ty);
                        env.insert(specialized_name, scheme);
                        method_types.push((trait_method_name.clone(), final_ty));
                    }
                    // 型推論に失敗した場合はスキップ（エラーにしない）
                }
            }
        }

        self.impl_registry.push(ImplInfo {
            trait_name: trait_name.to_string(),
            type_name: type_name.to_string(),
            methods: method_types,
        });

        Ok(())
    }
}
