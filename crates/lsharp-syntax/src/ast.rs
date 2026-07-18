use crate::metadata::MetadataForm;
use crate::span::Span;

/// リテラル値
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Unit,
}

/// 型式（パーサー段階の未解決な型表現）
#[derive(Debug, Clone, PartialEq)]
pub enum TypeExpr {
    /// 型名 (Int, String, Bool, ...)
    Named(Span, String),
    /// 型適用 (Option Int), (Result String Int)
    App(Span, Box<TypeExpr>, Vec<TypeExpr>),
    /// 関数型 (-> Int Int Int)
    Fun(Span, Vec<TypeExpr>, Box<TypeExpr>),
    /// 型変数 (小文字の識別子: a, b, ...)
    Var(Span, String),
    /// レコード型 (record (: field1 Type1) (: field2 Type2))
    Record(Span, Vec<(String, TypeExpr)>),
}

/// パラメータ
#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub span: Span,
    pub name: String,
    /// オプションの型注釈
    pub ty: Option<TypeExpr>,
}

/// パターン（match や let で使用）
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// ワイルドカード _
    Wildcard(Span),
    /// 変数束縛
    Var(Span, String),
    /// リテラルパターン
    Lit(Span, Literal),
    /// コンストラクタパターン (Some x), None
    Constructor(Span, String, Vec<Pattern>),
    /// レコードパターン {TypeName field1 pat1 field2 pat2}
    RecordPat(Span, String, Vec<(String, Pattern)>),
}

/// match の腕
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub span: Span,
    pub pattern: Pattern,
    /// ガード条件 (when 節): パターンマッチ成功後に評価される追加条件
    pub guard: Option<Box<Expr>>,
    pub body: Expr,
}

/// 制約（型制約）
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    Gte(Expr),
    Lte(Expr),
    Range(Expr, Expr),
    Matches(String),
    MinLength(Expr),
    MaxLength(Expr),
    OneOf(Vec<Expr>),
    Satisfies(String),
}

/// トレイトメソッド定義
#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    pub span: Span,
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: Option<TypeExpr>,
    pub default_impl: Option<Expr>,
}

/// トレイト制約 (:where [(Trait a) ...])
#[derive(Debug, Clone, PartialEq)]
pub struct WhereClause {
    pub span: Span,
    pub trait_name: String,
    pub type_var: String,
}

/// 構造化メタデータ
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Metadata {
    pub doc: Option<String>,
    pub params: Vec<(String, String)>,
    pub returns: Option<String>,
    pub invariant: Option<Expr>,
    pub rationale: Option<String>,
    pub see_also: Vec<String>,
    pub example: Vec<Expr>,
    pub since: Option<String>,
    /// ADT 状態遷移メタデータ :transitions [(from -> to) ...]
    pub transitions: Vec<(String, String)>,
    /// source order と directive span を保持する lossless contract forms。
    /// 既存 field は v0.1 consumer 向けの互換 projection として併存する。
    pub forms: Vec<MetadataForm>,
}

/// 式
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// リテラル
    Lit(Span, Literal),
    /// 変数参照
    Var(Span, String),
    /// if 式
    If(Span, Box<Expr>, Box<Expr>, Box<Expr>),
    /// let 束縛
    Let(Span, Vec<(Pattern, Expr)>, Box<Expr>),
    /// ラムダ式 (fn [x y] body)
    Lambda(Span, Vec<Param>, Box<Expr>),
    /// 関数適用 (f arg1 arg2)
    App(Span, Box<Expr>, Vec<Expr>),
    /// パターンマッチ
    Match(Span, Box<Expr>, Vec<MatchArm>),
    /// 逐次実行 (do expr1 expr2 ...)
    Do(Span, Vec<Expr>),
    /// 型注釈 (: expr type)
    Ann(Span, Box<Expr>, TypeExpr),
    /// レコードリテラル {TypeName field1 val1 field2 val2}
    RecordLit(Span, String, Vec<(String, Expr)>),
    /// フィールドアクセス TypeName.field expr
    FieldAccess(Span, Box<Expr>, String),
    /// レコード更新 {expr | field1 val1 ...}
    RecordUpdate(Span, Box<Expr>, Vec<(String, Expr)>),
    /// Computation Expression (builder-name { body })
    /// let! によるモナディック束縛と return を含む
    Computation(Span, String, Vec<ComputationStep>),
    /// P10-1: Quote 式 'expr -- AST をデータとして扱う
    Quote(Span, Box<Expr>),
    /// P10-1: Unquote 式 ~expr -- quote 内で式を評価する
    Unquote(Span, Box<Expr>),
    /// P10-1: UnquoteSplice 式 ~@expr -- quote 内でリストを展開する
    UnquoteSplice(Span, Box<Expr>),
}

/// Computation Expression のステップ
#[derive(Debug, Clone, PartialEq)]
pub enum ComputationStep {
    /// let! x = expr (モナディック束縛 -- bind に脱糖)
    LetBang(Span, Pattern, Expr),
    /// do! expr (モナディック実行 -- bind に脱糖、結果を捨てる)
    DoBang(Span, Expr),
    /// return expr (return に脱糖)
    Return(Span, Expr),
    /// 通常の式
    Expr(Expr),
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Lit(s, _)
            | Expr::Var(s, _)
            | Expr::If(s, _, _, _)
            | Expr::Let(s, _, _)
            | Expr::Lambda(s, _, _)
            | Expr::App(s, _, _)
            | Expr::Match(s, _, _)
            | Expr::Do(s, _)
            | Expr::Ann(s, _, _)
            | Expr::RecordLit(s, _, _)
            | Expr::FieldAccess(s, _, _)
            | Expr::RecordUpdate(s, _, _)
            | Expr::Computation(s, _, _)
            | Expr::Quote(s, _)
            | Expr::Unquote(s, _)
            | Expr::UnquoteSplice(s, _) => *s,
        }
    }
}

/// 代数的データ型のバリアント
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub span: Span,
    pub name: String,
    pub fields: Vec<TypeExpr>,
    /// GADT: バリアント別の戻り型（None の場合は通常 ADT）
    pub return_type: Option<TypeExpr>,
}

/// トップレベル宣言
#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    /// 関数定義 (defn name [params] body)
    Defn {
        span: Span,
        name: String,
        params: Vec<Param>,
        return_ty: Option<TypeExpr>,
        body: Expr,
        where_clauses: Vec<WhereClause>,
        metadata: Option<Metadata>,
    },
    /// 型定義 (type (Name a b) (Variant1 ...) (Variant2 ...))
    TypeDef {
        span: Span,
        name: String,
        type_params: Vec<String>,
        variants: Vec<Variant>,
        metadata: Option<Metadata>,
    },
    /// レコード型定義 (type Name (record (: field1 Type1) ...))
    RecordDef {
        span: Span,
        name: String,
        type_params: Vec<String>,
        fields: Vec<(String, TypeExpr)>,
    },
    /// 型エイリアス (type-alias Name Type)
    TypeAlias {
        span: Span,
        name: String,
        params: Vec<String>,
        target: TypeExpr,
    },
    /// 制約付き型 (type-constrained Name BaseType :constraints [...])
    TypeConstrained {
        span: Span,
        name: String,
        base_type: TypeExpr,
        constraints: Vec<Constraint>,
    },
    /// モジュール宣言 (module Name) または (module Name decl1 decl2 ...)
    ModuleDecl {
        span: Span,
        name: String,
        /// ネストモジュールの本体宣言（空の場合はマーカーのみ）
        body: Vec<Decl>,
    },
    /// インポート宣言 (import Name ...)
    ImportDecl {
        span: Span,
        module: String,
        alias: Option<String>,
        only: Option<Vec<String>>,
        open: bool,
    },
    /// トレイト定義 (trait (TraitName a) ...)
    TraitDef {
        span: Span,
        name: String,
        type_param: String,
        methods: Vec<TraitMethod>,
    },
    /// トレイト実装 (impl (TraitName Type) ...)
    ImplDef {
        span: Span,
        trait_name: String,
        type_name: String,
        methods: Vec<Decl>,
    },
    /// 非公開宣言 (private (defn ...))
    Private { span: Span, inner: Box<Decl> },
    /// Computation Builder 宣言 (computation-builder name bind-fn return-fn)
    ComputationBuilder {
        span: Span,
        name: String,
        bind_fn: String,
        return_fn: String,
    },
    /// P10-2: マクロ定義 (defmacro name [params] body)
    DefMacro {
        span: Span,
        name: String,
        params: Vec<Param>,
        /// オプションの型シグネチャ (P10-3)
        macro_type: Option<TypeExpr>,
        body: Expr,
    },
}

/// プログラム全体
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub decls: Vec<Decl>,
}

/// AST の人間が読める表示
impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, decl) in self.decls.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{decl}")?;
        }
        Ok(())
    }
}

impl std::fmt::Display for Decl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Decl::Defn {
                name,
                params,
                return_ty,
                body,
                ..
            } => {
                write!(f, "(defn {name} [")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", p.name)?;
                }
                write!(f, "]")?;
                if let Some(ty) = return_ty {
                    write!(f, " : {ty}")?;
                }
                write!(f, " {body})")
            }
            Decl::TypeDef {
                name,
                type_params,
                variants,
                ..
            } => {
                if type_params.is_empty() {
                    write!(f, "(type {name}")?;
                } else {
                    write!(f, "(type ({name}")?;
                    for p in type_params {
                        write!(f, " {p}")?;
                    }
                    write!(f, ")")?;
                }
                for v in variants {
                    if v.fields.is_empty() {
                        write!(f, " {}", v.name)?;
                    } else {
                        write!(f, " ({}", v.name)?;
                        for field in &v.fields {
                            write!(f, " {field}")?;
                        }
                        write!(f, ")")?;
                    }
                }
                write!(f, ")")
            }
            Decl::RecordDef {
                name,
                type_params,
                fields,
                ..
            } => {
                if type_params.is_empty() {
                    write!(f, "(type {name} (record")?;
                } else {
                    write!(f, "(type ({name}")?;
                    for p in type_params {
                        write!(f, " {p}")?;
                    }
                    write!(f, ") (record")?;
                }
                for (fname, fty) in fields {
                    write!(f, " (: {fname} {fty})")?;
                }
                write!(f, "))")
            }
            Decl::TypeAlias {
                name,
                params,
                target,
                ..
            } => {
                if params.is_empty() {
                    write!(f, "(type-alias {name} {target})")
                } else {
                    write!(f, "(type-alias ({name}")?;
                    for p in params {
                        write!(f, " {p}")?;
                    }
                    write!(f, ") {target})")
                }
            }
            Decl::TypeConstrained {
                name, base_type, ..
            } => {
                write!(f, "(type-constrained {name} {base_type} ...)")
            }
            Decl::ModuleDecl { name, body, .. } => {
                if body.is_empty() {
                    write!(f, "(module {name})")
                } else {
                    write!(f, "(module {name}")?;
                    for d in body {
                        write!(f, " {d}")?;
                    }
                    write!(f, ")")
                }
            }
            Decl::ImportDecl {
                module,
                alias,
                only,
                open,
                ..
            } => {
                write!(f, "(import {module}")?;
                if let Some(a) = alias {
                    write!(f, " :as {a}")?;
                }
                if let Some(syms) = only {
                    write!(f, " :only [")?;
                    for (i, s) in syms.iter().enumerate() {
                        if i > 0 {
                            write!(f, " ")?;
                        }
                        write!(f, "{s}")?;
                    }
                    write!(f, "]")?;
                }
                if *open {
                    write!(f, " :open")?;
                }
                write!(f, ")")
            }
            Decl::TraitDef {
                name,
                type_param,
                methods,
                ..
            } => {
                write!(f, "(trait ({name} {type_param})")?;
                for m in methods {
                    write!(f, " (defn {} [", m.name)?;
                    for (i, p) in m.params.iter().enumerate() {
                        if i > 0 {
                            write!(f, " ")?;
                        }
                        write!(f, "{}", p.name)?;
                    }
                    write!(f, "]")?;
                    if let Some(body) = &m.default_impl {
                        write!(f, " {body}")?;
                    }
                    write!(f, ")")?;
                }
                write!(f, ")")
            }
            Decl::ImplDef {
                trait_name,
                type_name,
                methods,
                ..
            } => {
                write!(f, "(impl ({trait_name} {type_name})")?;
                for m in methods {
                    write!(f, " {m}")?;
                }
                write!(f, ")")
            }
            Decl::Private { inner, .. } => {
                write!(f, "(private {inner})")
            }
            Decl::ComputationBuilder {
                name,
                bind_fn,
                return_fn,
                ..
            } => {
                write!(f, "(computation-builder {name} {bind_fn} {return_fn})")
            }
            Decl::DefMacro {
                name, params, body, ..
            } => {
                write!(f, "(defmacro {name} [")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", p.name)?;
                }
                write!(f, "] {body})")
            }
        }
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Lit(_, lit) => write!(f, "{lit}"),
            Expr::Var(_, name) => write!(f, "{name}"),
            Expr::If(_, cond, then, else_) => {
                write!(f, "(if {cond} {then} {else_})")
            }
            Expr::Let(_, bindings, body) => {
                write!(f, "(let [")?;
                for (i, (pat, val)) in bindings.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{pat} {val}")?;
                }
                write!(f, "] {body})")
            }
            Expr::Lambda(_, params, body) => {
                write!(f, "(fn [")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", p.name)?;
                }
                write!(f, "] {body})")
            }
            Expr::App(_, func, args) => {
                write!(f, "({func}")?;
                for arg in args {
                    write!(f, " {arg}")?;
                }
                write!(f, ")")
            }
            Expr::Match(_, scrutinee, arms) => {
                write!(f, "(match {scrutinee}")?;
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        write!(f, " [{} when {} {}]", arm.pattern, guard, arm.body)?;
                    } else {
                        write!(f, " [{} {}]", arm.pattern, arm.body)?;
                    }
                }
                write!(f, ")")
            }
            Expr::Do(_, exprs) => {
                write!(f, "(do")?;
                for e in exprs {
                    write!(f, " {e}")?;
                }
                write!(f, ")")
            }
            Expr::Ann(_, expr, ty) => {
                write!(f, "(: {expr} {ty})")
            }
            Expr::RecordLit(_, type_name, fields) => {
                write!(f, "{{{type_name}")?;
                for (name, val) in fields {
                    write!(f, " {name} {val}")?;
                }
                write!(f, "}}")
            }
            Expr::FieldAccess(_, expr, field) => {
                write!(f, "(. {expr} {field})")
            }
            Expr::RecordUpdate(_, base, fields) => {
                write!(f, "{{({base}) |")?;
                for (name, val) in fields {
                    write!(f, " {name} {val}")?;
                }
                write!(f, "}}")
            }
            Expr::Computation(_, builder, steps) => {
                write!(f, "(computation")?;
                if !builder.is_empty() {
                    write!(f, " {builder}")?;
                }
                for step in steps {
                    write!(f, " ")?;
                    match step {
                        ComputationStep::LetBang(_, pat, expr) => write!(f, "(let! {pat} {expr})")?,
                        ComputationStep::DoBang(_, expr) => write!(f, "(do! {expr})")?,
                        ComputationStep::Return(_, expr) => write!(f, "(return {expr})")?,
                        ComputationStep::Expr(expr) => write!(f, "{expr}")?,
                    }
                }
                write!(f, ")")
            }
            Expr::Quote(_, expr) => write!(f, "'{expr}"),
            Expr::Unquote(_, expr) => write!(f, "~{expr}"),
            Expr::UnquoteSplice(_, expr) => write!(f, "~@{expr}"),
        }
    }
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Int(n) => write!(f, "{n}"),
            Literal::Float(n) => write!(f, "{n}"),
            Literal::String(s) => write!(f, "\"{s}\""),
            Literal::Bool(b) => write!(f, "{b}"),
            Literal::Unit => write!(f, "()"),
        }
    }
}

impl std::fmt::Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pattern::Wildcard(_) => write!(f, "_"),
            Pattern::Var(_, name) => write!(f, "{name}"),
            Pattern::Lit(_, lit) => write!(f, "{lit}"),
            Pattern::Constructor(_, name, fields) => {
                if fields.is_empty() {
                    write!(f, "{name}")
                } else {
                    write!(f, "({name}")?;
                    for field in fields {
                        write!(f, " {field}")?;
                    }
                    write!(f, ")")
                }
            }
            Pattern::RecordPat(_, type_name, fields) => {
                write!(f, "{{{type_name}")?;
                for (name, pat) in fields {
                    write!(f, " {name} {pat}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl std::fmt::Display for TypeExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeExpr::Named(_, name) => write!(f, "{name}"),
            TypeExpr::Var(_, name) => write!(f, "{name}"),
            TypeExpr::App(_, base, args) => {
                write!(f, "({base}")?;
                for arg in args {
                    write!(f, " {arg}")?;
                }
                write!(f, ")")
            }
            TypeExpr::Fun(_, params, ret) => {
                write!(f, "(->")?;
                for p in params {
                    write!(f, " {p}")?;
                }
                write!(f, " {ret})")
            }
            TypeExpr::Record(_, fields) => {
                write!(f, "(record")?;
                for (name, ty) in fields {
                    write!(f, " (: {name} {ty})")?;
                }
                write!(f, ")")
            }
        }
    }
}
