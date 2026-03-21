use crate::ast::*;
use crate::span::Span;
use crate::token::{Token, TokenKind};

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
    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut decls = Vec::new();
        while !self.is_eof() {
            decls.push(self.parse_decl()?);
        }
        Ok(Program { decls })
    }

    /// トップレベル宣言をパース
    fn parse_decl(&mut self) -> Result<Decl, ParseError> {
        let start_span = self.expect(TokenKind::LParen)?.span;

        let decl = match self.peek_kind() {
            Some(TokenKind::Defn) => self.parse_defn(start_span)?,
            Some(TokenKind::Type) => self.parse_type_def(start_span)?,
            Some(kind) => {
                let span = self.peek_span();
                return Err(ParseError::UnknownForm {
                    name: kind.to_string(),
                    span,
                });
            }
            None => {
                return Err(ParseError::UnexpectedEof {
                    expected: "宣言".to_string(),
                })
            }
        };

        Ok(decl)
    }

    /// (defn name [params] body)
    /// (defn name [params] : RetType body)
    fn parse_defn(&mut self, start_span: Span) -> Result<Decl, ParseError> {
        self.advance(); // defn をスキップ
        let name = self.expect_symbol()?;
        let params = self.parse_params()?;

        // オプションの戻り値型注釈 `: RetType`
        let return_ty = if self.check(TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        let body = self.parse_expr()?;
        let end_span = self.expect(TokenKind::RParen)?.span;

        Ok(Decl::Defn {
            span: start_span.merge(end_span),
            name,
            params,
            return_ty,
            body,
        })
    }

    /// (type Name Variant1 Variant2 ...)
    /// (type (Name a b) (Variant1 Type1) Variant2 ...)
    fn parse_type_def(&mut self, start_span: Span) -> Result<Decl, ParseError> {
        self.advance(); // type をスキップ

        let (name, type_params) = if self.check(TokenKind::LParen) {
            // (type (Name a b) ...)
            self.advance();
            let name = self.expect_symbol()?;
            let mut params = Vec::new();
            while !self.check(TokenKind::RParen) {
                params.push(self.expect_symbol()?);
            }
            self.advance(); // )
            (name, params)
        } else {
            // (type Name ...)
            let name = self.expect_symbol()?;
            (name, Vec::new())
        };

        let mut variants = Vec::new();
        while !self.check(TokenKind::RParen) {
            variants.push(self.parse_variant()?);
        }
        let end_span = self.advance().span; // )

        Ok(Decl::TypeDef {
            span: start_span.merge(end_span),
            name,
            type_params,
            variants,
        })
    }

    /// バリアント: Name または (Name Type1 Type2 ...)
    fn parse_variant(&mut self) -> Result<Variant, ParseError> {
        if self.check(TokenKind::LParen) {
            let start_span = self.advance().span;
            let name = self.expect_symbol()?;
            let mut fields = Vec::new();
            while !self.check(TokenKind::RParen) {
                fields.push(self.parse_type_expr()?);
            }
            let end_span = self.advance().span; // )
            Ok(Variant {
                span: start_span.merge(end_span),
                name,
                fields,
            })
        } else {
            let span = self.peek_span();
            let name = self.expect_symbol()?;
            Ok(Variant {
                span,
                name,
                fields: Vec::new(),
            })
        }
    }

    /// パラメータリスト [x y] または [(: x Int) y]
    fn parse_params(&mut self) -> Result<Vec<Param>, ParseError> {
        self.expect(TokenKind::LBracket)?;
        let mut params = Vec::new();
        while !self.check(TokenKind::RBracket) {
            params.push(self.parse_param()?);
        }
        self.advance(); // ]
        Ok(params)
    }

    /// パラメータ: name または (: name Type)
    fn parse_param(&mut self) -> Result<Param, ParseError> {
        if self.check(TokenKind::LParen) {
            self.advance().span;
            self.expect(TokenKind::Colon)?;
            let name_span = self.peek_span();
            let name = self.expect_symbol()?;
            let ty = self.parse_type_expr()?;
            let end_span = self.expect(TokenKind::RParen)?.span;
            Ok(Param {
                span: name_span.merge(end_span),
                name,
                ty: Some(ty),
            })
        } else {
            let span = self.peek_span();
            let name = self.expect_symbol()?;
            Ok(Param {
                span,
                name,
                ty: None,
            })
        }
    }

    /// 式をパース
    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        match self.peek_kind() {
            Some(TokenKind::LParen) => self.parse_list_expr(),
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
                    Ok(Expr::Var(tok.span, name))
                } else {
                    unreachable!()
                }
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
            _ => self.parse_app(start_span),
        }
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
            let body = self.parse_expr()?;
            let arm_end = self.expect(TokenKind::RBracket)?.span;
            arms.push(MatchArm {
                span: arm_start.merge(arm_end),
                pattern,
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

    /// (: expr Type) — 型注釈
    fn parse_ann(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        self.advance(); // :
        let expr = self.parse_expr()?;
        let ty = self.parse_type_expr()?;
        let end_span = self.expect(TokenKind::RParen)?.span;
        Ok(Expr::Ann(
            start_span.merge(end_span),
            Box::new(expr),
            ty,
        ))
    }

    /// 関数適用 (f arg1 arg2 ...)
    fn parse_app(&mut self, start_span: Span) -> Result<Expr, ParseError> {
        let func = self.parse_expr()?;
        let mut args = Vec::new();
        while !self.check(TokenKind::RParen) {
            args.push(self.parse_expr()?);
        }
        let end_span = self.advance().span; // )
        Ok(Expr::App(
            start_span.merge(end_span),
            Box::new(func),
            args,
        ))
    }

    /// パターンをパース
    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
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
            Some(kind) => Err(ParseError::Unexpected {
                expected: "パターン".to_string(),
                found: kind.to_string(),
                span: self.peek_span(),
            }),
            None => Err(ParseError::UnexpectedEof {
                expected: "パターン".to_string(),
            }),
        }
    }

    /// 型式をパース
    fn parse_type_expr(&mut self) -> Result<TypeExpr, ParseError> {
        match self.peek_kind() {
            Some(TokenKind::Symbol(ref s))
                if s.starts_with(|c: char| c.is_ascii_uppercase()) =>
            {
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

    // --- ヘルパーメソッド ---

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
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
            let found = self.peek_kind().map(|k| k.to_string()).unwrap_or("EOF".to_string());
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
                let found = self.peek_kind().map(|k| k.to_string()).unwrap_or("EOF".to_string());
                Err(ParseError::Unexpected {
                    expected: "シンボル".to_string(),
                    found,
                    span: self.peek_span(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(input: &str) -> Program {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_program().unwrap()
    }

    fn parse_expr_str(input: &str) -> Expr {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_expr().unwrap()
    }

    #[test]
    fn test_simple_defn() {
        let prog = parse("(defn add [x y] (+ x y))");
        assert_eq!(prog.decls.len(), 1);
        assert_eq!(prog.to_string(), "(defn add [x y] (+ x y))");
    }

    #[test]
    fn test_defn_with_type_annotation() {
        let prog = parse("(defn add [(: x Int) (: y Int)] : Int (+ x y))");
        assert_eq!(prog.decls.len(), 1);
        assert_eq!(
            prog.to_string(),
            "(defn add [x y] : Int (+ x y))"
        );
    }

    #[test]
    fn test_fib() {
        let prog = parse(
            "(defn fib [n] (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))",
        );
        assert_eq!(prog.decls.len(), 1);
    }

    #[test]
    fn test_let_expr() {
        let expr = parse_expr_str("(let [x 10 y 20] (+ x y))");
        assert_eq!(expr.to_string(), "(let [x 10 y 20] (+ x y))");
    }

    #[test]
    fn test_lambda() {
        let expr = parse_expr_str("(fn [x y] (+ x y))");
        assert_eq!(expr.to_string(), "(fn [x y] (+ x y))");
    }

    #[test]
    fn test_match() {
        let expr = parse_expr_str("(match x [(Some v) v] [None 0])");
        assert_eq!(expr.to_string(), "(match x [(Some v) v] [None 0])");
    }

    #[test]
    fn test_type_def() {
        let prog = parse("(type (Option a) (Some a) None)");
        assert_eq!(prog.decls.len(), 1);
        assert_eq!(prog.to_string(), "(type (Option a) (Some a) None)");
    }

    #[test]
    fn test_do_expr() {
        let expr = parse_expr_str("(do (print 1) (print 2))");
        assert_eq!(expr.to_string(), "(do (print 1) (print 2))");
    }

    #[test]
    fn test_type_annotation_expr() {
        let expr = parse_expr_str("(: 42 Int)");
        assert_eq!(expr.to_string(), "(: 42 Int)");
    }

    #[test]
    fn test_unit() {
        let expr = parse_expr_str("()");
        assert_eq!(expr.to_string(), "()");
    }

    #[test]
    fn test_multiple_decls() {
        let prog = parse(
            "(defn add [x y] (+ x y))
             (defn main [] (print (add 1 2)))",
        );
        assert_eq!(prog.decls.len(), 2);
    }
}
