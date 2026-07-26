#![allow(clippy::result_large_err, clippy::type_complexity)]

use std::collections::{HashMap, HashSet};

use crate::types::*;
use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

mod builtin_env;
mod error;
mod expr;
mod generalize;
mod unify;
pub use error::{TypeError, TypeErrorCode};

#[cfg(test)]
include!("infer_tests.rs");

/// Kind の互換性チェック
/// トレイトの Kind と実装型の Kind が一致するかを判定
fn kinds_compatible(trait_kind: &Kind, type_kind: &Kind) -> bool {
    match (trait_kind, type_kind) {
        (Kind::Star, Kind::Star) => true,
        (Kind::Arrow(_, _), Kind::Arrow(_, _)) => trait_kind == type_kind,
        _ => false,
    }
}

/// モジュール環境（モジュールごとの型環境・可視性情報）
#[derive(Debug, Clone, Default)]
pub struct ModuleEnv {
    /// モジュール名
    pub name: Option<String>,
    /// エクスポートされるシンボル（None = 全て公開）
    pub exports: Option<Vec<String>>,
    /// 非公開シンボル
    pub privates: Vec<String>,
    /// インポートされたモジュール
    pub imports: Vec<ModuleImport>,
}

/// モジュールインポート情報
#[derive(Debug, Clone)]
pub struct ModuleImport {
    /// インポート元モジュール名
    pub module: String,
    /// エイリアス名
    pub alias: Option<String>,
    /// 選択的インポート（None = 全て）
    pub only: Option<Vec<String>>,
    /// open インポート（名前空間なしで参照可能）
    pub open: bool,
}

/// 型推論器
/// 制約解決待ちのトレイト制約
#[derive(Debug, Clone)]
struct PendingConstraint {
    trait_name: String,
    type_var: TypeVarId,
    span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExprTypeKey {
    pub scope: String,
    pub span: Span,
}

impl ExprTypeKey {
    pub fn new(scope: impl Into<String>, span: Span) -> Self {
        Self {
            scope: scope.into(),
            span,
        }
    }
}

pub struct Infer {
    var_gen: TypeVarGen,
    /// レコード型の登録情報
    record_registry: HashMap<String, RecordInfo>,
    /// 型エイリアスの登録情報
    type_aliases: HashMap<String, (Vec<String>, Type)>,
    /// トレイト定義
    trait_registry: HashMap<String, TraitInfo>,
    /// トレイト実装
    impl_registry: Vec<ImplInfo>,
    /// 制約付き型の登録情報
    constrained_types: HashMap<String, ConstrainedTypeInfo>,
    /// モジュール環境
    pub module_env: ModuleEnv,
    /// トレイトのデフォルト実装キャッシュ
    /// (trait_name, method_name) -> (params, return_ty, body)
    default_impls: HashMap<(String, String), (Vec<Param>, Option<TypeExpr>, Expr)>,
    /// 制約解決待ちリスト
    pending_constraints: Vec<PendingConstraint>,
    /// 種環境（型名 -> Kind）
    pub kind_env: HashMap<String, Kind>,
    /// グローバル代入（制約チェック用）
    global_subst: Substitution,
    /// GADT バリアントの戻り型情報（コンストラクタ名 -> 戻り型）
    gadt_return_types: HashMap<String, Type>,
    /// Computation Builder 登録情報（ビルダー名 -> (bind関数名, return関数名)）
    computation_builders: HashMap<String, (String, String)>,
    /// 外部モジュールから注入された型環境
    external_types: HashMap<String, TypeScheme>,
    /// top-level scope ごとの式型結果
    expr_type_results: HashMap<ExprTypeKey, Type>,
    /// 同じ key に異なる型が衝突した曖昧な式キー
    ambiguous_expr_type_keys: HashSet<ExprTypeKey>,
    /// 現在推論中の式スコープ
    current_expr_scope: Option<String>,
}

struct DefnInferenceInput<'a> {
    env: &'a TypeEnv,
    name: &'a str,
    expr_scope: &'a str,
    params: &'a [Param],
    return_ty: Option<&'a TypeExpr>,
    body: &'a Expr,
    span: Span,
}

impl Infer {
    pub fn new() -> Self {
        Self {
            var_gen: TypeVarGen::new(),
            record_registry: HashMap::new(),
            type_aliases: HashMap::new(),
            trait_registry: HashMap::new(),
            impl_registry: Vec::new(),
            constrained_types: HashMap::new(),
            module_env: ModuleEnv::default(),
            default_impls: HashMap::new(),
            pending_constraints: Vec::new(),
            kind_env: HashMap::new(),
            global_subst: Substitution::new(),
            gadt_return_types: HashMap::new(),
            computation_builders: HashMap::new(),
            external_types: HashMap::new(),
            expr_type_results: HashMap::new(),
            ambiguous_expr_type_keys: HashSet::new(),
            current_expr_scope: None,
        }
    }

    pub fn expr_type_results_snapshot(&self) -> HashMap<ExprTypeKey, Type> {
        self.expr_type_results.clone()
    }

    /// 外部モジュールの型環境を注入
    ///
    /// クロスモジュールコンパイル時に、依存先モジュールの型推論結果を
    /// 現在のモジュールの初期環境に追加する。
    pub fn inject_external_types(&mut self, types: &[(String, TypeScheme)]) {
        for (name, scheme) in types {
            self.external_types.insert(name.clone(), scheme.clone());
        }
    }

    /// import 可視性に従って外部モジュールの型環境を注入
    pub fn inject_external_types_for_import(
        &mut self,
        module: &str,
        only: Option<&[String]>,
        hidden: &HashSet<String>,
        types: &[(String, TypeScheme)],
    ) {
        for (name, scheme) in types {
            if Self::is_type_visible_from_import(module, name, only, hidden) {
                self.external_types.insert(name.clone(), scheme.clone());
            }
        }
    }

    /// 現在の external_types のスナップショットを Vec 形式で返す
    ///
    /// 再帰的 import 解決時に、解決済みの型を推移的に注入するために使用する。
    pub fn external_types_snapshot(&self) -> Vec<(String, TypeScheme)> {
        self.external_types
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    fn is_type_visible_from_import(
        module: &str,
        name: &str,
        only: Option<&[String]>,
        hidden: &HashSet<String>,
    ) -> bool {
        let module_qualified = format!("{module}.{name}");
        if hidden.contains(name) || hidden.contains(&module_qualified) {
            return false;
        }

        let visible_name = name.strip_prefix(&format!("{module}.")).unwrap_or(name);

        match only {
            Some(only) => only
                .iter()
                .any(|symbol| symbol == visible_name || symbol == name),
            None => true,
        }
    }

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

    /// 型定義のコンストラクタを環境に登録
    fn register_type_def(
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
    fn register_record_def(
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
    fn register_type_alias(
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
    fn register_type_constrained(
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
    fn register_trait_def(
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
    fn register_impl_def(
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

    /// 制約解決待ちの全制約をチェック
    ///
    /// 型推論完了後に呼ばれ、制約に含まれる型変数が具体型に解決されている場合、
    /// 対応する impl が登録されているか確認する。
    /// 型変数がまだ未解決（多相のまま）の場合はスキップ（多相関数ではOK）。
    fn check_pending_constraints(&self, subst: &Substitution) -> Result<(), TypeError> {
        // 型を名前文字列に変換するヘルパー
        fn type_to_name_str(ty: &Type) -> Option<String> {
            match ty {
                Type::Con(name) => Some(name.clone()),
                Type::App(name, _) => Some(name.clone()),
                _ => None,
            }
        }

        for constraint in &self.pending_constraints {
            let resolved_type = Type::Var(constraint.type_var).apply_subst(subst);

            // 型変数がまだ未解決の場合はスキップ（多相関数では制約は保持されるだけ）
            if matches!(resolved_type, Type::Var(_)) {
                continue;
            }

            // 具体型に解決されている場合、impl が存在するか確認
            let type_name = type_to_name_str(&resolved_type);
            if let Some(ref name) = type_name {
                let has_impl = self.impl_registry.iter().any(|info| {
                    info.trait_name == constraint.trait_name && info.type_name == *name
                });
                if !has_impl {
                    return Err(TypeError::MissingImpl {
                        trait_name: constraint.trait_name.clone(),
                        type_name: name.clone(),
                        span: constraint.span,
                    });
                }
            }
        }
        Ok(())
    }

    /// TypeExpr を Type に変換
    pub(super) fn resolve_type_expr(
        &self,
        type_expr: &TypeExpr,
        param_vars: &[(String, TypeVarId)],
    ) -> Type {
        match type_expr {
            TypeExpr::Named(_, name) => {
                if let Some((_, id)) = param_vars.iter().find(|(n, _)| n == name) {
                    Type::Var(*id)
                } else if let Some((_params, target)) = self.type_aliases.get(name) {
                    // 型エイリアスを透過的に展開
                    target.clone()
                } else if let Some(record_info) = self.record_registry.get(name)
                    && record_info.type_params.is_empty()
                {
                    Type::Record(record_info.name.clone(), record_info.fields.clone())
                } else {
                    Type::Con(name.clone())
                }
            }
            TypeExpr::Var(_, name) => {
                if let Some((_, id)) = param_vars.iter().find(|(n, _)| n == name) {
                    Type::Var(*id)
                } else if let Some((_params, target)) = self.type_aliases.get(name) {
                    target.clone()
                } else if let Some(record_info) = self.record_registry.get(name)
                    && record_info.type_params.is_empty()
                {
                    Type::Record(record_info.name.clone(), record_info.fields.clone())
                } else {
                    Type::Con(name.clone())
                }
            }
            TypeExpr::App(_, base, args) => {
                let base_name = match base.as_ref() {
                    TypeExpr::Named(_, name) | TypeExpr::Var(_, name) => name.clone(),
                    _ => "?".to_string(),
                };

                // パラメトリック型エイリアスの展開
                if let Some((alias_params, target)) = self.type_aliases.get(&base_name).cloned()
                    && alias_params.len() == args.len()
                {
                    // エイリアスパラメータの型変数を特定
                    // target 内の Type::Var(id) を引数の型で置換
                    let resolved_args: Vec<Type> = args
                        .iter()
                        .map(|a| self.resolve_type_expr(a, param_vars))
                        .collect();

                    // target の自由型変数を収集して、パラメータ順に対応付け
                    let free_vars = target.free_vars();
                    let mut subst = Substitution::new();
                    // free_vars は sorted + dedup 済み、alias_params と同じ順序のはず
                    for (i, &var_id) in free_vars.iter().enumerate() {
                        if i < resolved_args.len() {
                            subst.insert(var_id, resolved_args[i].clone());
                        }
                    }
                    return target.apply_subst(&subst);
                }

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
            TypeExpr::Record(_, fields) => {
                let resolved_fields: Vec<(String, Type)> = fields
                    .iter()
                    .map(|(name, ty)| (name.clone(), self.resolve_type_expr(ty, param_vars)))
                    .collect();
                Type::Record("_anon".to_string(), resolved_fields)
            }
        }
    }

    /// モジュールエイリアス経由の完全修飾名解決
    /// prefix がモジュールエイリアスまたはモジュール名と一致する場合、
    /// 完全修飾名（モジュール名.関数名）を返す
    pub(super) fn resolve_qualified_name(&self, prefix: &str, suffix: &str) -> Option<String> {
        for imp in &self.module_env.imports {
            // エイリアス名と一致
            if imp.alias.as_deref() == Some(prefix) {
                // 選択的インポートの場合、指定されたシンボルのみ許可
                if let Some(ref only) = imp.only
                    && !only.contains(&suffix.to_string())
                {
                    continue;
                }
                return Some(format!("{}.{}", imp.module, suffix));
            }
            // モジュール名と直接一致
            if imp.module == prefix {
                if let Some(ref only) = imp.only
                    && !only.contains(&suffix.to_string())
                {
                    continue;
                }
                return Some(format!("{}.{}", imp.module, suffix));
            }
        }
        None
    }

    /// TypeExpr がエイリアス名かどうか検出
    pub(super) fn detect_alias_name(&self, type_expr: &TypeExpr) -> Option<String> {
        match type_expr {
            TypeExpr::Named(_, name) | TypeExpr::Var(_, name) => {
                if self.type_aliases.contains_key(name) {
                    Some(name.clone())
                } else {
                    None
                }
            }
            TypeExpr::App(_, base, _) => match base.as_ref() {
                TypeExpr::Named(_, name) | TypeExpr::Var(_, name) => {
                    if self.type_aliases.contains_key(name) {
                        Some(name.clone())
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    /// TypeError のエラーコードを差し替えるヘルパー
    fn with_error_code(err: TypeError, code: TypeErrorCode) -> TypeError {
        match err {
            TypeError::Mismatch {
                expected,
                found,
                span,
                ..
            } => TypeError::Mismatch {
                expected,
                found,
                span,
                error_code: code,
            },
            TypeError::MismatchWithAlias {
                expected,
                found,
                alias_name,
                expanded,
                span,
                ..
            } => TypeError::MismatchWithAlias {
                expected,
                found,
                alias_name,
                expanded,
                span,
                error_code: code,
            },
            other => other,
        }
    }

    pub(super) fn record_expr_type(&mut self, expr: &Expr, subst: &Substitution, ty: &Type) {
        let Some(scope) = self.current_expr_scope.clone() else {
            return;
        };
        let key = ExprTypeKey::new(scope, expr.span());
        if self.ambiguous_expr_type_keys.contains(&key) {
            return;
        }
        let final_ty = ty.apply_subst(subst);
        match self.expr_type_results.get(&key) {
            Some(existing) if existing != &final_ty => {
                self.expr_type_results.remove(&key);
                self.ambiguous_expr_type_keys.insert(key);
            }
            Some(_) => {}
            None => {
                self.expr_type_results.insert(key, final_ty);
            }
        }
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
    fn infer_defn(
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

    /// 型スキームをインスタンス化
    pub(super) fn instantiate(&mut self, scheme: &TypeScheme) -> Type {
        let mut subst = Substitution::new();
        for &var in &scheme.vars {
            subst.insert(var, self.var_gen.fresh());
        }
        // 制約があれば pending に追加（型変数も新しいものに置換）
        for constraint in &scheme.constraints {
            let new_var = if let Some(Type::Var(v)) = subst.get(constraint.type_var) {
                *v
            } else {
                constraint.type_var
            };
            self.pending_constraints.push(PendingConstraint {
                trait_name: constraint.trait_name.clone(),
                type_var: new_var,
                span: Span::new(0, 0), // instantiate 時はスパン情報がない
            });
        }
        scheme.ty.apply_subst(&subst)
    }
}

impl Default for Infer {
    fn default() -> Self {
        Self::new()
    }
}
