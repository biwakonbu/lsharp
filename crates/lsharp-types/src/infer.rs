#![allow(clippy::result_large_err, clippy::type_complexity)]

use std::collections::{HashMap, HashSet};

use crate::types::*;
use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

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

/// 型推論エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum TypeError {
    #[error("[{error_code}] 型の不一致: expected {expected}, found {found} ({span})")]
    Mismatch {
        expected: Type,
        found: Type,
        span: Span,
        /// エラーコード (E0002=if条件, E0003=分岐不一致, E0004=引数不一致, E0006=一般)
        error_code: TypeErrorCode,
    },

    #[error("[E0005] 無限型 (infinite type): t{var} は {ty} に出現します ({span})")]
    InfiniteType {
        var: TypeVarId,
        ty: Type,
        span: Span,
    },

    #[error("[E0001] 未定義の変数 (undefined): {name} ({span})")]
    UndefinedVar { name: String, span: Span },

    #[error("[E0001] 未定義のコンストラクタ: {name} ({span})")]
    UndefinedConstructor { name: String, span: Span },

    #[error("[E0006] 引数の数が不一致: 期待 {expected}, 実際 {found} ({span})")]
    ArityMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("[E0001] 未定義のレコード型: {name} ({span})")]
    UndefinedRecord { name: String, span: Span },

    #[error("[E0001] 未定義のフィールド: {record_name}.{field_name} ({span})")]
    UndefinedField {
        record_name: String,
        field_name: String,
        span: Span,
    },

    #[error("[E0006] 再帰的な型エイリアス: {name} ({span})")]
    RecursiveAlias { name: String, span: Span },

    #[error("[E0001] 未定義の型エイリアス: {name} ({span})")]
    UndefinedAlias { name: String, span: Span },

    #[error("[E0001] 未定義のトレイト: {name} ({span})")]
    UndefinedTrait { name: String, span: Span },

    #[error("[E0006] トレイト {trait_name} の実装が見つかりません: {type_name} ({span})")]
    MissingImpl {
        trait_name: String,
        type_name: String,
        span: Span,
    },

    #[error(
        "[{error_code}] 型の不一致 (mismatch): expected {expected}, found {found} (エイリアス '{alias_name}' は {expanded} に展開) ({span})"
    )]
    MismatchWithAlias {
        expected: Type,
        found: Type,
        alias_name: String,
        expanded: Type,
        span: Span,
        error_code: TypeErrorCode,
    },

    #[error(
        "[E0006] Kind の不一致: {type_name} は {actual_kind} ですが、トレイト {trait_name} は {expected_kind} を要求します ({span})"
    )]
    KindMismatch {
        type_name: String,
        trait_name: String,
        expected_kind: Kind,
        actual_kind: Kind,
        span: Span,
    },
}

impl TypeError {
    /// 型推論エラーを公開診断コードへ変換する。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Mismatch { error_code, .. } | Self::MismatchWithAlias { error_code, .. } => {
                match error_code {
                    TypeErrorCode::IfCondition
                    | TypeErrorCode::IfBranch
                    | TypeErrorCode::General => "LS1002",
                    TypeErrorCode::ArgMismatch => "LS1004",
                }
            }
            Self::InfiniteType { .. } => "LS1003",
            Self::UndefinedVar { .. } => "LS1001",
            Self::UndefinedConstructor { .. } => "LS1005",
            Self::ArityMismatch { .. } => "LS1004",
            Self::UndefinedRecord { .. } => "LS1006",
            Self::UndefinedField { .. } => "LS1007",
            Self::RecursiveAlias { .. } => "LS1008",
            Self::UndefinedAlias { .. } => "LS1009",
            Self::UndefinedTrait { .. } => "LS1010",
            Self::MissingImpl { .. } => "LS1011",
            Self::KindMismatch { .. } => "LS1013",
        }
    }

    pub fn span(&self) -> Option<Span> {
        Some(match self {
            Self::Mismatch { span, .. }
            | Self::InfiniteType { span, .. }
            | Self::UndefinedVar { span, .. }
            | Self::UndefinedConstructor { span, .. }
            | Self::ArityMismatch { span, .. }
            | Self::UndefinedRecord { span, .. }
            | Self::UndefinedField { span, .. }
            | Self::RecursiveAlias { span, .. }
            | Self::UndefinedAlias { span, .. }
            | Self::UndefinedTrait { span, .. }
            | Self::MissingImpl { span, .. }
            | Self::MismatchWithAlias { span, .. }
            | Self::KindMismatch { span, .. } => *span,
        })
    }
}

/// 型エラーコード (E0001 形式)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeErrorCode {
    /// E0002: if 条件が Bool でない
    IfCondition,
    /// E0003: if 分岐の型不一致
    IfBranch,
    /// E0004: 関数引数の型不一致
    ArgMismatch,
    /// E0006: 一般的な型不一致
    General,
}

impl std::fmt::Display for TypeErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeErrorCode::IfCondition => write!(f, "E0002"),
            TypeErrorCode::IfBranch => write!(f, "E0003"),
            TypeErrorCode::ArgMismatch => write!(f, "E0004"),
            TypeErrorCode::General => write!(f, "E0006"),
        }
    }
}

impl std::error::Error for TypeErrorCode {}

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
        for op in ["<", ">", "<=", ">=", "==", "!=", "="] {
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
                constraints: Vec::new(),
                ty: Type::Fun(vec![Type::Var(a)], Box::new(Type::unit())),
            },
        );

        // __alloc: Int -> Int (メモリアロケーション: サイズ -> アドレス)
        env.insert(
            "__alloc".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::int()], Box::new(Type::int()))),
        );

        // string-length: String -> Int (文字列のバイト長を返す)
        env.insert(
            "string-length".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::string()], Box::new(Type::int()))),
        );

        // string-concat: String -> String -> String (文字列結合)
        env.insert(
            "string-concat".to_string(),
            TypeScheme::mono(Type::Fun(
                vec![Type::string(), Type::string()],
                Box::new(Type::string()),
            )),
        );

        // string-eq: String -> String -> Bool (文字列等価比較)
        env.insert(
            "string-eq".to_string(),
            TypeScheme::mono(Type::Fun(
                vec![Type::string(), Type::string()],
                Box::new(Type::bool()),
            )),
        );

        // print-string: String -> Unit (文字列を出力)
        env.insert(
            "print-string".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::string()], Box::new(Type::unit()))),
        );

        // string-char-at: String -> Int -> Int (文字列のインデックス位置のバイト値を返す)
        env.insert(
            "string-char-at".to_string(),
            TypeScheme::mono(Type::Fun(
                vec![Type::string(), Type::int()],
                Box::new(Type::int()),
            )),
        );

        // substring: String -> Int -> Int -> String (部分文字列を返す)
        env.insert(
            "substring".to_string(),
            TypeScheme::mono(Type::Fun(
                vec![Type::string(), Type::int(), Type::int()],
                Box::new(Type::string()),
            )),
        );

        // int-to-string: Int -> String (整数を文字列に変換)
        env.insert(
            "int-to-string".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::int()], Box::new(Type::string()))),
        );

        // proc-exit: Int -> Unit (プロセス終了)
        env.insert(
            "proc-exit".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::int()], Box::new(Type::unit()))),
        );

        // ref-new: forall a. a -> Ref a (Ref Cell 作成: 値 -> ヒープハンドル)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "ref-new".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::Var(a)], Box::new(Type::ref_of(Type::Var(a)))),
                },
            );
        }

        // ref-get: forall a. Ref a -> a (Ref Cell 読み出し: ヒープハンドル -> 値)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "ref-get".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::ref_of(Type::Var(a))], Box::new(Type::Var(a))),
                },
            );
        }

        // ref-set: forall a. (Ref a, a) -> Unit (Ref Cell 書き込み: ヒープハンドル, 値 -> Unit)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "ref-set".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(
                        vec![Type::ref_of(Type::Var(a)), Type::Var(a)],
                        Box::new(Type::unit()),
                    ),
                },
            );
        }

        // === Vector (可変長配列) ビルトイン ===

        // vector-new: Int -> Vector (capacity を指定して空ベクタを作成)
        env.insert(
            "vector-new".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::int()], Box::new(Type::vector()))),
        );

        // vector-length: Vector -> Int (ベクタの現在の長さを返す)
        env.insert(
            "vector-length".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::vector()], Box::new(Type::int()))),
        );

        // vector-get: forall a. (Vector, Int) -> a (インデックス指定で要素を取得)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "vector-get".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::vector(), Type::int()], Box::new(Type::Var(a))),
                },
            );
        }

        // vector-set: forall a. (Vector, Int, a) -> Vector (インデックス指定で要素を上書き)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "vector-set".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(
                        vec![Type::vector(), Type::int(), Type::Var(a)],
                        Box::new(Type::vector()),
                    ),
                },
            );
        }

        // vector-push: forall a. (Vector, a) -> Vector (要素を末尾に追加)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "vector-push".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::vector(), Type::Var(a)], Box::new(Type::vector())),
                },
            );
        }

        // === HashMap ビルトイン ===

        // map-new: () -> Map (デフォルト容量で空のハッシュマップを作成)
        env.insert(
            "map-new".to_string(),
            TypeScheme::mono(Type::Fun(vec![], Box::new(Type::map()))),
        );

        // map-size: Map -> Int (エントリ数を返す)
        env.insert(
            "map-size".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::map()], Box::new(Type::int()))),
        );

        // map-insert: forall k a. (Map, k, a) -> Map (キーと値を挿入)
        {
            let k = self.var_gen.fresh_id();
            let a = self.var_gen.fresh_id();
            env.insert(
                "map-insert".to_string(),
                TypeScheme {
                    vars: vec![k, a],
                    constraints: Vec::new(),
                    ty: Type::Fun(
                        vec![Type::map(), Type::Var(k), Type::Var(a)],
                        Box::new(Type::map()),
                    ),
                },
            );
        }

        // map-get: forall k a. (Map, k) -> a (キーで値を取得、未存在時は 0)
        {
            let k = self.var_gen.fresh_id();
            let a = self.var_gen.fresh_id();
            env.insert(
                "map-get".to_string(),
                TypeScheme {
                    vars: vec![k, a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::map(), Type::Var(k)], Box::new(Type::Var(a))),
                },
            );
        }

        // map-contains?: forall k. (Map, k) -> Bool (キーの存在チェック)
        {
            let k = self.var_gen.fresh_id();
            env.insert(
                "map-contains?".to_string(),
                TypeScheme {
                    vars: vec![k],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::map(), Type::Var(k)], Box::new(Type::int())),
                },
            );
        }

        // map-remove: forall k. (Map, k) -> Map (キーを削除)
        {
            let k = self.var_gen.fresh_id();
            env.insert(
                "map-remove".to_string(),
                TypeScheme {
                    vars: vec![k],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::map(), Type::Var(k)], Box::new(Type::map())),
                },
            );
        }

        // read-file: String -> String (ファイル内容を読み込み)
        env.insert(
            "read-file".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::string()], Box::new(Type::string()))),
        );

        // write-file: (String, String) -> Int (ファイルに書き込み、書き込みバイト数を返す)
        env.insert(
            "write-file".to_string(),
            TypeScheme::mono(Type::Fun(
                vec![Type::string(), Type::string()],
                Box::new(Type::int()),
            )),
        );

        // write-file-bytes: (String, Vector) -> Int (Vector の下位 8 bit を raw bytes として書き込み)
        env.insert(
            "write-file-bytes".to_string(),
            TypeScheme::mono(Type::Fun(
                vec![Type::string(), Type::vector()],
                Box::new(Type::int()),
            )),
        );

        // file-exists?: String -> Bool (ファイルが存在するか)
        env.insert(
            "file-exists?".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::string()], Box::new(Type::bool()))),
        );

        // command-line-args: () -> Int (引数の数を返す)
        env.insert(
            "command-line-args".to_string(),
            TypeScheme::mono(Type::Fun(vec![], Box::new(Type::int()))),
        );

        // command-line-arg: Int -> String (指定 index の argv 要素を返す)
        env.insert(
            "command-line-arg".to_string(),
            TypeScheme::mono(Type::Fun(vec![Type::int()], Box::new(Type::string()))),
        );

        // read-stdin: () -> String (stdin 全体を読む)
        env.insert(
            "read-stdin".to_string(),
            TypeScheme::mono(Type::Fun(vec![], Box::new(Type::string()))),
        );

        // root_push: forall a. a -> Int (任意値を root stack に積み、slot handle を返す)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "root_push".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::Var(a)], Box::new(Type::int())),
                },
            );
        }

        // root_pop: forall a. () -> a (root stack から最新値を取り出す)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "root_pop".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![], Box::new(Type::Var(a))),
                },
            );
        }

        // root_set: forall a. (Int, a) -> Int (既存 slot を別値へ差し替える)
        {
            let a = self.var_gen.fresh_id();
            env.insert(
                "root_set".to_string(),
                TypeScheme {
                    vars: vec![a],
                    constraints: Vec::new(),
                    ty: Type::Fun(vec![Type::int(), Type::Var(a)], Box::new(Type::int())),
                },
            );
        }

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

        // 組み込み型の Kind を登録
        for name in ["Int", "Float", "String", "Bool", "Unit"] {
            self.kind_env.insert(name.to_string(), Kind::star());
        }

        // Functor トレイト: fmap : (a -> b) -> f a -> f b
        // Kind 制約: f : * -> *
        {
            let f_var = self.var_gen.fresh_id();
            let a_var = self.var_gen.fresh_id();
            let b_var = self.var_gen.fresh_id();
            let fmap_type = Type::Fun(
                vec![
                    // (a -> b)
                    Type::Fun(vec![Type::Var(a_var)], Box::new(Type::Var(b_var))),
                    // f a
                    Type::App("__f__".to_string(), vec![Type::Var(a_var)]),
                ],
                // f b
                Box::new(Type::App("__f__".to_string(), vec![Type::Var(b_var)])),
            );
            let fmap_scheme = TypeScheme {
                vars: vec![f_var, a_var, b_var],
                constraints: vec![TraitConstraint {
                    trait_name: "Functor".to_string(),
                    type_var: f_var,
                }],
                ty: fmap_type,
            };
            self.trait_registry.insert(
                "Functor".to_string(),
                TraitInfo {
                    name: "Functor".to_string(),
                    type_param: f_var,
                    methods: vec![("fmap".to_string(), fmap_scheme.clone())],
                },
            );
            self.kind_env.insert("Functor".to_string(), Kind::unary());
        }

        // Monad トレイト: bind : m a -> (a -> m b) -> m b, pure : a -> m a
        // Kind 制約: m : * -> *
        {
            let m_var = self.var_gen.fresh_id();
            let a_var = self.var_gen.fresh_id();
            let b_var = self.var_gen.fresh_id();
            let monad_constraint = TraitConstraint {
                trait_name: "Monad".to_string(),
                type_var: m_var,
            };
            // bind : m a -> (a -> m b) -> m b
            let bind_type = Type::Fun(
                vec![
                    Type::App("__m__".to_string(), vec![Type::Var(a_var)]),
                    Type::Fun(
                        vec![Type::Var(a_var)],
                        Box::new(Type::App("__m__".to_string(), vec![Type::Var(b_var)])),
                    ),
                ],
                Box::new(Type::App("__m__".to_string(), vec![Type::Var(b_var)])),
            );
            let bind_scheme = TypeScheme {
                vars: vec![m_var, a_var, b_var],
                constraints: vec![monad_constraint.clone()],
                ty: bind_type,
            };
            // pure : a -> m a
            let a2_var = self.var_gen.fresh_id();
            let pure_type = Type::Fun(
                vec![Type::Var(a2_var)],
                Box::new(Type::App("__m__".to_string(), vec![Type::Var(a2_var)])),
            );
            let pure_scheme = TypeScheme {
                vars: vec![m_var, a2_var],
                constraints: vec![monad_constraint],
                ty: pure_type,
            };
            self.trait_registry.insert(
                "Monad".to_string(),
                TraitInfo {
                    name: "Monad".to_string(),
                    type_param: m_var,
                    methods: vec![
                        ("bind".to_string(), bind_scheme),
                        ("pure".to_string(), pure_scheme),
                    ],
                },
            );
            self.kind_env.insert("Monad".to_string(), Kind::unary());
        }

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
    fn resolve_type_expr(&self, type_expr: &TypeExpr, param_vars: &[(String, TypeVarId)]) -> Type {
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
    fn resolve_qualified_name(&self, prefix: &str, suffix: &str) -> Option<String> {
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
    fn detect_alias_name(&self, type_expr: &TypeExpr) -> Option<String> {
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

    fn record_expr_type(&mut self, expr: &Expr, subst: &Substitution, ty: &Type) {
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

    /// 式の型推論 (Algorithm W)
    fn infer_expr(
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

            let current_env = env.apply_subst(&subst);
            let (s1, field_ty) = self.infer_expr(&current_env, field_expr)?;
            subst = subst.compose(&s1);

            let s2 = self.unify(&field_ty, &expected_ty.apply_subst(&subst), span)?;
            subst = subst.compose(&s2);

            result_fields.push((field_name.clone(), field_ty.apply_subst(&subst)));
        }

        let record_type = Type::Record(type_name.to_string(), result_fields);
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
                            let mut pat_subst = Substitution::new();
                            for (sub_pat, expected_ty) in sub_pats.iter().zip(param_types.iter()) {
                                let (pat_ty, bindings) = self.infer_pattern(env, sub_pat)?;
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

                            let final_ret = ret_type.apply_subst(&pat_subst);
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

                let record_type = Type::Record(type_name.to_string(), result_fields);
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

    /// 型スキームをインスタンス化
    fn instantiate(&mut self, scheme: &TypeScheme) -> Type {
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

    /// 型を汎化
    fn generalize(&self, env: &TypeEnv, ty: &Type) -> TypeScheme {
        let env_vars = env.free_vars();
        let ty_vars = ty.free_vars();
        let vars: Vec<TypeVarId> = ty_vars
            .into_iter()
            .filter(|v| !env_vars.contains(v))
            .collect();
        TypeScheme {
            vars,
            constraints: Vec::new(),
            ty: ty.clone(),
        }
    }

    /// 2つの型を統合 (Unification)
    fn unify(&mut self, t1: &Type, t2: &Type, span: Span) -> Result<Substitution, TypeError> {
        match (t1, t2) {
            (Type::Con(a), Type::Con(b)) if a == b => Ok(Substitution::new()),

            (left, right) if Self::int_heap_compatible(left, right) => Ok(Substitution::new()),

            (Type::Var(id), ty) | (ty, Type::Var(id)) => self.bind_var(*id, ty, span),

            (Type::Fun(params1, ret1), Type::Fun(params2, ret2)) => {
                if params1.len() != params2.len() {
                    return Err(TypeError::Mismatch {
                        expected: t1.clone(),
                        found: t2.clone(),
                        span,
                        error_code: TypeErrorCode::General,
                    });
                }
                let mut subst = Substitution::new();
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    let s = self.unify(&p1.apply_subst(&subst), &p2.apply_subst(&subst), span)?;
                    subst = subst.compose(&s);
                }
                let s_ret =
                    self.unify(&ret1.apply_subst(&subst), &ret2.apply_subst(&subst), span)?;
                Ok(subst.compose(&s_ret))
            }

            (Type::App(name1, args1), Type::App(name2, args2))
                if name1 == name2 && args1.len() == args2.len() =>
            {
                let mut subst = Substitution::new();
                for (a1, a2) in args1.iter().zip(args2.iter()) {
                    let s = self.unify(&a1.apply_subst(&subst), &a2.apply_subst(&subst), span)?;
                    subst = subst.compose(&s);
                }
                Ok(subst)
            }

            (Type::Record(name1, fields1), Type::Record(name2, fields2))
                if name1 == name2 && fields1.len() == fields2.len() =>
            {
                let mut subst = Substitution::new();
                for ((n1, t1), (n2, t2)) in fields1.iter().zip(fields2.iter()) {
                    if n1 != n2 {
                        return Err(TypeError::Mismatch {
                            expected: Type::Record(name1.clone(), fields1.clone()),
                            found: Type::Record(name2.clone(), fields2.clone()),
                            span,
                            error_code: TypeErrorCode::General,
                        });
                    }
                    let s = self.unify(&t1.apply_subst(&subst), &t2.apply_subst(&subst), span)?;
                    subst = subst.compose(&s);
                }
                Ok(subst)
            }

            // レコード型と Con 型の統合（レコード名が一致する場合）
            (Type::Record(name, _), Type::Con(con_name))
            | (Type::Con(con_name), Type::Record(name, _))
                if name == con_name =>
            {
                Ok(Substitution::new())
            }

            _ => Err(TypeError::Mismatch {
                expected: t1.clone(),
                found: t2.clone(),
                span,
                error_code: TypeErrorCode::General,
            }),
        }
    }

    fn int_heap_compatible(left: &Type, right: &Type) -> bool {
        (matches!(left, Type::Con(name) if name == "Int") && right.is_heap_handle())
            || (matches!(right, Type::Con(name) if name == "Int") && left.is_heap_handle())
    }

    /// 型変数を型に束縛（occurs check 付き）
    fn bind_var(
        &mut self,
        var: TypeVarId,
        ty: &Type,
        span: Span,
    ) -> Result<Substitution, TypeError> {
        if let Type::Var(id) = ty
            && *id == var
        {
            return Ok(Substitution::new());
        }

        if ty.free_vars().contains(&var) {
            return Err(TypeError::InfiniteType {
                var,
                ty: ty.clone(),
                span,
            });
        }

        let mut subst = Substitution::new();
        subst.insert(var, ty.clone());
        // グローバル代入に累積（制約チェック用）
        self.global_subst.insert(var, ty.clone());
        Ok(subst)
    }
}

impl Default for Infer {
    fn default() -> Self {
        Self::new()
    }
}
