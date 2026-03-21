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
}

/// match の腕
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub span: Span,
    pub pattern: Pattern,
    pub body: Expr,
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
            | Expr::Ann(s, _, _) => *s,
        }
    }
}

/// 代数的データ型のバリアント
#[derive(Debug, Clone, PartialEq)]
pub struct Variant {
    pub span: Span,
    pub name: String,
    pub fields: Vec<TypeExpr>,
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
    },
    /// 型定義 (type (Name a b) (Variant1 ...) (Variant2 ...))
    TypeDef {
        span: Span,
        name: String,
        type_params: Vec<String>,
        variants: Vec<Variant>,
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
                    write!(f, " [{} {}]", arm.pattern, arm.body)?;
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
        }
    }
}
