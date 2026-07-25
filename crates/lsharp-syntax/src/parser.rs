use crate::ast::*;
use crate::span::Span;
use crate::token::{Token, TokenKind};

mod decl;
mod evidence;
mod expr;
mod metadata;
mod pattern;
mod type_expr;

/// パースエラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    #[error("予期しないトークン: {found} (期待: {expected}) ({span})")]
    Unexpected {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("予期しない入力終端 (期待: {expected})")]
    UnexpectedEof { expected: String },

    #[error("不明なフォーム: {name} ({span})")]
    UnknownForm { name: String, span: Span },

    #[error("複数のパースエラー: {}", .0.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; "))]
    Multiple(Vec<ParseError>),
}

impl ParseError {
    /// 利用者向けの安定した診断コードを返す。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unexpected { .. } => "LS0101",
            Self::UnexpectedEof { .. } => "LS0102",
            Self::UnknownForm { .. } => "LS0103",
            Self::Multiple(_) => "LS0104",
        }
    }

    /// 診断に対応する source span を返す。
    /// EOF は現在の AST/API が位置を保持していないため `None` になる。
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Unexpected { span, .. } | Self::UnknownForm { span, .. } => Some(*span),
            Self::UnexpectedEof { .. } => None,
            Self::Multiple(errors) => errors.first().and_then(Self::span),
        }
    }
}

/// パーサー
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    /// プログラム全体をパース
    /// エラーが発生した場合、次のトップレベル宣言まで回復して継続する。
    /// 複数エラーがある場合は `ParseError::Multiple` で一括報告する。
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let (prog, errors) = self.parse_program_recovering();
        if errors.is_empty() {
            Ok(prog)
        } else if errors.len() == 1 {
            Err(errors.into_iter().next().unwrap())
        } else {
            Err(ParseError::Multiple(errors))
        }
    }

    /// プログラム全体をパースし、部分的な結果とエラーを両方返す。
    /// エラーがあっても正常にパースできた宣言を取得できる。
    pub fn parse_program_recovering(&mut self) -> (Program, Vec<ParseError>) {
        let mut decls = Vec::new();
        let mut errors = Vec::new();

        while !self.is_eof() {
            match self.parse_decl() {
                Ok(decl) => decls.push(decl),
                Err(e) => {
                    errors.push(e);
                    self.recover_to_next_decl();
                }
            }
        }

        (Program { decls }, errors)
    }

    /// エラー回復: 次のトップレベル宣言の先頭まで tokens をスキップする。
    /// トップレベル宣言は `(` で始まるため、括弧のネスト深度を追跡しながら
    /// 現在の不正な宣言を飛ばし、次の `(` が深度 0 で出現する位置まで進む。
    fn recover_to_next_decl(&mut self) {
        let mut depth: i32 = 0;
        while !self.is_eof() {
            match self.peek_kind() {
                Some(TokenKind::LParen) => {
                    if depth <= 0 {
                        // 次のトップレベル宣言の開始位置に到達
                        return;
                    }
                    depth += 1;
                    self.advance();
                }
                Some(TokenKind::RParen) => {
                    depth -= 1;
                    self.advance();
                    if depth <= 0 {
                        // 現在の宣言の閉じ括弧を消費した
                        return;
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    pub(super) fn expect_metadata_string(
        &mut self,
        expected: &str,
    ) -> Result<(String, Span), ParseError> {
        match self.peek_kind() {
            Some(TokenKind::String(_)) => {
                let token = self.advance();
                match token.kind {
                    TokenKind::String(value) => Ok((value, token.span)),
                    _ => unreachable!("peek_kind が String を返した後に変化しない"),
                }
            }
            Some(kind) => Err(ParseError::Unexpected {
                expected: expected.to_string(),
                found: kind.to_string(),
                span: self.peek_span(),
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: expected.to_string(),
            }),
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|t| t.kind.clone())
    }

    fn peek_span(&self) -> Span {
        self.peek().map(|t| t.span).unwrap_or(Span::dummy())
    }

    fn is_eof(&self) -> bool {
        matches!(self.peek_kind(), Some(TokenKind::Eof) | None)
    }

    fn check(&self, kind: TokenKind) -> bool {
        matches!(self.peek_kind(), Some(ref k) if std::mem::discriminant(k) == std::mem::discriminant(&kind))
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.pos].clone();
        self.pos += 1;
        tok
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, ParseError> {
        if self.check(kind.clone()) {
            Ok(self.advance())
        } else {
            let found = self
                .peek_kind()
                .map(|k| k.to_string())
                .unwrap_or("EOF".to_string());
            Err(ParseError::Unexpected {
                expected: kind.to_string(),
                found,
                span: self.peek_span(),
            })
        }
    }

    fn expect_symbol(&mut self) -> Result<String, ParseError> {
        match self.peek_kind() {
            Some(TokenKind::Symbol(_)) => {
                let tok = self.advance();
                if let TokenKind::Symbol(name) = tok.kind {
                    Ok(name)
                } else {
                    unreachable!()
                }
            }
            _ => {
                let found = self
                    .peek_kind()
                    .map(|k| k.to_string())
                    .unwrap_or("EOF".to_string());
                Err(ParseError::Unexpected {
                    expected: "シンボル".to_string(),
                    found,
                    span: self.peek_span(),
                })
            }
        }
    }
}

fn type_expr_span(ty: &TypeExpr) -> Span {
    match ty {
        TypeExpr::Named(span, _)
        | TypeExpr::App(span, _, _)
        | TypeExpr::Fun(span, _, _)
        | TypeExpr::Var(span, _)
        | TypeExpr::Record(span, _) => *span,
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod type_expr_tests;

#[cfg(test)]
mod expr_tests;

#[cfg(test)]
mod evidence_tests;

#[cfg(test)]
mod decl_tests;

#[cfg(test)]
mod metadata_tests;

#[cfg(test)]
mod pattern_tests;

#[cfg(test)]
mod transitions_tests;

#[cfg(test)]
mod computation_tests;
