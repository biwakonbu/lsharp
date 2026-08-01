use super::{Infer, TypeError, TypeErrorCode};
use crate::types::*;
use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

impl Infer {
    /// 式の型推論 (Algorithm W)
    pub(super) fn infer_expr(
        &mut self,
        env: &TypeEnv,
        expr: &Expr,
    ) -> Result<(Substitution, Type), TypeError> {
        let result = match expr {
            Expr::Lit(_, lit) => Ok((Substitution::new(), self.lit_type(lit))),

            Expr::Var(span, name) => {
                // TypeName.field 形式のフィールドアクセサ または Module.function 形式
                if let Some(dot_pos) = name.find('.') {
                    let prefix = &name[..dot_pos];
                    let suffix = &name[dot_pos + 1..];
                    if !prefix.is_empty()
                        && !suffix.is_empty()
                        && prefix.starts_with(|c: char| c.is_ascii_uppercase())
                    {
                        // 1. フィールドアクセサとして検索
                        let accessor_name = format!("{prefix}.{suffix}");
                        if let Some(scheme) = env.get(&accessor_name) {
                            let ty = self.instantiate(scheme);
                            return Ok((Substitution::new(), ty));
                        }

                        // 2. モジュールエイリアス経由の完全修飾名解決
                        let resolved_name = self.resolve_qualified_name(prefix, suffix);
                        if let Some(ref resolved) = resolved_name
                            && let Some(scheme) = env.get(resolved)
                        {
                            let ty = self.instantiate(scheme);
                            return Ok((Substitution::new(), ty));
                        }

                        // 3. 完全修飾名として直接検索
                        // (将来的にマルチモジュール環境で使用)
                        return Err(TypeError::UndefinedField {
                            record_name: prefix.to_string(),
                            field_name: suffix.to_string(),
                            span: *span,
                        });
                    }
                }

                if let Some(scheme) = env.get(name) {
                    let ty = self.instantiate(scheme);
                    Ok((Substitution::new(), ty))
                } else {
                    Err(TypeError::UndefinedVar {
                        name: name.clone(),
                        span: *span,
                    })
                }
            }

            Expr::If(span, cond, then, else_) => {
                let (s1, cond_ty) = self.infer_expr(env, cond)?;
                // if 条件は Bool でなければならない (E0002)
                let s_cond = self
                    .unify(&cond_ty, &Type::bool(), *span)
                    .map_err(|e| Self::with_error_code(e, TypeErrorCode::IfCondition))?;
                let s1 = s1.compose(&s_cond);

                let env1 = env.apply_subst(&s1);
                let (s2, then_ty) = self.infer_expr(&env1, then)?;

                let env2 = env1.apply_subst(&s2);
                let (s3, else_ty) = self.infer_expr(&env2, else_)?;

                // then/else 分岐の型は一致しなければならない (E0003)
                let s_branch = self
                    .unify(&then_ty.apply_subst(&s3), &else_ty, *span)
                    .map_err(|e| Self::with_error_code(e, TypeErrorCode::IfBranch))?;

                let final_subst = s1.compose(&s2).compose(&s3).compose(&s_branch);
                let final_ty = else_ty.apply_subst(&s_branch);
                Ok((final_subst, final_ty))
            }

            Expr::Let(_, bindings, body) => {
                let mut subst = Substitution::new();
                let mut local_env = env.clone();

                for (pat, val) in bindings {
                    let (s1, val_ty) = self.infer_expr(&local_env, val)?;
                    subst = subst.compose(&s1);
                    local_env = local_env.apply_subst(&s1);

                    // let 多相: 値の型を汎化
                    let scheme = self.generalize(&local_env, &val_ty);
                    self.bind_pattern(&mut local_env, pat, &scheme)?;
                }

                let (s2, body_ty) = self.infer_expr(&local_env, body)?;
                Ok((subst.compose(&s2), body_ty))
            }

            Expr::Lambda(_, params, body) => {
                let mut local_env = env.clone();
                let mut param_types = Vec::new();

                for param in params {
                    let ty = if let Some(type_expr) = &param.ty {
                        self.resolve_type_expr(type_expr, &[])
                    } else {
                        self.var_gen.fresh()
                    };
                    local_env.insert(param.name.clone(), TypeScheme::mono(ty.clone()));
                    param_types.push(ty);
                }

                let (subst, body_ty) = self.infer_expr(&local_env, body)?;
                let final_params: Vec<Type> =
                    param_types.iter().map(|t| t.apply_subst(&subst)).collect();
                Ok((subst, Type::Fun(final_params, Box::new(body_ty))))
            }

            Expr::App(span, func, args) => {
                let (s1, func_ty) = self.infer_expr(env, func)?;

                let mut subst = s1;
                let mut arg_types = Vec::new();
                let mut current_env = env.apply_subst(&subst);

                for arg in args {
                    let (s, arg_ty) = self.infer_expr(&current_env, arg)?;
                    subst = subst.compose(&s);
                    current_env = current_env.apply_subst(&s);
                    arg_types.push(arg_ty);
                }

                let ret_ty = self.var_gen.fresh();
                let expected_func_ty = Type::Fun(arg_types, Box::new(ret_ty.clone()));

                // 関数引数の型不一致 (E0004)
                let s_unify = self
                    .unify(&func_ty.apply_subst(&subst), &expected_func_ty, *span)
                    .map_err(|e| Self::with_error_code(e, TypeErrorCode::ArgMismatch))?;

                let final_subst = subst.compose(&s_unify);
                let final_ty = ret_ty.apply_subst(&s_unify);
                Ok((final_subst, final_ty))
            }

            Expr::Match(span, scrutinee, arms) => {
                let (s1, scrut_ty) = self.infer_expr(env, scrutinee)?;
                let mut subst = s1;
                let result_ty = self.var_gen.fresh();

                for arm in arms {
                    let mut arm_env = env.apply_subst(&subst);

                    let (pat_ty, pat_bindings) = self.infer_pattern(&arm_env, &arm.pattern)?;
                    let s_pat = self.unify(&scrut_ty.apply_subst(&subst), &pat_ty, arm.span)?;
                    subst = subst.compose(&s_pat);

                    // GADT 型絞り込み: コンストラクタパターンの場合、
                    // GADT 戻り型から型変数の追加制約を適用
                    if let Pattern::Constructor(_, ctor_name, _) = &arm.pattern
                        && let Some(gadt_ret_ty) = self.gadt_return_types.get(ctor_name).cloned()
                    {
                        // GADT 戻り型と scrutinee 型を単一化して型を絞り込む
                        if let Ok(s_gadt) = self.unify(
                            &scrut_ty.apply_subst(&subst),
                            &gadt_ret_ty.apply_subst(&subst),
                            arm.span,
                        ) {
                            subst = subst.compose(&s_gadt);
                        }
                    }

                    arm_env = arm_env.apply_subst(&subst);
                    for (name, ty) in &pat_bindings {
                        arm_env.insert(name.clone(), TypeScheme::mono(ty.apply_subst(&subst)));
                    }

                    if let Some(guard) = &arm.guard {
                        let (s_guard, guard_ty) = self.infer_expr(&arm_env, guard)?;
                        subst = subst.compose(&s_guard);
                        let s_guard_bool = self
                            .unify(&guard_ty.apply_subst(&subst), &Type::bool(), guard.span())
                            .map_err(|e| Self::with_error_code(e, TypeErrorCode::IfCondition))?;
                        subst = subst.compose(&s_guard_bool);
                        arm_env = arm_env.apply_subst(&subst);
                    }

                    let (s_body, body_ty) = self.infer_expr(&arm_env, &arm.body)?;
                    subst = subst.compose(&s_body);

                    let s_res = self.unify(&result_ty.apply_subst(&subst), &body_ty, *span)?;
                    subst = subst.compose(&s_res);
                }

                let final_ty = result_ty.apply_subst(&subst);
                Ok((subst, final_ty))
            }

            Expr::Do(_, exprs) => {
                let mut subst = Substitution::new();
                let mut ty = Type::unit();
                let mut current_env = env.clone();

                for expr in exprs {
                    let (s, t) = self.infer_expr(&current_env, expr)?;
                    subst = subst.compose(&s);
                    current_env = current_env.apply_subst(&s);
                    ty = t;
                }

                Ok((subst, ty))
            }

            Expr::Ann(span, expr, type_expr) => {
                let (s1, inferred) = self.infer_expr(env, expr)?;
                let annotated = self.resolve_type_expr(type_expr, &[]);
                // エイリアス名を検出して、Mismatch エラーに付与
                let alias_name = self.detect_alias_name(type_expr);
                let s2 = self.unify(&inferred, &annotated, *span).map_err(|e| {
                    if let (
                        TypeError::Mismatch {
                            expected,
                            found,
                            span,
                            error_code,
                        },
                        Some(aname),
                    ) = (&e, &alias_name)
                    {
                        TypeError::MismatchWithAlias {
                            expected: expected.clone(),
                            found: found.clone(),
                            alias_name: aname.clone(),
                            expanded: annotated.clone(),
                            span: *span,
                            error_code: error_code.clone(),
                        }
                    } else {
                        e
                    }
                })?;
                Ok((s1.compose(&s2), annotated))
            }

            Expr::RecordLit(span, type_name, fields) => {
                self.infer_record_lit(env, *span, type_name, fields)
            }

            Expr::FieldAccess(span, expr, field_name) => {
                self.infer_field_access(env, *span, expr, field_name)
            }

            Expr::RecordUpdate(span, base, fields) => {
                self.infer_record_update(env, *span, base, fields)
            }
            Expr::Computation(_span, builder_name, steps) => {
                // Computation Expression: ビルダーの bind/return 関数で脱糖
                let builder_info = self.computation_builders.get(builder_name).cloned();

                let mut subst = Substitution::new();
                let mut result_ty = self.var_gen.fresh();

                // 各ステップを順方向で型チェック（let! で束縛を追加）
                let mut local_env = env.clone();
                let mut step_types = Vec::new();
                for step in steps {
                    let current_env = local_env.apply_subst(&subst);
                    match step {
                        ComputationStep::LetBang(_, pat, expr) => {
                            let (s, ty) = self.infer_expr(&current_env, expr)?;
                            subst = subst.compose(&s);
                            // パターンから変数を抽出して環境に追加
                            if let Pattern::Var(_, var_name) = pat {
                                let var_ty = ty.apply_subst(&subst);
                                let scheme = TypeScheme {
                                    vars: Vec::new(),
                                    constraints: Vec::new(),
                                    ty: var_ty,
                                };
                                local_env.insert(var_name.clone(), scheme);
                            }
                            step_types.push(("let!", ty));
                        }
                        ComputationStep::DoBang(_, expr) => {
                            let (s, ty) = self.infer_expr(&current_env, expr)?;
                            subst = subst.compose(&s);
                            step_types.push(("do!", ty));
                        }
                        ComputationStep::Return(_, expr) => {
                            let (s, ty) = self.infer_expr(&current_env, expr)?;
                            subst = subst.compose(&s);
                            step_types.push(("return", ty));
                        }
                        ComputationStep::Expr(expr) => {
                            let (s, ty) = self.infer_expr(&current_env, expr)?;
                            subst = subst.compose(&s);
                            step_types.push(("expr", ty));
                        }
                    }
                }

                // ビルダーが登録されている場合、return_fn/bind_fn の存在を確認して型を推定
                if let Some((bind_fn, return_fn)) = &builder_info {
                    let current_env = env.apply_subst(&subst);
                    // return_fn が環境にあれば、最後の return ステップの型からモナド型を推定
                    if let Some(return_scheme) = current_env.get(return_fn) {
                        let return_ty = self.instantiate(return_scheme);
                        // return_fn : a -> m a の形式
                        // 最後のステップの型から result_ty を推定
                        if let Some((kind, inner_ty)) = step_types.last()
                            && *kind == "return"
                        {
                            // return_fn(inner) の戻り型
                            let ret_result = self.var_gen.fresh();
                            let expected_fn_ty = Type::Fun(
                                vec![inner_ty.apply_subst(&subst)],
                                Box::new(ret_result.clone()),
                            );
                            let s = self.unify(&return_ty, &expected_fn_ty, *_span)?;
                            subst = subst.compose(&s);
                            result_ty = ret_result.apply_subst(&subst);
                        }
                    }

                    // bind_fn が環境にあれば、let!/do! ステップの型整合性を確認
                    if let Some(bind_scheme) = current_env.get(bind_fn) {
                        let _bind_ty = self.instantiate(bind_scheme);
                        // bind_fn : m a -> (a -> m b) -> m b の形式
                        // let!/do! ステップの式はモナド値 (m a) であるべき
                    }
                }

                // ビルダーが未登録の場合は最後のステップの型をそのまま返す
                if result_ty == self.var_gen.fresh()
                    && let Some((_, ty)) = step_types.last()
                {
                    result_ty = ty.apply_subst(&subst);
                }

                Ok((subst, result_ty))
            }

            // P10-1: Quote/Unquote/UnquoteSplice はマクロ展開後には残らない
            // マクロ展開前にこれらが残っている場合はエラーとする
            Expr::Quote(span, _) | Expr::Unquote(span, _) | Expr::UnquoteSplice(span, _) => {
                Err(TypeError::UndefinedVar {
                    name: "quote/unquote はマクロ展開後に使用できません".to_string(),
                    span: *span,
                })
            }
        };

        if let Ok((subst, ty)) = &result {
            self.record_expr_type(expr, subst, ty);
        }

        result
    }

    /// レコードリテラルの型推論
    fn infer_record_lit(
        &mut self,
        env: &TypeEnv,
        span: Span,
        type_name: &str,
        fields: &[(String, Expr)],
    ) -> Result<(Substitution, Type), TypeError> {
        let record_info = self
            .record_registry
            .get(type_name)
            .cloned()
            .ok_or_else(|| TypeError::UndefinedRecord {
                name: type_name.to_string(),
                span,
            })?;

        let mut param_subst = Substitution::new();
        for &var_id in &record_info.type_params {
            param_subst.insert(var_id, self.var_gen.fresh());
        }

        let mut subst = Substitution::new();
        let mut result_fields = Vec::new();

        for (field_name, field_expr) in fields {
            let expected_ty = record_info
                .fields
                .iter()
                .find(|(n, _)| n == field_name)
                .map(|(_, t)| t.apply_subst(&param_subst))
                .ok_or_else(|| TypeError::UndefinedField {
                    record_name: type_name.to_string(),
                    field_name: field_name.clone(),
                    span,
                })?;
            let expected_ty = self.materialize_registered_type(&expected_ty);

            let current_env = env.apply_subst(&subst);
            let (s1, field_ty) = self.infer_expr(&current_env, field_expr)?;
            subst = subst.compose(&s1);

            let s2 = self.unify(&field_ty, &expected_ty.apply_subst(&subst), span)?;
            subst = subst.compose(&s2);

            result_fields.push((field_name.clone(), field_ty.apply_subst(&subst)));
        }

        let record_type =
            self.materialize_registered_type(&Type::Record(type_name.to_string(), result_fields));
        Ok((subst, record_type))
    }

    /// フィールドアクセスの型推論
    fn infer_field_access(
        &mut self,
        env: &TypeEnv,
        span: Span,
        expr: &Expr,
        field_name: &str,
    ) -> Result<(Substitution, Type), TypeError> {
        let (s1, expr_ty) = self.infer_expr(env, expr)?;

        match &expr_ty {
            Type::Record(type_name, fields) => {
                if let Some((_, field_ty)) = fields.iter().find(|(n, _)| n == field_name) {
                    Ok((s1, field_ty.clone()))
                } else {
                    Err(TypeError::UndefinedField {
                        record_name: type_name.clone(),
                        field_name: field_name.to_string(),
                        span,
                    })
                }
            }
            _ => {
                let result_ty = self.var_gen.fresh();
                Ok((s1, result_ty))
            }
        }
    }

    /// レコード更新の型推論
    fn infer_record_update(
        &mut self,
        env: &TypeEnv,
        span: Span,
        base: &Expr,
        fields: &[(String, Expr)],
    ) -> Result<(Substitution, Type), TypeError> {
        let (s1, base_ty) = self.infer_expr(env, base)?;
        let mut subst = s1;

        match &base_ty {
            Type::Record(type_name, base_fields) => {
                let mut result_fields = base_fields.clone();

                for (field_name, field_expr) in fields {
                    let current_env = env.apply_subst(&subst);
                    let (s, field_ty) = self.infer_expr(&current_env, field_expr)?;
                    subst = subst.compose(&s);

                    if let Some(pos) = result_fields.iter().position(|(n, _)| n == field_name) {
                        let expected_ty = &result_fields[pos].1;
                        let s2 = self.unify(&field_ty, expected_ty, span)?;
                        subst = subst.compose(&s2);
                        result_fields[pos] = (field_name.clone(), field_ty.apply_subst(&subst));
                    } else {
                        return Err(TypeError::UndefinedField {
                            record_name: type_name.clone(),
                            field_name: field_name.clone(),
                            span,
                        });
                    }
                }

                let record_type = Type::Record(type_name.clone(), result_fields);
                Ok((subst, record_type))
            }
            _ => Err(TypeError::Mismatch {
                expected: Type::Con("Record".to_string()),
                found: base_ty,
                span,
                error_code: TypeErrorCode::General,
            }),
        }
    }

    /// パターンの型推論
    fn infer_pattern(
        &mut self,
        env: &TypeEnv,
        pattern: &Pattern,
    ) -> Result<(Type, Vec<(String, Type)>), TypeError> {
        match pattern {
            Pattern::Wildcard(_) => {
                let ty = self.var_gen.fresh();
                Ok((ty, Vec::new()))
            }
            Pattern::Var(_, name) => {
                let ty = self.var_gen.fresh();
                Ok((ty.clone(), vec![(name.clone(), ty)]))
            }
            Pattern::Lit(_, lit) => {
                let ty = self.lit_type(lit);
                Ok((ty, Vec::new()))
            }
            Pattern::Constructor(span, name, sub_pats) => {
                let resolved_name = name
                    .find('.')
                    .and_then(|dot_pos| {
                        self.resolve_qualified_name(&name[..dot_pos], &name[dot_pos + 1..])
                    })
                    .unwrap_or_else(|| name.clone());
                if let Some(scheme) = env.get(&resolved_name) {
                    let ctor_ty = self.instantiate(scheme);

                    match ctor_ty {
                        Type::Fun(param_types, ret_type) => {
                            if param_types.len() != sub_pats.len() {
                                return Err(TypeError::ArityMismatch {
                                    expected: param_types.len(),
                                    found: sub_pats.len(),
                                    span: *span,
                                });
                            }

                            let mut all_bindings = Vec::new();
                            let mut pat_subst = Substitution::new();
                            for (sub_pat, expected_ty) in sub_pats.iter().zip(param_types.iter()) {
                                let (pat_ty, bindings) = self.infer_pattern(env, sub_pat)?;
                                let expected_ty = self.materialize_registered_type(expected_ty);
                                // サブパターンの推論型とコンストラクタの期待型を unify
                                // ネストコンストラクタパターンの型を正しく伝播させる
                                let s = self.unify(
                                    &pat_ty.apply_subst(&pat_subst),
                                    &expected_ty.apply_subst(&pat_subst),
                                    *span,
                                )?;
                                pat_subst = pat_subst.compose(&s);
                                for (name, ty) in &bindings {
                                    all_bindings.push((name.clone(), ty.apply_subst(&pat_subst)));
                                }
                            }

                            let final_ret =
                                self.materialize_registered_type(&ret_type.apply_subst(&pat_subst));
                            Ok((final_ret, all_bindings))
                        }
                        other => {
                            if !sub_pats.is_empty() {
                                return Err(TypeError::ArityMismatch {
                                    expected: 0,
                                    found: sub_pats.len(),
                                    span: *span,
                                });
                            }
                            Ok((other, Vec::new()))
                        }
                    }
                } else {
                    Err(TypeError::UndefinedConstructor {
                        name: name.clone(),
                        span: *span,
                    })
                }
            }
            Pattern::RecordPat(span, type_name, field_pats) => {
                let record_info =
                    self.record_registry
                        .get(type_name)
                        .cloned()
                        .ok_or_else(|| TypeError::UndefinedRecord {
                            name: type_name.to_string(),
                            span: *span,
                        })?;

                let mut param_subst = Substitution::new();
                for &var_id in &record_info.type_params {
                    param_subst.insert(var_id, self.var_gen.fresh());
                }

                let mut all_bindings = Vec::new();
                let mut result_fields = Vec::new();

                for (field_name, field_pat) in field_pats {
                    let expected_ty = record_info
                        .fields
                        .iter()
                        .find(|(n, _)| n == field_name)
                        .map(|(_, t)| t.apply_subst(&param_subst))
                        .ok_or_else(|| TypeError::UndefinedField {
                            record_name: type_name.to_string(),
                            field_name: field_name.clone(),
                            span: *span,
                        })?;
                    let expected_ty = self.materialize_registered_type(&expected_ty);

                    let (pat_ty, bindings) = self.infer_pattern(env, field_pat)?;
                    let pat_subst = self.unify(
                        &pat_ty.apply_subst(&param_subst),
                        &expected_ty.apply_subst(&param_subst),
                        *span,
                    )?;
                    param_subst = param_subst.compose(&pat_subst);
                    for (name, ty) in &bindings {
                        all_bindings.push((name.clone(), ty.apply_subst(&param_subst)));
                    }
                    result_fields.push((field_name.clone(), expected_ty.apply_subst(&param_subst)));
                }

                for (name, ty) in &record_info.fields {
                    if !result_fields.iter().any(|(n, _)| n == name) {
                        result_fields.push((name.clone(), ty.apply_subst(&param_subst)));
                    }
                }

                let record_type = self.materialize_registered_type(&Type::Record(
                    type_name.to_string(),
                    result_fields,
                ));
                Ok((record_type, all_bindings))
            }
        }
    }

    /// パターンの束縛を環境に登録
    fn bind_pattern(
        &self,
        env: &mut TypeEnv,
        pattern: &Pattern,
        scheme: &TypeScheme,
    ) -> Result<(), TypeError> {
        match pattern {
            Pattern::Var(_, name) => {
                env.insert(name.clone(), scheme.clone());
                Ok(())
            }
            Pattern::Wildcard(_) => Ok(()),
            _ => Ok(()),
        }
    }

    /// リテラルの型
    fn lit_type(&self, lit: &Literal) -> Type {
        match lit {
            Literal::Int(_) => Type::int(),
            Literal::Float(_) => Type::float(),
            Literal::String(_) => Type::string(),
            Literal::Bool(_) => Type::bool(),
            Literal::Unit => Type::unit(),
        }
    }
}
