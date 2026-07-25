use crate::ast::{ComputationStep, Expr, Literal, MatchArm};
use crate::span::Span;
use crate::token::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    /// 式をパース
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        // 深いネスト式での stack overflow を防ぐため、必要時にヒープ上へスタックを拡張
        stacker::maybe_grow(64 * 1024, 1024 * 1024, || self.parse_expr_inner())
    }

    fn parse_expr_inner(&mut self) -> Result<Expr, ParseError> {
        match self.peek_kind() {
            Some(TokenKind::LParen) => self.parse_list_expr(),
            Some(TokenKind::LBrace) => self.parse_brace_expr(),
            Some(TokenKind::Int(_)) => {
                let tok = self.advance();
                if let TokenKind::Int(n) = tok.kind {
                    Ok(Expr::Lit(tok.span, Literal::Int(n)))
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::Float(_)) => {
                let tok = self.advance();
                if let TokenKind::Float(n) = tok.kind {
                    Ok(Expr::Lit(tok.span, Literal::Float(n)))
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::String(_)) => {
                let tok = self.advance();
                if let TokenKind::String(s) = tok.kind {
                    Ok(Expr::Lit(tok.span, Literal::String(s)))
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::Bool(_)) => {
                let tok = self.advance();
                if let TokenKind::Bool(b) = tok.kind {
                    Ok(Expr::Lit(tok.span, Literal::Bool(b)))
                } else {
                    unreachable!()
                }
            }
            Some(TokenKind::Symbol(_)) => {
                let tok = self.advance();
                if let TokenKind::Symbol(name) = tok.kind {
                    // ドット区切りシンボルの処理: TypeName.field
                    if let Some(dot_pos) = name.find('.') {
                        let prefix = &name[..dot_pos];
                        let suffix = &name[dot_pos + 1..];
                        if !prefix.is_empty()
                            && !suffix.is_empty()
                            && prefix.starts_with(|c: char| c.is_ascii_uppercase())
                        {
                            // TypeName.field 形式のフィールドアクセス（関数として使用）
                            return Ok(Expr::Var(tok.span, name));
                        }
                    }
                    Ok(Expr::Var(tok.span, name))
                } else {
                    unreachable!()
                }
            }
            // P10-1: Quote 式 -- 'expr
            Some(TokenKind::Quote) => {
                let tok = self.advance();
                let inner = self.parse_expr()?;
                let span = tok.span.merge(inner.span());
                Ok(Expr::Quote(span, Box::new(inner)))
            }
            // P10-1: Unquote 式 -- ~expr
            Some(TokenKind::Unquote) => {
                let tok = self.advance();
                let inner = self.parse_expr()?;
                let span = tok.span.merge(inner.span());
                Ok(Expr::Unquote(span, Box::new(inner)))
            }
            // P10-1: SpliceUnquote 式 -- ~@expr
            Some(TokenKind::SpliceUnquote) => {
                let tok = self.advance();
                let inner = self.parse_expr()?;
                let span = tok.span.merge(inner.span());
                Ok(Expr::UnquoteSplice(span, Box::new(inner)))
            }
            Some(kind) => Err(ParseError::Unexpected {
                expected: "式".to_string(),
                found: kind.to_string(),
                span: self.peek_span(),
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: "式".to_string(),
            }),
        }
    }

    /// ブレース式のパース
    /// {TypeName field1 val1 field2 val2}  -- レコードリテラル
    /// {expr | field1 val1 ...}            -- レコード更新
    fn parse_brace_expr(&mut self) -> Result<Expr, ParseError> {
        let start_span = self.expect(TokenKind::LBrace)?.span;

        // 最初のトークンを確認
        let first = self.parse_expr()?;

        // パイプがあればレコード更新
        if self.check(TokenKind::Pipe) {
            self.advance(); // |
            let mut fields = Vec::new();
            while !self.check(TokenKind::RBrace) {
                let field_name = self.expect_symbol()?;
                let field_val = self.parse_expr()?;
                fields.push((field_name, field_val));
            }
            let end_span = self.expect(TokenKind::RBrace)?.span;
            return Ok(Expr::RecordUpdate(
                start_span.merge(end_span),
                Box::new(first),
                fields,
            ));
        }

        // 最初のトークンが大文字シンボルならレコードリテラル
        if let Expr::Var(_, ref name) = first
            && name.starts_with(|c: char| c.is_ascii_uppercase())
        {
            let type_name = name.clone();
            let mut fields = Vec::new();
            while !self.check(TokenKind::RBrace) {
                let field_name = self.expect_symbol()?;
                let field_val = self.parse_expr()?;
                fields.push((field_name, field_val));
            }
            let end_span = self.expect(TokenKind::RBrace)?.span;
            return Ok(Expr::RecordLit(
                start_span.merge(end_span),
                type_name,
                fields,
            ));
        }

        // その他のブレース式はエラー
        Err(ParseError::Unexpected {
            expected: "レコードリテラルまたはレコード更新".to_string(),
            found: "不明なブレース式".to_string(),
            span: start_span,
        })
    }

    /// 括弧で始まる式をパース
    fn parse_list_expr(&mut self) -> Result<Expr, ParseError> {
        let start_span = self.expect(TokenKind::LParen)?.span;

        // 空リスト = unit
        if self.check(TokenKind::RParen) {
            let end_span = self.advance().span;
            return Ok(Expr::Lit(start_span.merge(end_span), Literal::Unit));
        }

        // 先頭トークンで分岐
        match self.peek_kind() {
            Some(TokenKind::If) => self.parse_if(start_span),
            Some(TokenKind::Let) => self.parse_let(start_span),
            Some(TokenKind::Fn) => self.parse_lambda(start_span),
            Some(TokenKind::Match) => self.parse_match(start_span),
            Some(TokenKind::Do) => self.parse_do(start_span),
            Some(TokenKind::Colon) => self.parse_ann(start_span),
            Some(TokenKind::Computation) => self.parse_computation(start_span),
            Some(TokenKind::Dot) => self.parse_field_access(start_span),
            _ => self.parse_app(start_span),
        }
    }

    /// `(. expr field)` -- レコード field access
    fn parse_field_access(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        self.advance(); // .
        let expr = self.parse_expr()?;
        let field = self.expect_symbol()?;
        let end_span = self.expect(TokenKind::RParen)?.span;
        Ok(Expr::FieldAccess(
            start_span.merge(end_span),
            Box::new(expr),
            field,
        ))
    }

    /// (if cond then else)
    fn parse_if(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        self.advance(); // if
        let cond = self.parse_expr()?;
        let then = self.parse_expr()?;
        let else_ = self.parse_expr()?;
        let end_span = self.expect(TokenKind::RParen)?.span;
        Ok(Expr::If(
            start_span.merge(end_span),
            Box::new(cond),
            Box::new(then),
            Box::new(else_),
        ))
    }

    /// (let [pat1 val1 pat2 val2 ...] body)
    fn parse_let(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        self.advance(); // let
        self.expect(TokenKind::LBracket)?;

        let mut bindings = Vec::new();
        while !self.check(TokenKind::RBracket) {
            let pat = self.parse_pattern()?;
            let val = self.parse_expr()?;
            bindings.push((pat, val));
        }
        self.advance(); // ]

        let body = self.parse_expr()?;
        let end_span = self.expect(TokenKind::RParen)?.span;
        Ok(Expr::Let(
            start_span.merge(end_span),
            bindings,
            Box::new(body),
        ))
    }

    /// (fn [params] body)
    fn parse_lambda(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        self.advance(); // fn
        let params = self.parse_params()?;
        let body = self.parse_expr()?;
        let end_span = self.expect(TokenKind::RParen)?.span;
        Ok(Expr::Lambda(
            start_span.merge(end_span),
            params,
            Box::new(body),
        ))
    }

    /// (match scrutinee [pat1 body1] [pat2 body2] ...)
    fn parse_match(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        self.advance(); // match
        let scrutinee = self.parse_expr()?;

        let mut arms = Vec::new();
        while self.check(TokenKind::LBracket) {
            let arm_start = self.advance().span; // [
            let pattern = self.parse_pattern()?;
            // ガード条件 (when 節) のチェック
            let guard = if let Some(TokenKind::Symbol(ref s)) = self.peek_kind() {
                if s == "when" {
                    self.advance(); // when
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                }
            } else {
                None
            };
            let body = self.parse_expr()?;
            let arm_end = self.expect(TokenKind::RBracket)?.span;
            arms.push(MatchArm {
                span: arm_start.merge(arm_end),
                pattern,
                guard,
                body,
            });
        }

        let end_span = self.expect(TokenKind::RParen)?.span;
        Ok(Expr::Match(
            start_span.merge(end_span),
            Box::new(scrutinee),
            arms,
        ))
    }

    /// (do expr1 expr2 ...)
    fn parse_do(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        self.advance(); // do
        let mut exprs = Vec::new();
        while !self.check(TokenKind::RParen) {
            exprs.push(self.parse_expr()?);
        }
        let end_span = self.advance().span; // )
        Ok(Expr::Do(start_span.merge(end_span), exprs))
    }

    /// (computation builder-name step1 step2 ...)
    /// ステップ: (let! pattern expr) | (do! expr) | (return expr) | expr
    fn parse_computation(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        self.advance(); // computation
        let builder_name = self.expect_symbol()?;
        let mut steps = Vec::new();
        while !self.check(TokenKind::RParen) {
            let step = self.parse_computation_step()?;
            steps.push(step);
        }
        let end_span = self.advance().span; // )
        Ok(Expr::Computation(
            start_span.merge(end_span),
            builder_name,
            steps,
        ))
    }

    /// Computation Expression のステップをパース
    fn parse_computation_step(&mut self) -> Result<ComputationStep, ParseError> {
        let span = self.peek_span();
        // (let! pat expr) または (do! expr) または (return expr)
        if self.check(TokenKind::LParen) {
            self.advance(); // (
            let step_span = self.peek_span();
            if let Some(TokenKind::Symbol(ref name)) = self.peek_kind() {
                let name = name.clone();
                match name.as_str() {
                    "let!" => {
                        self.advance(); // let!
                        let pat = self.parse_pattern()?;
                        let expr = self.parse_expr()?;
                        let end = self.expect(TokenKind::RParen)?.span;
                        return Ok(ComputationStep::LetBang(step_span.merge(end), pat, expr));
                    }
                    "do!" => {
                        self.advance(); // do!
                        let expr = self.parse_expr()?;
                        let end = self.expect(TokenKind::RParen)?.span;
                        return Ok(ComputationStep::DoBang(step_span.merge(end), expr));
                    }
                    "return" => {
                        self.advance(); // return
                        let expr = self.parse_expr()?;
                        let end = self.expect(TokenKind::RParen)?.span;
                        return Ok(ComputationStep::Return(step_span.merge(end), expr));
                    }
                    _ => {}
                }
            }
            // 通常の S 式として巻き戻してパース
            // ( は既に消費済みなので parse_list_expr の内部からパースする
            let expr = match self.peek_kind() {
                Some(TokenKind::If) => self.parse_if(span)?,
                Some(TokenKind::Let) => self.parse_let(span)?,
                Some(TokenKind::Fn) => self.parse_lambda(span)?,
                Some(TokenKind::Match) => self.parse_match(span)?,
                Some(TokenKind::Do) => self.parse_do(span)?,
                Some(TokenKind::Colon) => self.parse_ann(span)?,
                Some(TokenKind::Computation) => self.parse_computation(span)?,
                _ => self.parse_app(span)?,
            };
            Ok(ComputationStep::Expr(expr))
        } else {
            // アトムの場合は通常の式
            let expr = self.parse_expr()?;
            Ok(ComputationStep::Expr(expr))
        }
    }

    /// (: expr Type) -- 型注釈
    fn parse_ann(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        self.advance(); // :
        let expr = self.parse_expr()?;
        let ty = self.parse_type_expr()?;
        let end_span = self.expect(TokenKind::RParen)?.span;
        Ok(Expr::Ann(start_span.merge(end_span), Box::new(expr), ty))
    }

    /// 関数適用 (f arg1 arg2 ...)
    fn parse_app(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        let func = self.parse_expr()?;
        let mut args = Vec::new();
        while !self.check(TokenKind::RParen) {
            args.push(self.parse_expr()?);
        }
        let end_span = self.advance().span; // )
        Ok(Expr::App(start_span.merge(end_span), Box::new(func), args))
    }
}
