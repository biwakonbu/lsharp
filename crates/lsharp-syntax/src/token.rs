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

    // 型注釈
    Colon, // :
    Arrow, // ->

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
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Arrow => write!(f, "->"),
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
