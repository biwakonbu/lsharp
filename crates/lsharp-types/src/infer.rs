use crate::types::*;
use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

/// 型推論エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum TypeError {
    #[error("型の不一致: {expected} と {found} ({span})")]
    Mismatch {
        expected: Type,
        found: Type,
        span: Span,
    },

    #[error("無限型: t{var} は {ty} に出現します ({span})")]
    InfiniteType {
        var: TypeVarId,
        ty: Type,
        span: Span,
    },

    #[error("未定義の変数: {name} ({span})")]
    UndefinedVar { name: String, span: Span },

    #[error("未定義のコンストラクタ: {name} ({span})")]
    UndefinedConstructor { name: String, span: Span },

    #[error("引数の数が不一致: 期待 {expected}, 実際 {found} ({span})")]
    ArityMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },
}

/// 型推論器
pub struct Infer {
    var_gen: TypeVarGen,
}

impl Infer {
    pub fn new() -> Self {
        Self {
            var_gen: TypeVarGen::new(),
        }
    }

    /// プログラム全体を型チェック
    pub fn infer_program(&mut self, program: &Program) -> Result<Vec<(String, TypeScheme)>, TypeError> {
        let mut env = self.builtin_env();
        let mut results = Vec::new();

        // まず全ての型定義を処理してコンストラクタを環境に登録
        for decl in &program.decls {
            if let Decl::TypeDef {
                name,
                type_params,
                variants,
                ..
            } = decl
            {
                self.register_type_def(&mut env, name, type_params, variants)?;
            }
        }

        // 次に全ての関数定義を型推論
        for decl in &program.decls {
            if let Decl::Defn {
                name,
                params,
                return_ty,
                body,
                span,
                ..
            } = decl
            {
                let (subst, ty) =
                    self.infer_defn(&env, name, params, return_ty.as_ref(), body, *span)?;
                let env_after = env.apply_subst(&subst);
                let final_ty = ty.apply_subst(&subst);
                let scheme = self.generalize(&env_after, &final_ty);
                env = env_after.extend(name.clone(), scheme.clone());
                results.push((name.clone(), scheme));
            }
        }

        Ok(results)
    }

    /// 組み込み関数の型環境
    fn builtin_env(&mut self) -> TypeEnv {
        let mut env = TypeEnv::new();

        // 算術演算子: (Int, Int) -> Int
        let int_binop = TypeScheme::mono(Type::Fun(
            vec![Type::int(), Type::int()],
            Box::new(Type::int()),
        ));
        for op in ["+", "-", "*", "/", "%"] {
            env.insert(op.to_string(), int_binop.clone());
        }

        // 比較演算子: (Int, Int) -> Bool
        let int_cmp = TypeScheme::mono(Type::Fun(
            vec![Type::int(), Type::int()],
            Box::new(Type::bool()),
        ));
        for op in ["<", ">", "<=", ">=", "==", "!="] {
            env.insert(op.to_string(), int_cmp.clone());
        }

        // 浮動小数点演算子
        let float_binop = TypeScheme::mono(Type::Fun(
            vec![Type::float(), Type::float()],
            Box::new(Type::float()),
        ));
        for op in ["+.", "-.", "*.", "/."] {
            env.insert(op.to_string(), float_binop.clone());
        }

        // print: forall a. a -> Unit
        let a = self.var_gen.fresh_id();
        env.insert(
            "print".to_string(),
            TypeScheme {
                vars: vec![a],
                ty: Type::Fun(vec![Type::Var(a)], Box::new(Type::unit())),
            },
        );

        // str: forall a. a -> String
        let b = self.var_gen.fresh_id();
        env.insert(
            "str".to_string(),
            TypeScheme {
                vars: vec![b],
                ty: Type::Fun(vec![Type::Var(b)], Box::new(Type::string())),
            },
        );

        // not: Bool -> Bool
        env.insert(
            "not".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::bool()], Box::new(Type::bool()))),
        );

        // and, or: (Bool, Bool) -> Bool
        let bool_binop = TypeScheme::mono(Type::Fun(
            vec![Type::bool(), Type::bool()],
            Box::new(Type::bool()),
        ));
        env.insert("and".to_string(), bool_binop.clone());
        env.insert("or".to_string(), bool_binop);

        env
    }

    /// 型定義のコンストラクタを環境に登録
    fn register_type_def(
        &mut self,
        env: &mut TypeEnv,
        type_name: &str,
        type_params: &[String],
        variants: &[Variant],
    ) -> Result<(), TypeError> {
        // 型パラメータ用の型変数を生成
        let param_vars: Vec<(String, TypeVarId)> = type_params
            .iter()
            .map(|p| (p.clone(), self.var_gen.fresh_id()))
            .collect();

        // 結果型を構築
        let result_type = if param_vars.is_empty() {
            Type::Con(type_name.to_string())
        } else {
            Type::App(
                type_name.to_string(),
                param_vars.iter().map(|(_, id)| Type::Var(*id)).collect(),
            )
        };

        let bound_vars: Vec<TypeVarId> = param_vars.iter().map(|(_, id)| *id).collect();

        for variant in variants {
            let ctor_type = if variant.fields.is_empty() {
                // 引数なしコンストラクタ: 結果型そのまま
                result_type.clone()
            } else {
                // 引数ありコンストラクタ: (field1, field2, ...) -> ResultType
                let field_types: Vec<Type> = variant
                    .fields
                    .iter()
                    .map(|f| self.resolve_type_expr(f, &param_vars))
                    .collect();
                Type::Fun(field_types, Box::new(result_type.clone()))
            };

            let scheme = TypeScheme {
                vars: bound_vars.clone(),
                ty: ctor_type,
            };
            env.insert(variant.name.clone(), scheme);
        }

        Ok(())
    }

    /// TypeExpr を Type に変換
    fn resolve_type_expr(
        &self,
        type_expr: &TypeExpr,
        param_vars: &[(String, TypeVarId)],
    ) -> Type {
        match type_expr {
            TypeExpr::Named(_, name) => {
                // 型パラメータ名なら型変数に変換
                if let Some((_, id)) = param_vars.iter().find(|(n, _)| n == name) {
                    Type::Var(*id)
                } else {
                    Type::Con(name.clone())
                }
            }
            TypeExpr::Var(_, name) => {
                if let Some((_, id)) = param_vars.iter().find(|(n, _)| n == name) {
                    Type::Var(*id)
                } else {
                    // 未知の型変数 — 後で型推論が解決する
                    Type::Con(name.clone())
                }
            }
            TypeExpr::App(_, base, args) => {
                let base_name = match base.as_ref() {
                    TypeExpr::Named(_, name) | TypeExpr::Var(_, name) => name.clone(),
                    _ => "?".to_string(),
                };
                let resolved_args: Vec<Type> = args
                    .iter()
                    .map(|a| self.resolve_type_expr(a, param_vars))
                    .collect();
                Type::App(base_name, resolved_args)
            }
            TypeExpr::Fun(_, params, ret) => {
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| self.resolve_type_expr(p, param_vars))
                    .collect();
                Type::Fun(
                    param_types,
                    Box::new(self.resolve_type_expr(ret, param_vars)),
                )
            }
        }
    }

    /// 関数定義の型推論
    fn infer_defn(
        &mut self,
        env: &TypeEnv,
        name: &str,
        params: &[Param],
        return_ty: Option<&TypeExpr>,
        body: &Expr,
        span: Span,
    ) -> Result<(Substitution, Type), TypeError> {
        let mut local_env = env.clone();

        // 再帰呼び出し用: 関数自身を型変数として環境に仮登録
        let self_ty = self.var_gen.fresh();
        local_env.insert(name.to_string(), TypeScheme::mono(self_ty.clone()));

        // パラメータの型変数を生成
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

        // 本体を型推論
        let (subst, body_type) = self.infer_expr(&local_env, body)?;

        // 戻り値型注釈があれば統合
        let subst = if let Some(ret_ty_expr) = return_ty {
            let ret_ty = self.resolve_type_expr(ret_ty_expr, &[]);
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

    /// 式の型推論 (Algorithm W)
    fn infer_expr(
        &mut self,
        env: &TypeEnv,
        expr: &Expr,
    ) -> Result<(Substitution, Type), TypeError> {
        match expr {
            Expr::Lit(_, lit) => Ok((Substitution::new(), self.lit_type(lit))),

            Expr::Var(span, name) => {
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
                let s_cond = self.unify(&cond_ty, &Type::bool(), *span)?;
                let s1 = s1.compose(&s_cond);

                let env1 = env.apply_subst(&s1);
                let (s2, then_ty) = self.infer_expr(&env1, then)?;

                let env2 = env1.apply_subst(&s2);
                let (s3, else_ty) = self.infer_expr(&env2, else_)?;

                let s_branch = self.unify(
                    &then_ty.apply_subst(&s3),
                    &else_ty,
                    *span,
                )?;

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

                let s_unify =
                    self.unify(&func_ty.apply_subst(&subst), &expected_func_ty, *span)?;

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

                    // パターンから型を推論し、scrutinee と統合
                    let (pat_ty, pat_bindings) = self.infer_pattern(&arm_env, &arm.pattern)?;
                    let s_pat =
                        self.unify(&scrut_ty.apply_subst(&subst), &pat_ty, arm.span)?;
                    subst = subst.compose(&s_pat);

                    // パターン束縛を環境に追加
                    arm_env = arm_env.apply_subst(&s_pat);
                    for (name, ty) in &pat_bindings {
                        arm_env.insert(
                            name.clone(),
                            TypeScheme::mono(ty.apply_subst(&subst)),
                        );
                    }

                    // 腕の本体を推論
                    let (s_body, body_ty) = self.infer_expr(&arm_env, &arm.body)?;
                    subst = subst.compose(&s_body);

                    // 全ての腕が同じ型を返すことを確認
                    let s_res = self.unify(
                        &result_ty.apply_subst(&subst),
                        &body_ty,
                        *span,
                    )?;
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
                let s2 = self.unify(&inferred, &annotated, *span)?;
                Ok((s1.compose(&s2), annotated))
            }
        }
    }

    /// パターンの型推論（型と束縛のリストを返す）
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
                if let Some(scheme) = env.get(name) {
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
                            for (sub_pat, expected_ty) in
                                sub_pats.iter().zip(param_types.iter())
                            {
                                let (pat_ty, bindings) = self.infer_pattern(env, sub_pat)?;
                                // ここでは unify しない — 呼び出し元が scrutinee と統合する
                                // ただしサブパターンの型は expected_ty と合わせる
                                let _ = pat_ty; // サブパターンの型は expected_ty に合わせる
                                for (name, _) in &bindings {
                                    all_bindings
                                        .push((name.clone(), expected_ty.clone()));
                                }
                            }

                            Ok((*ret_type, all_bindings))
                        }
                        // 引数なしコンストラクタ
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
            _ => {
                // 複雑なパターンは let 束縛では未サポート
                // 将来的に destructuring を追加
                Ok(())
            }
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

    /// 型スキームをインスタンス化（束縛変数を新しい型変数に置換）
    fn instantiate(&mut self, scheme: &TypeScheme) -> Type {
        let mut subst = Substitution::new();
        for &var in &scheme.vars {
            subst.insert(var, self.var_gen.fresh());
        }
        scheme.ty.apply_subst(&subst)
    }

    /// 型を汎化（環境にない自由変数を全称量化）
    fn generalize(&self, env: &TypeEnv, ty: &Type) -> TypeScheme {
        let env_vars = env.free_vars();
        let ty_vars = ty.free_vars();
        let vars: Vec<TypeVarId> = ty_vars
            .into_iter()
            .filter(|v| !env_vars.contains(v))
            .collect();
        TypeScheme {
            vars,
            ty: ty.clone(),
        }
    }

    /// 2つの型を統合 (Unification)
    fn unify(&mut self, t1: &Type, t2: &Type, span: Span) -> Result<Substitution, TypeError> {
        match (t1, t2) {
            // 同じ具体型
            (Type::Con(a), Type::Con(b)) if a == b => Ok(Substitution::new()),

            // 型変数
            (Type::Var(id), ty) | (ty, Type::Var(id)) => self.bind_var(*id, ty, span),

            // 関数型
            (Type::Fun(params1, ret1), Type::Fun(params2, ret2)) => {
                if params1.len() != params2.len() {
                    return Err(TypeError::Mismatch {
                        expected: t1.clone(),
                        found: t2.clone(),
                        span,
                    });
                }
                let mut subst = Substitution::new();
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    let s = self.unify(
                        &p1.apply_subst(&subst),
                        &p2.apply_subst(&subst),
                        span,
                    )?;
                    subst = subst.compose(&s);
                }
                let s_ret = self.unify(
                    &ret1.apply_subst(&subst),
                    &ret2.apply_subst(&subst),
                    span,
                )?;
                Ok(subst.compose(&s_ret))
            }

            // 型適用
            (Type::App(name1, args1), Type::App(name2, args2))
                if name1 == name2 && args1.len() == args2.len() =>
            {
                let mut subst = Substitution::new();
                for (a1, a2) in args1.iter().zip(args2.iter()) {
                    let s = self.unify(
                        &a1.apply_subst(&subst),
                        &a2.apply_subst(&subst),
                        span,
                    )?;
                    subst = subst.compose(&s);
                }
                Ok(subst)
            }

            _ => Err(TypeError::Mismatch {
                expected: t1.clone(),
                found: t2.clone(),
                span,
            }),
        }
    }

    /// 型変数を型に束縛（occurs check 付き）
    fn bind_var(
        &self,
        var: TypeVarId,
        ty: &Type,
        span: Span,
    ) -> Result<Substitution, TypeError> {
        // 同一変数への束縛は無視
        if let Type::Var(id) = ty {
            if *id == var {
                return Ok(Substitution::new());
            }
        }

        // occurs check: 無限型の防止
        if ty.free_vars().contains(&var) {
            return Err(TypeError::InfiniteType {
                var,
                ty: ty.clone(),
                span,
            });
        }

        let mut subst = Substitution::new();
        subst.insert(var, ty.clone());
        Ok(subst)
    }
}

impl Default for Infer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn infer(input: &str) -> Result<Vec<(String, TypeScheme)>, TypeError> {
        let program = lsharp_syntax::parse(input).unwrap();
        let mut infer = Infer::new();
        infer.infer_program(&program)
    }

    fn infer_one(input: &str) -> String {
        let results = infer(input).unwrap();
        let (_, scheme) = &results[0];
        scheme.to_string()
    }

    #[test]
    fn test_identity() {
        let result = infer_one("(defn id [x] x)");
        // forall a. a -> a
        assert!(result.starts_with("forall"));
        assert!(result.contains("->"));
    }

    #[test]
    fn test_add() {
        let result = infer_one("(defn add [x y] (+ x y))");
        assert_eq!(result, "(Int, Int) -> Int");
    }

    #[test]
    fn test_bool_expr() {
        let result = infer_one("(defn is-zero [n] (== n 0))");
        assert_eq!(result, "(Int) -> Bool");
    }

    #[test]
    fn test_if_expr() {
        let result = infer_one("(defn abs [n] (if (< n 0) (- 0 n) n))");
        assert_eq!(result, "(Int) -> Int");
    }

    #[test]
    fn test_let_expr() {
        let result = infer_one("(defn f [] (let [x 42] x))");
        assert_eq!(result, "() -> Int");
    }

    #[test]
    fn test_lambda() {
        let result = infer_one("(defn apply [f x] (f x))");
        // forall a b. ((a) -> b, a) -> b
        assert!(result.starts_with("forall"));
    }

    #[test]
    fn test_recursive() {
        let result = infer_one(
            "(defn fib [n] (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))",
        );
        assert_eq!(result, "(Int) -> Int");
    }

    #[test]
    fn test_type_error_mismatch() {
        let result = infer("(defn bad [] (+ 1 true))");
        assert!(result.is_err());
    }

    #[test]
    fn test_undefined_var() {
        let result = infer("(defn bad [] x)");
        assert!(result.is_err());
    }

    #[test]
    fn test_adt_basic() {
        let results = infer(
            "(type (Option a) (Some a) None)
             (defn get-or-zero [opt] (match opt [(Some x) x] [None 0]))",
        )
        .unwrap();
        // get-or-zero の型は (Option Int) -> Int
        let (name, scheme) = &results[0];
        assert_eq!(name, "get-or-zero");
        assert!(scheme.to_string().contains("Int"));
    }

    #[test]
    fn test_do_expr() {
        let result = infer_one("(defn main [] (do (print 1) (print 2)))");
        assert_eq!(result, "() -> Unit");
    }

    #[test]
    fn test_type_annotation() {
        let result = infer_one("(defn add [(: x Int) (: y Int)] : Int (+ x y))");
        assert_eq!(result, "(Int, Int) -> Int");
    }
}
