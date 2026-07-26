#![allow(clippy::result_large_err, clippy::type_complexity)]

use std::collections::{HashMap, HashSet};

use crate::types::*;
use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

mod builtin_env;
mod decl;
mod error;
mod expr;
mod generalize;
mod registration;
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

    /// 制約解決待ちの全制約をチェック
    ///
    /// 型推論完了後に呼ばれ、制約に含まれる型変数が具体型に解決されている場合、
    /// 対応する impl が登録されているか確認する。
    /// 型変数がまだ未解決（多相のまま）の場合はスキップ（多相関数ではOK）。
    pub(super) fn check_pending_constraints(&self, subst: &Substitution) -> Result<(), TypeError> {
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
