use crate::ast::{Literal, Pattern};
use crate::token::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// パターンをパース
    pub(super) fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        stacker::maybe_grow(64 * 1024, 1024 * 1024, || self.parse_pattern_inner())
    }

    fn parse_pattern_inner(&mut self) -> Result<Pattern, ParseError> {
        match self.peek_kind() {
            Some(TokenKind::Symbol(ref s)) if s == "_" => {
                let tok = self.advance();
                Ok(Pattern::Wildcard(tok.span))
            }
            Some(TokenKind::Symbol(ref s)) if s.starts_with(|c: char| c.is_ascii_uppercase()) => {
                // コンストラクタ（引数なし）
                let tok = self.advance();
                if let TokenKind::Symbol(name) = tok.kind {
                    Ok(Pattern::Constructor(tok.span, name, Vec::new()))
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::Symbol(_)) => {
                let tok = self.advance();
                if let TokenKind::Symbol(name) = tok.kind {
                    Ok(Pattern::Var(tok.span, name))
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::Int(_)) => {
                let tok = self.advance();
                if let TokenKind::Int(n) = tok.kind {
                    Ok(Pattern::Lit(tok.span, Literal::Int(n)))
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::Bool(_)) => {
                let tok = self.advance();
                if let TokenKind::Bool(b) = tok.kind {
                    Ok(Pattern::Lit(tok.span, Literal::Bool(b)))
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::String(_)) => {
                let tok = self.advance();
                if let TokenKind::String(s) = tok.kind {
                    Ok(Pattern::Lit(tok.span, Literal::String(s)))
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::LParen) => {
                // (Constructor pat1 pat2 ...)
                let start_span = self.advance().span;
                let name = self.expect_symbol()?;
                let mut fields = Vec::new();
                while !self.check(TokenKind::RParen) {
                    fields.push(self.parse_pattern()?);
                }
                let end_span = self.advance().span; // )
                Ok(Pattern::Constructor(
                    start_span.merge(end_span),
                    name,
                    fields,
                ))
            }
            Some(TokenKind::LBrace) => {
                // {TypeName field1 pat1 field2 pat2 ...}
                let start_span = self.advance().span;
                let type_name = self.expect_symbol()?;
                let mut fields = Vec::new();
                while !self.check(TokenKind::RBrace) {
                    let field_name = self.expect_symbol()?;
                    let field_pat = self.parse_pattern()?;
                    fields.push((field_name, field_pat));
                }
                let end_span = self.advance().span; // }
                Ok(Pattern::RecordPat(
                    start_span.merge(end_span),
                    type_name,
                    fields,
                ))
            }
            Some(TokenKind::Eof) | None => Err(ParseError::UnexpectedEof {
                expected: "パターン".to_string(),
                span: self.peek_span(),
            }),
            Some(kind) => Err(ParseError::Unexpected {
                expected: "パターン".to_string(),
                found: kind.to_string(),
                span: self.peek_span(),
            }),
        }
    }
}
