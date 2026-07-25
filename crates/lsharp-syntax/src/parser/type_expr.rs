use crate::ast::TypeExpr;
use crate::token::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// 型式をパース
    pub(super) fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
        stacker::maybe_grow(64 * 1024, 1024 * 1024, || self.parse_type_expr_inner())
    }

    fn parse_type_expr_inner(&mut self) -> Result<TypeExpr, ParseError> {
        match self.peek_kind() {
            Some(TokenKind::Symbol(ref s)) if s.starts_with(|c: char| c.is_ascii_uppercase()) => {
                let tok = self.advance();
                if let TokenKind::Symbol(name) = tok.kind {
                    Ok(TypeExpr::Named(tok.span, name))
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::Symbol(_)) => {
                // 小文字 = 型変数
                let tok = self.advance();
                if let TokenKind::Symbol(name) = tok.kind {
                    Ok(TypeExpr::Var(tok.span, name))
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::LParen) => {
                let start_span = self.advance().span;

                // レコード型
                if self.check(TokenKind::Record) {
                    self.advance(); // record
                    let mut fields = Vec::new();
                    while !self.check(TokenKind::RParen) {
                        self.expect(TokenKind::LParen)?;
                        self.expect(TokenKind::Colon)?;
                        let field_name = self.expect_symbol()?;
                        let field_type = self.parse_type_expr()?;
                        self.expect(TokenKind::RParen)?;
                        fields.push((field_name, field_type));
                    }
                    let end_span = self.advance().span;
                    return Ok(TypeExpr::Record(start_span.merge(end_span), fields));
                }

                if self.check(TokenKind::Arrow) {
                    // (-> Param1 Param2 Ret)
                    self.advance(); // ->
                    let mut types = Vec::new();
                    while !self.check(TokenKind::RParen) {
                        types.push(self.parse_type_expr()?);
                    }
                    let end_span = self.advance().span;
                    let ret = types.pop().ok_or(ParseError::UnexpectedEof {
                        expected: "戻り値型".to_string(),
                    })?;
                    Ok(TypeExpr::Fun(
                        start_span.merge(end_span),
                        types,
                        Box::new(ret),
                    ))
                } else {
                    // (TypeName Arg1 Arg2 ...)
                    let base = self.parse_type_expr()?;
                    let mut args = Vec::new();
                    while !self.check(TokenKind::RParen) {
                        args.push(self.parse_type_expr()?);
                    }
                    let end_span = self.advance().span;
                    Ok(TypeExpr::App(
                        start_span.merge(end_span),
                        Box::new(base),
                        args,
                    ))
                }
            }
            Some(kind) => Err(ParseError::Unexpected {
                expected: "型".to_string(),
                found: kind.to_string(),
                span: self.peek_span(),
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: "型".to_string(),
            }),
        }
    }
}
