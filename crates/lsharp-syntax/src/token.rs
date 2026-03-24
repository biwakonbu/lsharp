use crate::span::Span;

/// トークンの種類
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // 区切り文字
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    LBrace,   // {
    RBrace,   // }

    // リテラル
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),

    // 識別子・シンボル
    Symbol(String),

    // キーワード
    Defn,
    Let,
    If,
    Match,
    Type,
    Fn,
    Do,
    Module,
    Import,
    Record,          // P1-1: レコード型キーワード
    Trait,           // P5-1: トレイトキーワード
    Impl,            // P5-1: impl キーワード
    Where,           // P5-1: where キーワード
    TypeAlias,       // P2-1: 型エイリアスキーワード
    TypeConstrained, // P2-1: 制約付き型キーワード
    Constraints,     // P2-1: 制約ブロックキーワード
    Private,         // P4-3: 可視性キーワード
    Computation,     // P6-3: Computation Expression キーワード
    ComputationBuilder, // P6-3: computation-builder 宣言キーワード
    DefMacro,        // P10-2: マクロ定義キーワード

    // 型注釈
    Colon, // :
    Arrow, // ->
    Pipe,  // | (レコード更新構文用)
    Dot,   // . (フィールドアクセス用)

    // マクロ (P10-1: Quote/Unquote 基盤)
    Quote,        // ' (quote)
    Unquote,      // ~ (unquote)
    SpliceUnquote, // ~@ (splice-unquote)

    // 特殊
    Eof,
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::Int(n) => write!(f, "{n}"),
            TokenKind::Float(n) => write!(f, "{n}"),
            TokenKind::String(s) => write!(f, "\"{s}\""),
            TokenKind::Bool(b) => write!(f, "{b}"),
            TokenKind::Symbol(s) => write!(f, "{s}"),
            TokenKind::Defn => write!(f, "defn"),
            TokenKind::Let => write!(f, "let"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::Type => write!(f, "type"),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Do => write!(f, "do"),
            TokenKind::Module => write!(f, "module"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::Record => write!(f, "record"),
            TokenKind::Trait => write!(f, "trait"),
            TokenKind::Impl => write!(f, "impl"),
            TokenKind::Where => write!(f, "where"),
            TokenKind::TypeAlias => write!(f, "type-alias"),
            TokenKind::TypeConstrained => write!(f, "type-constrained"),
            TokenKind::Constraints => write!(f, "constraints"),
            TokenKind::Private => write!(f, "private"),
            TokenKind::Computation => write!(f, "computation"),
            TokenKind::ComputationBuilder => write!(f, "computation-builder"),
            TokenKind::DefMacro => write!(f, "defmacro"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::Quote => write!(f, "'"),
            TokenKind::Unquote => write!(f, "~"),
            TokenKind::SpliceUnquote => write!(f, "~@"),
            TokenKind::Eof => write!(f, "EOF"),
        }
    }
}

/// Span 付きトークン
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
