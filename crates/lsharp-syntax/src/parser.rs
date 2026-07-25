use crate::ast::*;
use crate::span::Span;
use crate::token::{Token, TokenKind};

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

    /// トップレベル宣言をパース
    fn parse_decl(&mut self) -> Result<Decl, ParseError> {
        let start_span = self.expect(TokenKind::LParen)?.span;

        let decl = match self.peek_kind() {
            Some(TokenKind::Defn) => self.parse_defn(start_span)?,
            Some(TokenKind::Type) => self.parse_type_def(start_span)?,
            Some(TokenKind::TypeAlias) => self.parse_type_alias(start_span)?,
            Some(TokenKind::TypeConstrained) => self.parse_type_constrained(start_span)?,
            Some(TokenKind::Module) => self.parse_module_decl(start_span)?,
            Some(TokenKind::Import) => self.parse_import_decl(start_span)?,
            Some(TokenKind::Trait) => self.parse_trait_def(start_span)?,
            Some(TokenKind::Impl) => self.parse_impl_def(start_span)?,
            Some(TokenKind::Private) => self.parse_private(start_span)?,
            Some(TokenKind::ComputationBuilder) => self.parse_computation_builder(start_span)?,
            Some(TokenKind::DefMacro) => self.parse_defmacro(start_span)?,
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
                });
            }
        };

        Ok(decl)
    }

    /// (defn name [params] body)
    /// (defn name [params] : RetType body)
    /// (defn name [params] : RetType :where [(Trait a) ...] body)
    fn parse_defn(&mut self, start_span: Span) -> Result<Decl, ParseError> {
        self.advance(); // defn をスキップ
        let name = self.expect_symbol()?;
        let params = self.parse_params()?;

        // オプションの戻り値型注釈 `: RetType`
        // `:` の後がディレクティブキーワード（where, doc 等）の場合は型注釈ではない
        let return_ty = if self.check(TokenKind::Colon) && !self.is_colon_directive() {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        // オプションの :where 制約句
        let where_clauses = self.parse_where_clauses()?;

        // オプションのメタデータ
        let metadata = self.try_parse_metadata()?;

        let body = self.parse_expr()?;
        let end_span = self.expect(TokenKind::RParen)?.span;

        Ok(Decl::Defn {
            span: start_span.merge(end_span),
            name,
            params,
            return_ty,
            body,
            where_clauses,
            metadata,
        })
    }

    /// :where [(Trait a) ...] 制約句のパース
    fn parse_where_clauses(&mut self) -> Result<Vec<WhereClause>, ParseError> {
        // :where キーワードを確認
        // レキサーでは : + Symbol("where") になるか、Where トークンになる
        if self.check(TokenKind::Colon) {
            if let Some(TokenKind::Symbol(ref s)) = self.peek_at(1).map(|t| &t.kind).cloned()
                && s == "where"
            {
                let span = self.peek_span();
                self.advance(); // :
                self.advance(); // where

                return self.parse_where_clause_list(span);
            }
            // Where トークンの場合
            if self.peek_at(1).map(|t| &t.kind) == Some(&TokenKind::Where) {
                let span = self.peek_span();
                self.advance(); // :
                self.advance(); // where

                return self.parse_where_clause_list(span);
            }
        }
        // where キーワード単体の場合
        if self.check(TokenKind::Where) {
            let span = self.peek_span();
            self.advance(); // where
            return self.parse_where_clause_list(span);
        }

        Ok(Vec::new())
    }

    /// where 制約リストのパース: [(Trait a) ...]
    fn parse_where_clause_list(
        &mut self,
        _where_span: Span,
    ) -> Result<Vec<WhereClause>, ParseError> {
        self.expect(TokenKind::LBracket)?;
        let mut clauses = Vec::new();
        while !self.check(TokenKind::RBracket) {
            let clause_start = self.expect(TokenKind::LParen)?.span;
            let trait_name = self.expect_symbol()?;
            let type_var = self.expect_symbol()?;
            let clause_end = self.expect(TokenKind::RParen)?.span;
            clauses.push(WhereClause {
                span: clause_start.merge(clause_end),
                trait_name,
                type_var,
            });
        }
        self.advance(); // ]
        Ok(clauses)
    }

    /// (type Name Variant1 Variant2 ...)
    /// (type (Name a b) (Variant1 Type1) Variant2 ...)
    /// (type Name (record (: field1 Type1) ...))
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

        // レコード型かどうかを確認
        if self.check(TokenKind::LParen) {
            // 先読みして record キーワードを確認
            if self.peek_at(1).map(|t| &t.kind) == Some(&TokenKind::Record) {
                return self.parse_record_def(start_span, name, type_params);
            }
        }

        let mut variants = Vec::new();
        while !self.check(TokenKind::RParen) && !self.check(TokenKind::Colon) {
            variants.push(self.parse_variant()?);
        }
        let metadata = self.try_parse_metadata()?;
        let end_span = self.advance().span; // )

        Ok(Decl::TypeDef {
            span: start_span.merge(end_span),
            name,
            type_params,
            variants,
            metadata,
        })
    }

    /// レコード型定義のパース
    /// (type Name (record (: field1 Type1) (: field2 Type2)))
    fn parse_record_def(
        &mut self,
        start_span: Span,
        name: String,
        type_params: Vec<String>,
    ) -> Result<Decl, ParseError> {
        self.expect(TokenKind::LParen)?; // (
        self.advance(); // record をスキップ

        let mut fields = Vec::new();
        while !self.check(TokenKind::RParen) {
            // (: field Type)
            self.expect(TokenKind::LParen)?;
            self.expect(TokenKind::Colon)?;
            let field_name = self.expect_symbol()?;
            let field_type = self.parse_type_expr()?;
            self.expect(TokenKind::RParen)?;
            fields.push((field_name, field_type));
        }
        self.advance(); // ) (record を閉じる)

        let metadata = self.try_parse_metadata()?;
        let end_span = self.expect(TokenKind::RParen)?.span; // (type を閉じる)

        Ok(Decl::RecordDef {
            span: start_span.merge(end_span),
            name,
            type_params,
            fields,
            metadata,
        })
    }

    /// 型エイリアスのパース
    /// (type-alias Name Type)
    /// (type-alias (Name a b) Type)
    fn parse_type_alias(&mut self, start_span: Span) -> Result<Decl, ParseError> {
        self.advance(); // type-alias をスキップ

        let (name, params) = if self.check(TokenKind::LParen) {
            self.advance();
            let name = self.expect_symbol()?;
            let mut params = Vec::new();
            while !self.check(TokenKind::RParen) {
                params.push(self.expect_symbol()?);
            }
            self.advance(); // )
            (name, params)
        } else {
            let name = self.expect_symbol()?;
            (name, Vec::new())
        };

        let target = self.parse_type_expr()?;
        let end_span = self.expect(TokenKind::RParen)?.span;

        Ok(Decl::TypeAlias {
            span: start_span.merge(end_span),
            name,
            params,
            target,
        })
    }

    /// 制約付き型のパース
    /// (type-constrained Name BaseType :constraints [(>= 0) (<= 100)])
    fn parse_type_constrained(&mut self, start_span: Span) -> Result<Decl, ParseError> {
        self.advance(); // type-constrained をスキップ

        let name = self.expect_symbol()?;
        let base_type = self.parse_type_expr()?;

        let mut constraints = Vec::new();

        // :constraints [...] を探す
        if self.check(TokenKind::Colon) {
            let next_is_constraints = match self.peek_at(1).map(|t| &t.kind) {
                Some(TokenKind::Constraints) => true,
                Some(TokenKind::Symbol(s)) if s == "constraints" => true,
                _ => false,
            };
            if next_is_constraints {
                self.advance(); // :
                self.advance(); // constraints
                self.expect(TokenKind::LBracket)?;

                while !self.check(TokenKind::RBracket) {
                    constraints.push(self.parse_constraint()?);
                }
                self.advance(); // ]
            }
        }

        let end_span = self.expect(TokenKind::RParen)?.span;

        Ok(Decl::TypeConstrained {
            span: start_span.merge(end_span),
            name,
            base_type,
            constraints,
        })
    }

    /// 制約述語のパース
    /// (>= N), (<= N), (range N M), (matches "..."), (min-length N),
    /// (max-length N), (one-of [v1 v2 ...]), (satisfies fn-name)
    fn parse_constraint(&mut self) -> Result<Constraint, ParseError> {
        self.expect(TokenKind::LParen)?;

        let op = self.expect_symbol()?;
        let constraint = match op.as_str() {
            ">=" => {
                let val = self.parse_expr()?;
                Constraint::Gte(val)
            }
            "<=" => {
                let val = self.parse_expr()?;
                Constraint::Lte(val)
            }
            "range" => {
                let lo = self.parse_expr()?;
                let hi = self.parse_expr()?;
                Constraint::Range(lo, hi)
            }
            "matches" => {
                let tok = self.advance();
                if let TokenKind::String(s) = tok.kind {
                    Constraint::Matches(s)
                } else {
                    return Err(ParseError::Unexpected {
                        expected: "文字列パターン".to_string(),
                        found: tok.kind.to_string(),
                        span: tok.span,
                    });
                }
            }
            "min-length" => {
                let val = self.parse_expr()?;
                Constraint::MinLength(val)
            }
            "max-length" => {
                let val = self.parse_expr()?;
                Constraint::MaxLength(val)
            }
            "one-of" => {
                self.expect(TokenKind::LBracket)?;
                let mut values = Vec::new();
                while !self.check(TokenKind::RBracket) {
                    values.push(self.parse_expr()?);
                }
                self.advance(); // ]
                Constraint::OneOf(values)
            }
            "satisfies" => {
                let fn_name = self.expect_symbol()?;
                Constraint::Satisfies(fn_name)
            }
            _ => {
                return Err(ParseError::UnknownForm {
                    name: format!("制約演算子: {op}"),
                    span: self.peek_span(),
                });
            }
        };

        self.expect(TokenKind::RParen)?;
        Ok(constraint)
    }

    /// 非公開宣言のパース
    /// (private (defn ...))
    fn parse_private(&mut self, start_span: Span) -> Result<Decl, ParseError> {
        self.advance(); // private をスキップ

        // 内側の宣言をパース（LParen から始まる）
        let inner = self.parse_decl()?;
        let end_span = self.expect(TokenKind::RParen)?.span;

        Ok(Decl::Private {
            span: start_span.merge(end_span),
            inner: Box::new(inner),
        })
    }

    /// (computation-builder name bind-fn return-fn)
    fn parse_computation_builder(&mut self, start_span: Span) -> Result<Decl, ParseError> {
        self.advance(); // computation-builder をスキップ
        let name = self.expect_symbol()?;
        let bind_fn = self.expect_symbol()?;
        let return_fn = self.expect_symbol()?;
        let end_span = self.expect(TokenKind::RParen)?.span;
        Ok(Decl::ComputationBuilder {
            span: start_span.merge(end_span),
            name,
            bind_fn,
            return_fn,
        })
    }

    /// P10-2: マクロ定義のパース
    /// (defmacro name [params] body)
    /// (defmacro name [params] : MacroType body)
    fn parse_defmacro(&mut self, start_span: Span) -> Result<Decl, ParseError> {
        self.advance(); // defmacro をスキップ
        let name = self.expect_symbol()?;
        let params = self.parse_params()?;

        // オプションの型シグネチャ
        let macro_type = if self.check(TokenKind::Colon) && !self.is_colon_directive() {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        let body = self.parse_expr()?;
        let end_span = self.expect(TokenKind::RParen)?.span;

        Ok(Decl::DefMacro {
            span: start_span.merge(end_span),
            name,
            params,
            macro_type,
            body,
        })
    }

    /// モジュール宣言のパース
    /// (module Name.Space)                       -- マーカーのみ
    /// (module Name (defn ...) (defn ...) ...)    -- ネストモジュール（本体あり）
    fn parse_module_decl(&mut self, start_span: Span) -> Result<Decl, ParseError> {
        self.advance(); // module をスキップ
        let name = self.expect_symbol()?;

        // RParen なら本体なしのマーカーモジュール
        // LParen なら本体ありのネストモジュール
        let mut body = Vec::new();
        while !self.check(TokenKind::RParen) {
            body.push(self.parse_decl()?);
        }
        let end_span = self.expect(TokenKind::RParen)?.span;

        Ok(Decl::ModuleDecl {
            span: start_span.merge(end_span),
            name,
            body,
        })
    }

    /// インポート宣言のパース
    /// (import Name :as Alias)
    /// (import Name :only [sym1 sym2])
    /// (import Name :open)
    fn parse_import_decl(&mut self, start_span: Span) -> Result<Decl, ParseError> {
        self.advance(); // import をスキップ
        let module = self.expect_symbol()?;

        let mut alias = None;
        let mut only = None;
        let mut open = false;

        // オプションの修飾子をパース
        while !self.check(TokenKind::RParen) {
            if self.check(TokenKind::Colon) {
                self.advance(); // :
            }
            match self.peek_kind() {
                Some(TokenKind::Symbol(ref s)) if s == "as" || s == ":as" => {
                    self.advance();
                    alias = Some(self.expect_symbol()?);
                }
                Some(TokenKind::Symbol(ref s)) if s == "only" || s == ":only" => {
                    self.advance();
                    self.expect(TokenKind::LBracket)?;
                    let mut syms = Vec::new();
                    while !self.check(TokenKind::RBracket) {
                        syms.push(self.expect_symbol()?);
                    }
                    self.advance(); // ]
                    only = Some(syms);
                }
                Some(TokenKind::Symbol(ref s)) if s == "open" || s == ":open" => {
                    self.advance();
                    open = true;
                }
                _ => break,
            }
        }

        let end_span = self.expect(TokenKind::RParen)?.span;

        Ok(Decl::ImportDecl {
            span: start_span.merge(end_span),
            module,
            alias,
            only,
            open,
        })
    }

    /// トレイト定義のパース
    /// (trait (TraitName a) (defn method [...] ...))
    fn parse_trait_def(&mut self, start_span: Span) -> Result<Decl, ParseError> {
        self.advance(); // trait をスキップ

        // (TraitName a)
        self.expect(TokenKind::LParen)?;
        let name = self.expect_symbol()?;
        let type_param = self.expect_symbol()?;
        self.expect(TokenKind::RParen)?;

        let mut methods = Vec::new();
        while !self.check(TokenKind::RParen) {
            methods.push(self.parse_trait_method()?);
        }

        let end_span = self.expect(TokenKind::RParen)?.span;

        Ok(Decl::TraitDef {
            span: start_span.merge(end_span),
            name,
            type_param,
            methods,
        })
    }

    /// トレイトメソッドのパース
    fn parse_trait_method(&mut self) -> Result<TraitMethod, ParseError> {
        let start_span = self.expect(TokenKind::LParen)?.span;
        self.expect(TokenKind::Defn)?;
        let name = self.expect_symbol()?;
        let params = self.parse_params()?;

        let return_ty = if self.check(TokenKind::Colon) {
            self.advance();
            Some(self.parse_type_expr()?)
        } else {
            None
        };

        let default_impl = if !self.check(TokenKind::RParen) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let end_span = self.expect(TokenKind::RParen)?.span;

        Ok(TraitMethod {
            span: start_span.merge(end_span),
            name,
            params,
            return_ty,
            default_impl,
        })
    }

    /// impl 定義のパース
    /// (impl (TraitName Type) (defn method [...] ...))
    fn parse_impl_def(&mut self, start_span: Span) -> Result<Decl, ParseError> {
        self.advance(); // impl をスキップ

        // (TraitName Type)
        self.expect(TokenKind::LParen)?;
        let trait_name = self.expect_symbol()?;
        let type_name = self.expect_symbol()?;
        self.expect(TokenKind::RParen)?;

        let mut methods = Vec::new();
        while !self.check(TokenKind::RParen) {
            let method_start = self.expect(TokenKind::LParen)?.span;
            let method = self.parse_defn(method_start)?;
            methods.push(method);
        }

        let end_span = self.expect(TokenKind::RParen)?.span;

        Ok(Decl::ImplDef {
            span: start_span.merge(end_span),
            trait_name,
            type_name,
            methods,
        })
    }

    /// バリアント:
    ///   Name                              -- 引数なし
    ///   (Name Type1 Type2 ...)            -- 通常 ADT
    ///   (: (Name Type1 ...) ReturnType)   -- GADT (戻り型指定)
    ///   (: Name ReturnType)               -- GADT (引数なし)
    fn parse_variant(&mut self) -> Result<Variant, ParseError> {
        if self.check(TokenKind::LParen) {
            let start_span = self.advance().span;

            // GADT 構文: (: <variant-form> <return-type>)
            if self.check(TokenKind::Colon) {
                self.advance(); // :
                // inner variant form (Name Type...) または bare Name
                let (name, fields) = if self.check(TokenKind::LParen) {
                    self.advance();
                    let name = self.expect_symbol()?;
                    let mut fields = Vec::new();
                    while !self.check(TokenKind::RParen) {
                        fields.push(self.parse_type_expr()?);
                    }
                    self.advance(); // )
                    (name, fields)
                } else {
                    (self.expect_symbol()?, Vec::new())
                };
                let ret_ty = self.parse_type_expr()?;
                let end_span = self.expect(TokenKind::RParen)?.span;
                return Ok(Variant {
                    span: start_span.merge(end_span),
                    name,
                    fields,
                    return_type: Some(ret_ty),
                });
            }

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
                return_type: None,
            })
        } else {
            let span = self.peek_span();
            let name = self.expect_symbol()?;
            Ok(Variant {
                span,
                name,
                fields: Vec::new(),
                return_type: None,
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
            self.advance();
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

    /// `:` の後がディレクティブキーワード（:where, :doc, :params 等）かを判定
    fn is_colon_directive(&self) -> bool {
        if !self.check(TokenKind::Colon) {
            return false;
        }
        match self.peek_at(1).map(|t| &t.kind) {
            Some(TokenKind::Where) => true,
            Some(TokenKind::Constraints) => true,
            Some(TokenKind::Symbol(s)) => matches!(
                s.as_str(),
                "where"
                    | "constraints"
                    | "intent"
                    | "claim"
                    | "assumption"
                    | "open-question"
                    | "motivates"
                    | "constrained-by"
                    | "tested-by"
                    | "supports"
                    | "contradicts"
                    | "evidence"
                    | "doc"
                    | "params"
                    | "returns"
                    | "rationale"
                    | "since"
                    | "see-also"
                    | "example"
                    | "invariant"
                    | "case"
                    | "assert"
                    | "property"
                    | "transitions"
            ),
            _ => false,
        }
    }

    // --- ヘルパーメソッド ---

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

    pub(super) fn parse_evidence_form(
        &mut self,
        id: String,
        id_span: Span,
    ) -> Result<(crate::metadata::EvidenceForm, Span), ParseError> {
        let mut subject = None;
        let mut method = None;
        let mut outcome = None;
        let mut runner = None;
        let mut target = None;
        let mut source_commit = None;
        let mut artifact_digest = None;
        let mut cases = None;
        let mut seed = None;
        let mut generator = None;
        let mut shrinks = None;
        let mut coverage = None;
        let mut producer = None;
        let mut tool_version = None;
        let mut timestamp = None;
        let mut independence = None;
        let mut end_span = id_span;

        while let Some(field) = self.peek_evidence_field_name() {
            self.advance(); // :field
            self.advance(); // field name
            end_span = match field.as_str() {
                "subject" => self.parse_evidence_string(&mut subject, "subject")?,
                "method" => self.parse_evidence_string(&mut method, "method")?,
                "outcome" => self.parse_evidence_string(&mut outcome, "outcome")?,
                "runner" => self.parse_evidence_string(&mut runner, "runner")?,
                "target" => self.parse_evidence_string(&mut target, "target")?,
                "source-commit" => {
                    self.parse_evidence_string(&mut source_commit, "source-commit")?
                }
                "artifact-digest" => {
                    self.parse_evidence_string(&mut artifact_digest, "artifact-digest")?
                }
                "cases" => self.parse_evidence_usize(&mut cases, "cases")?,
                "seed" => self.parse_evidence_u64(&mut seed, "seed")?,
                "generator" => self.parse_evidence_string(&mut generator, "generator")?,
                "shrinks" => self.parse_evidence_shrinks(&mut shrinks)?,
                "coverage" => self.parse_evidence_coverage(&mut coverage)?,
                "producer" => self.parse_evidence_string(&mut producer, "producer")?,
                "tool-version" => self.parse_evidence_string(&mut tool_version, "tool-version")?,
                "timestamp" => self.parse_evidence_string(&mut timestamp, "timestamp")?,
                "independence" => self.parse_evidence_string(&mut independence, "independence")?,
                _ => unreachable!("peek_evidence_field_name が未知 field を返した"),
            };
        }

        let record = crate::metadata::EvidenceForm::new(
            id,
            self.require_evidence_string(subject, "subject")?,
            self.require_evidence_string(method, "method")?,
            self.require_evidence_string(outcome, "outcome")?,
            self.require_evidence_string(runner, "runner")?,
            self.require_evidence_string(target, "target")?,
            self.require_evidence_string(source_commit, "source-commit")?,
            self.require_evidence_string(artifact_digest, "artifact-digest")?,
            self.require_evidence_usize(cases, "cases")?,
            self.require_evidence_u64(seed, "seed")?,
            self.require_evidence_string(generator, "generator")?,
            shrinks.unwrap_or_default(),
            coverage.unwrap_or_default(),
            self.require_evidence_string(producer, "producer")?,
            self.require_evidence_string(tool_version, "tool-version")?,
            self.require_evidence_string(timestamp, "timestamp")?,
            self.require_evidence_string(independence, "independence")?,
        );
        Ok((record, end_span))
    }

    fn peek_evidence_field_name(&self) -> Option<String> {
        if !self.check(TokenKind::Colon) {
            return None;
        }
        match self.peek_at(1).map(|token| &token.kind) {
            Some(TokenKind::Symbol(name)) if Self::is_evidence_field(name) => Some(name.clone()),
            _ => None,
        }
    }

    fn is_evidence_field(name: &str) -> bool {
        matches!(
            name,
            "subject"
                | "method"
                | "outcome"
                | "runner"
                | "target"
                | "source-commit"
                | "artifact-digest"
                | "cases"
                | "seed"
                | "generator"
                | "shrinks"
                | "coverage"
                | "producer"
                | "tool-version"
                | "timestamp"
                | "independence"
        )
    }

    fn parse_evidence_string(
        &mut self,
        slot: &mut Option<String>,
        field: &str,
    ) -> Result<Span, ParseError> {
        let (value, span) = self.expect_metadata_string(&format!(":evidence {field}"))?;
        if slot.replace(value).is_some() {
            return Err(ParseError::Unexpected {
                expected: format!("one :evidence {field}"),
                found: format!("duplicate :{field}"),
                span,
            });
        }
        Ok(span)
    }

    fn parse_evidence_usize(
        &mut self,
        slot: &mut Option<usize>,
        field: &str,
    ) -> Result<Span, ParseError> {
        let token = self.advance();
        let value = match token.kind {
            TokenKind::Int(value) if value >= 0 => {
                usize::try_from(value).map_err(|_| ParseError::Unexpected {
                    expected: format!(":evidence {field}"),
                    found: value.to_string(),
                    span: token.span,
                })?
            }
            kind => {
                return Err(ParseError::Unexpected {
                    expected: format!(":evidence {field}"),
                    found: kind.to_string(),
                    span: token.span,
                });
            }
        };
        if slot.replace(value).is_some() {
            return Err(ParseError::Unexpected {
                expected: format!("one :evidence {field}"),
                found: format!("duplicate :{field}"),
                span: token.span,
            });
        }
        Ok(token.span)
    }

    fn parse_evidence_u64(
        &mut self,
        slot: &mut Option<u64>,
        field: &str,
    ) -> Result<Span, ParseError> {
        let token = self.advance();
        let value = match token.kind {
            TokenKind::Int(value) if value >= 0 => {
                u64::try_from(value).map_err(|_| ParseError::Unexpected {
                    expected: format!(":evidence {field}"),
                    found: value.to_string(),
                    span: token.span,
                })?
            }
            kind => {
                return Err(ParseError::Unexpected {
                    expected: format!(":evidence {field}"),
                    found: kind.to_string(),
                    span: token.span,
                });
            }
        };
        if slot.replace(value).is_some() {
            return Err(ParseError::Unexpected {
                expected: format!("one :evidence {field}"),
                found: format!("duplicate :{field}"),
                span: token.span,
            });
        }
        Ok(token.span)
    }

    fn parse_evidence_shrinks(&mut self, slot: &mut Option<Vec<u64>>) -> Result<Span, ParseError> {
        let start = self.expect(TokenKind::LBracket)?.span;
        let mut values = Vec::new();
        while !self.check(TokenKind::RBracket) {
            if self.is_eof() {
                return Err(ParseError::UnexpectedEof {
                    expected: "]:evidence shrinks".to_string(),
                });
            }
            let token = self.advance();
            let value = match token.kind {
                TokenKind::Int(value) if value >= 0 => {
                    u64::try_from(value).map_err(|_| ParseError::Unexpected {
                        expected: ":evidence shrinks".to_string(),
                        found: value.to_string(),
                        span: token.span,
                    })?
                }
                kind => {
                    return Err(ParseError::Unexpected {
                        expected: ":evidence shrinks".to_string(),
                        found: kind.to_string(),
                        span: token.span,
                    });
                }
            };
            values.push(value);
        }
        let end = self.advance().span;
        if slot.replace(values).is_some() {
            return Err(ParseError::Unexpected {
                expected: "one :evidence shrinks".to_string(),
                found: "duplicate :shrinks".to_string(),
                span: start,
            });
        }
        Ok(start.merge(end))
    }

    fn parse_evidence_coverage(
        &mut self,
        slot: &mut Option<Vec<(String, usize)>>,
    ) -> Result<Span, ParseError> {
        let start = self.expect(TokenKind::LBracket)?.span;
        let mut values = Vec::new();
        while !self.check(TokenKind::RBracket) {
            if self.is_eof() {
                return Err(ParseError::UnexpectedEof {
                    expected: "]:evidence coverage".to_string(),
                });
            }
            self.expect(TokenKind::LParen)?;
            let bucket = self.expect_metadata_string("evidence coverage bucket")?.0;
            let token = self.advance();
            let count = match token.kind {
                TokenKind::Int(value) if value >= 0 => {
                    usize::try_from(value).map_err(|_| ParseError::Unexpected {
                        expected: ":evidence coverage".to_string(),
                        found: value.to_string(),
                        span: token.span,
                    })?
                }
                kind => {
                    return Err(ParseError::Unexpected {
                        expected: ":evidence coverage".to_string(),
                        found: kind.to_string(),
                        span: token.span,
                    });
                }
            };
            let end = self.expect(TokenKind::RParen)?.span;
            if values.iter().any(|(name, _)| name == &bucket) {
                return Err(ParseError::Unexpected {
                    expected: "unique :evidence coverage buckets".to_string(),
                    found: format!("duplicate coverage bucket {bucket}"),
                    span: end,
                });
            }
            values.push((bucket, count));
        }
        let end = self.advance().span;
        if slot.replace(values).is_some() {
            return Err(ParseError::Unexpected {
                expected: "one :evidence coverage".to_string(),
                found: "duplicate :coverage".to_string(),
                span: start,
            });
        }
        Ok(start.merge(end))
    }

    fn require_evidence_string(
        &self,
        value: Option<String>,
        field: &str,
    ) -> Result<String, ParseError> {
        value.ok_or_else(|| self.missing_evidence_field(field))
    }

    fn require_evidence_usize(
        &self,
        value: Option<usize>,
        field: &str,
    ) -> Result<usize, ParseError> {
        value.ok_or_else(|| self.missing_evidence_field(field))
    }

    fn require_evidence_u64(&self, value: Option<u64>, field: &str) -> Result<u64, ParseError> {
        value.ok_or_else(|| self.missing_evidence_field(field))
    }

    fn missing_evidence_field(&self, field: &str) -> ParseError {
        ParseError::Unexpected {
            expected: format!(":evidence {field}"),
            found: self
                .peek_kind()
                .map(|kind| kind.to_string())
                .unwrap_or_else(|| "EOF".to_string()),
            span: self.peek_span(),
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
mod metadata_tests;

#[cfg(test)]
mod pattern_tests;

#[cfg(test)]
mod transitions_tests;

#[cfg(test)]
mod computation_tests;
