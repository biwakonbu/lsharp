use crate::ast::*;
use crate::metadata::{
    CaseExpectation, MetadataForm, MetadataFormKind, PropertyBinder, PropertyForm,
};
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

    /// メタデータのパース試行
    /// :doc "..." :params [(x "desc") ...] :returns "desc" など
    fn try_parse_metadata(&mut self) -> Result<Option<Metadata>, ParseError> {
        let mut metadata = Metadata::default();
        let mut found = false;

        loop {
            if !self.check(TokenKind::Colon) {
                break;
            }

            // 次のトークンがメタデータキーワードかチェック
            let next = self.peek_at(1).map(|t| t.kind.clone());
            match next {
                Some(TokenKind::Symbol(ref s)) => {
                    match s.as_str() {
                        "doc" => {
                            self.advance(); // :
                            self.advance(); // doc
                            if let Some(TokenKind::String(_)) = self.peek_kind() {
                                let tok = self.advance();
                                if let TokenKind::String(s) = tok.kind {
                                    metadata.doc = Some(s);
                                    found = true;
                                }
                            }
                        }
                        "params" => {
                            self.advance(); // :
                            self.advance(); // params
                            self.expect(TokenKind::LBracket)?;
                            while !self.check(TokenKind::RBracket) {
                                self.expect(TokenKind::LParen)?;
                                let param_name = self.expect_symbol()?;
                                let param_desc =
                                    if let Some(TokenKind::String(_)) = self.peek_kind() {
                                        let tok = self.advance();
                                        if let TokenKind::String(s) = tok.kind {
                                            s
                                        } else {
                                            String::new()
                                        }
                                    } else {
                                        String::new()
                                    };
                                self.expect(TokenKind::RParen)?;
                                metadata.params.push((param_name, param_desc));
                            }
                            self.advance(); // ]
                            found = true;
                        }
                        "returns" => {
                            self.advance(); // :
                            self.advance(); // returns
                            if let Some(TokenKind::String(_)) = self.peek_kind() {
                                let tok = self.advance();
                                if let TokenKind::String(s) = tok.kind {
                                    metadata.returns = Some(s);
                                    found = true;
                                }
                            }
                        }
                        "rationale" => {
                            self.advance(); // :
                            self.advance(); // rationale
                            if let Some(TokenKind::String(_)) = self.peek_kind() {
                                let tok = self.advance();
                                if let TokenKind::String(s) = tok.kind {
                                    metadata.rationale = Some(s);
                                    found = true;
                                }
                            }
                        }
                        "since" => {
                            self.advance(); // :
                            self.advance(); // since
                            if let Some(TokenKind::String(_)) = self.peek_kind() {
                                let tok = self.advance();
                                if let TokenKind::String(s) = tok.kind {
                                    metadata.since = Some(s);
                                    found = true;
                                }
                            }
                        }
                        "see-also" => {
                            self.advance(); // :
                            self.advance(); // see-also
                            self.expect(TokenKind::LBracket)?;
                            while !self.check(TokenKind::RBracket) {
                                metadata.see_also.push(self.expect_symbol()?);
                            }
                            self.advance(); // ]
                            found = true;
                        }
                        "example" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // example
                            self.expect(TokenKind::LBracket)?;
                            let mut expressions = Vec::new();
                            while !self.check(TokenKind::RBracket) {
                                expressions.push(self.parse_expr()?);
                            }
                            let form_end = self.advance().span; // ]
                            metadata.example.extend(expressions.iter().cloned());
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(form_end),
                                MetadataFormKind::LegacyExample { expressions },
                            ));
                            found = true;
                        }
                        "invariant" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // invariant
                            let predicate = self.parse_expr()?;
                            metadata.invariant = Some(predicate.clone());
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(predicate.span()),
                                MetadataFormKind::LegacyInvariant { predicate },
                            ));
                            found = true;
                        }
                        "case" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // case
                            self.expect(TokenKind::LBracket)?;
                            let mut expectations = Vec::new();
                            while !self.check(TokenKind::RBracket) {
                                expectations.push(self.parse_case_expectation()?);
                            }
                            let form_end = self.advance().span; // ]
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(form_end),
                                MetadataFormKind::Case { expectations },
                            ));
                            found = true;
                        }
                        "assert" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // assert
                            self.expect(TokenKind::LBracket)?;
                            let mut predicates = Vec::new();
                            while !self.check(TokenKind::RBracket) {
                                predicates.push(self.parse_expr()?);
                            }
                            let form_end = self.advance().span; // ]
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(form_end),
                                MetadataFormKind::Assertion { predicates },
                            ));
                            found = true;
                        }
                        "property" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // property
                            self.expect(TokenKind::LBracket)?;
                            let mut properties = Vec::new();
                            while !self.check(TokenKind::RBracket) {
                                properties.push(self.parse_property_form()?);
                            }
                            let form_end = self.advance().span; // ]
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(form_end),
                                MetadataFormKind::Property { properties },
                            ));
                            found = true;
                        }
                        "transitions" => {
                            // :transitions [(From -> To) ...]
                            self.advance(); // :
                            self.advance(); // transitions
                            self.expect(TokenKind::LBracket)?;
                            while !self.check(TokenKind::RBracket) {
                                self.expect(TokenKind::LParen)?;
                                let from = self.expect_symbol()?;
                                // -> 記号を読み飛ばす（Arrow トークン）
                                self.expect(TokenKind::Arrow)?;
                                let to = self.expect_symbol()?;
                                self.expect(TokenKind::RParen)?;
                                metadata.transitions.push((from, to));
                            }
                            self.advance(); // ]
                            found = true;
                        }
                        _ => break,
                    }
                }
                _ => break,
            }
        }

        Ok(if found { Some(metadata) } else { None })
    }

    fn parse_case_expectation(&mut self) -> Result<CaseExpectation, ParseError> {
        let entry_start = self.expect(TokenKind::LParen)?.span;
        let head_span = self.peek_span();
        let head = self.expect_symbol()?;
        if head != "expect" {
            return Err(ParseError::Unexpected {
                expected: "expect".to_string(),
                found: head,
                span: head_span,
            });
        }
        let actual = self.parse_expr()?;
        let expected = self.parse_expr()?;
        let entry_end = self.expect(TokenKind::RParen)?.span;
        Ok(CaseExpectation::new(
            entry_start.merge(entry_end),
            actual,
            expected,
        ))
    }

    fn parse_property_form(&mut self) -> Result<PropertyForm, ParseError> {
        let entry_start = self.expect(TokenKind::LParen)?.span;
        let head_span = self.peek_span();
        let head = self.expect_symbol()?;
        if head != "for-all" {
            return Err(ParseError::Unexpected {
                expected: "for-all".to_string(),
                found: head,
                span: head_span,
            });
        }

        self.expect(TokenKind::LBracket)?;
        let mut binders = Vec::new();
        while !self.check(TokenKind::RBracket) {
            let binder_start = self.peek_span();
            let name = self.expect_symbol()?;
            let ty = self.parse_type_expr()?;
            binders.push(PropertyBinder::new(
                binder_start.merge(type_expr_span(&ty)),
                name,
                ty,
            ));
        }
        self.advance(); // ]

        let mut preconditions = Vec::new();
        let mut postcondition = None;
        let mut cases = None;
        let mut seed = None;
        let mut shrink = None;
        while !self.check(TokenKind::RParen) {
            self.expect(TokenKind::Colon)?;
            let option_span = self.peek_span();
            let option = self.expect_symbol()?;
            match option.as_str() {
                "precondition" => {
                    self.expect(TokenKind::LBracket)?;
                    while !self.check(TokenKind::RBracket) {
                        preconditions.push(self.parse_expr()?);
                    }
                    self.advance(); // ]
                }
                "postcondition" => postcondition = Some(self.parse_expr()?),
                "cases" => cases = Some(self.parse_property_usize("non-negative case count")?),
                "seed" => seed = Some(self.parse_property_u64("non-negative seed")?),
                "shrink" => shrink = Some(self.parse_property_bool()?),
                _ => {
                    return Err(ParseError::Unexpected {
                        expected: "property option (precondition/postcondition/cases/seed/shrink)"
                            .to_string(),
                        found: option,
                        span: option_span,
                    });
                }
            }
        }
        let entry_end = self.advance().span; // )
        let postcondition = postcondition.ok_or_else(|| ParseError::Unexpected {
            expected: ":postcondition".to_string(),
            found: ")".to_string(),
            span: entry_end,
        })?;

        Ok(PropertyForm::new(
            entry_start.merge(entry_end),
            binders,
            preconditions,
            postcondition,
            cases,
            seed,
            shrink,
        ))
    }

    fn parse_property_usize(&mut self, expected: &str) -> Result<usize, ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Int(value) if value >= 0 => {
                usize::try_from(value).map_err(|_| ParseError::Unexpected {
                    expected: expected.to_string(),
                    found: value.to_string(),
                    span: token.span,
                })
            }
            kind => Err(ParseError::Unexpected {
                expected: expected.to_string(),
                found: kind.to_string(),
                span: token.span,
            }),
        }
    }

    fn parse_property_u64(&mut self, expected: &str) -> Result<u64, ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Int(value) if value >= 0 => {
                u64::try_from(value).map_err(|_| ParseError::Unexpected {
                    expected: expected.to_string(),
                    found: value.to_string(),
                    span: token.span,
                })
            }
            kind => Err(ParseError::Unexpected {
                expected: expected.to_string(),
                found: kind.to_string(),
                span: token.span,
            }),
        }
    }

    fn parse_property_bool(&mut self) -> Result<bool, ParseError> {
        let token = self.advance();
        match token.kind {
            TokenKind::Bool(value) => Ok(value),
            kind => Err(ParseError::Unexpected {
                expected: "Bool shrink flag".to_string(),
                found: kind.to_string(),
                span: token.span,
            }),
        }
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
        while !self.check(TokenKind::RParen) {
            variants.push(self.parse_variant()?);
        }
        let end_span = self.advance().span; // )

        // オプションのメタデータ（型定義後）はスキップ（外側の ) 前で処理済み）
        Ok(Decl::TypeDef {
            span: start_span.merge(end_span),
            name,
            type_params,
            variants,
            metadata: None,
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

        let end_span = self.expect(TokenKind::RParen)?.span; // (type を閉じる)

        Ok(Decl::RecordDef {
            span: start_span.merge(end_span),
            name,
            type_params,
            fields,
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
            && !name.contains('.')
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

    /// パターンをパース
    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
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
        assert_eq!(prog.to_string(), "(defn add [x y] : Int (+ x y))");
    }

    #[test]
    fn test_fib() {
        let prog = parse("(defn fib [n] (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))");
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

    // --- レコード型テスト ---

    #[test]
    fn test_record_type_def() {
        let prog = parse("(type Point (record (: x Float) (: y Float)))");
        assert_eq!(prog.decls.len(), 1);
        assert_eq!(
            prog.to_string(),
            "(type Point (record (: x Float) (: y Float)))"
        );
    }

    #[test]
    fn test_parametric_record_type_def() {
        let prog = parse("(type (Pair a b) (record (: fst a) (: snd b)))");
        assert_eq!(prog.decls.len(), 1);
        assert_eq!(
            prog.to_string(),
            "(type (Pair a b) (record (: fst a) (: snd b)))"
        );
    }

    #[test]
    fn test_record_literal() {
        let expr = parse_expr_str("{Point x 1.0 y 2.0}");
        assert_eq!(expr.to_string(), "{Point x 1 y 2}");
    }

    #[test]
    fn test_record_update() {
        let expr = parse_expr_str("{p | x 3.0}");
        assert_eq!(expr.to_string(), "{(p) | x 3}");
    }

    // --- 型エイリアステスト ---

    #[test]
    fn test_type_alias() {
        let prog = parse("(type-alias Str String)");
        assert_eq!(prog.decls.len(), 1);
        assert_eq!(prog.to_string(), "(type-alias Str String)");
    }

    #[test]
    fn test_parametric_type_alias() {
        let prog = parse("(type-alias (Callback a b) (-> a b))");
        assert_eq!(prog.decls.len(), 1);
    }

    // --- 制約付き型テスト ---

    #[test]
    fn test_type_constrained_basic() {
        let prog = parse("(type-constrained Natural Int :constraints [(>= 0)])");
        assert_eq!(prog.decls.len(), 1);
        if let Decl::TypeConstrained {
            name, constraints, ..
        } = &prog.decls[0]
        {
            assert_eq!(name, "Natural");
            assert_eq!(constraints.len(), 1);
        } else {
            panic!("Expected TypeConstrained");
        }
    }

    #[test]
    fn test_type_constrained_range() {
        let prog = parse("(type-constrained Percentage Int :constraints [(>= 0) (<= 100)])");
        assert_eq!(prog.decls.len(), 1);
        if let Decl::TypeConstrained {
            name, constraints, ..
        } = &prog.decls[0]
        {
            assert_eq!(name, "Percentage");
            assert_eq!(constraints.len(), 2);
        } else {
            panic!("Expected TypeConstrained");
        }
    }

    #[test]
    fn test_type_constrained_matches() {
        let prog =
            parse(r#"(type-constrained Email String :constraints [(matches "^[^@]+@[^@]+$")])"#);
        assert_eq!(prog.decls.len(), 1);
        if let Decl::TypeConstrained { constraints, .. } = &prog.decls[0] {
            assert_eq!(constraints.len(), 1);
            assert!(matches!(&constraints[0], Constraint::Matches(_)));
        } else {
            panic!("Expected TypeConstrained");
        }
    }

    #[test]
    fn test_type_constrained_satisfies() {
        let prog = parse("(type-constrained EvenInt Int :constraints [(satisfies is-even)])");
        assert_eq!(prog.decls.len(), 1);
        if let Decl::TypeConstrained { constraints, .. } = &prog.decls[0] {
            assert_eq!(constraints.len(), 1);
            assert!(matches!(&constraints[0], Constraint::Satisfies(_)));
        } else {
            panic!("Expected TypeConstrained");
        }
    }

    // --- where 制約テスト ---

    #[test]
    fn test_defn_with_where_clause() {
        let prog = parse("(defn show-it [x] :where [(Show a)] (show x))");
        assert_eq!(prog.decls.len(), 1);
        if let Decl::Defn { where_clauses, .. } = &prog.decls[0] {
            assert_eq!(where_clauses.len(), 1);
            assert_eq!(where_clauses[0].trait_name, "Show");
            assert_eq!(where_clauses[0].type_var, "a");
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_defn_with_multiple_where_clauses() {
        let prog = parse("(defn show-eq [x y] :where [(Show a) (Eq a)] (do (show x) (== x y)))");
        assert_eq!(prog.decls.len(), 1);
        if let Decl::Defn { where_clauses, .. } = &prog.decls[0] {
            assert_eq!(where_clauses.len(), 2);
        } else {
            panic!("Expected Defn");
        }
    }

    // --- メタデータテスト ---

    #[test]
    fn test_defn_with_metadata() {
        let prog = parse(r#"(defn add [x y] :doc "adds two numbers" (+ x y))"#);
        assert_eq!(prog.decls.len(), 1);
        if let Decl::Defn { metadata, .. } = &prog.decls[0] {
            assert!(metadata.is_some());
            let m = metadata.as_ref().unwrap();
            assert_eq!(m.doc.as_deref(), Some("adds two numbers"));
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_defn_with_params_metadata() {
        let prog = parse(
            r#"(defn add [x y] :doc "addition" :params [(x "left") (y "right")] :returns "sum" (+ x y))"#,
        );
        assert_eq!(prog.decls.len(), 1);
        if let Decl::Defn { metadata, .. } = &prog.decls[0] {
            let m = metadata.as_ref().unwrap();
            assert_eq!(m.params.len(), 2);
            assert_eq!(m.params[0].0, "x");
            assert_eq!(m.returns.as_deref(), Some("sum"));
        } else {
            panic!("Expected Defn");
        }
    }

    // --- モジュールテスト ---

    #[test]
    fn test_module_decl() {
        let prog = parse("(module MyModule)");
        assert_eq!(prog.decls.len(), 1);
        assert_eq!(prog.to_string(), "(module MyModule)");
    }

    #[test]
    fn test_nested_module_decl() {
        let prog = parse("(module Utils (defn helper [x] (+ x 1)))");
        assert_eq!(prog.decls.len(), 1);
        if let Decl::ModuleDecl { name, body, .. } = &prog.decls[0] {
            assert_eq!(name, "Utils");
            assert_eq!(body.len(), 1);
            assert!(matches!(&body[0], Decl::Defn { name, .. } if name == "helper"));
        } else {
            panic!("Expected ModuleDecl");
        }
    }

    #[test]
    fn test_nested_module_multiple_decls() {
        let prog = parse(
            "(module App.Utils
              (defn add [x y] (+ x y))
              (defn mul [x y] (* x y)))",
        );
        assert_eq!(prog.decls.len(), 1);
        if let Decl::ModuleDecl { name, body, .. } = &prog.decls[0] {
            assert_eq!(name, "App.Utils");
            assert_eq!(body.len(), 2);
        } else {
            panic!("Expected ModuleDecl");
        }
    }

    #[test]
    fn test_nested_module_with_types() {
        let prog = parse(
            "(module Models
              (type Point (record (: x Float) (: y Float)))
              (defn origin [] {Point x 0.0 y 0.0}))",
        );
        assert_eq!(prog.decls.len(), 1);
        if let Decl::ModuleDecl { name, body, .. } = &prog.decls[0] {
            assert_eq!(name, "Models");
            assert_eq!(body.len(), 2);
            assert!(matches!(&body[0], Decl::RecordDef { .. }));
            assert!(matches!(&body[1], Decl::Defn { .. }));
        } else {
            panic!("Expected ModuleDecl");
        }
    }

    #[test]
    fn test_nested_module_display() {
        let prog = parse("(module Utils (defn id [x] x))");
        assert_eq!(prog.to_string(), "(module Utils (defn id [x] x))");
    }

    #[test]
    fn test_deeply_nested_modules() {
        let prog = parse(
            "(module App
              (module Sub
                (defn inner [] 42)))",
        );
        assert_eq!(prog.decls.len(), 1);
        if let Decl::ModuleDecl { name, body, .. } = &prog.decls[0] {
            assert_eq!(name, "App");
            assert_eq!(body.len(), 1);
            if let Decl::ModuleDecl {
                name: inner_name,
                body: inner_body,
                ..
            } = &body[0]
            {
                assert_eq!(inner_name, "Sub");
                assert_eq!(inner_body.len(), 1);
            } else {
                panic!("Expected inner ModuleDecl");
            }
        } else {
            panic!("Expected ModuleDecl");
        }
    }

    #[test]
    fn test_import_decl() {
        let prog = parse("(import MyModule)");
        assert_eq!(prog.decls.len(), 1);
    }

    // --- トレイトテスト ---

    #[test]
    fn test_trait_def() {
        let prog = parse("(trait (Show a) (defn show [self] : String))");
        assert_eq!(prog.decls.len(), 1);
    }

    #[test]
    fn test_impl_def() {
        let prog = parse("(impl (Show Int) (defn show [self] (str self)))");
        assert_eq!(prog.decls.len(), 1);
    }

    // --- エラーリカバリテスト ---

    fn parse_result(input: &str) -> Result<Program, ParseError> {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    #[test]
    fn test_parse_multiple_errors() {
        // 2つの壊れた宣言がある入力で、両方のエラーが報告されることを確認
        let source = "(defn [) (defn [)";
        let result = parse_result(source);
        assert!(result.is_err());
        if let Err(ParseError::Multiple(errors)) = &result {
            assert!(
                errors.len() >= 2,
                "Expected at least 2 errors, got {}",
                errors.len()
            );
        } else {
            panic!("Expected Multiple error variant, got: {:?}", result);
        }
    }

    #[test]
    fn test_parse_recovery_after_error() {
        // 最初の宣言にエラーがあっても、2番目の宣言がパースされることを確認
        // parse_program は回復して継続するが、エラーがあるので Err を返す
        let source = "(defn [) (defn ok [] 42)";
        let result = parse_result(source);
        assert!(result.is_err());
        // 単一エラーが返ること（2番目はパース成功するため）
        assert!(
            !matches!(result, Err(ParseError::Multiple(_))),
            "Expected single error (second decl should parse ok), got: {:?}",
            result
        );
    }

    #[test]
    fn test_parse_recovery_returns_partial_result() {
        // エラーがあっても、正常な宣言はパースされることを確認
        // parse_program_recovering で部分的な結果を取得可能
        let source = "(defn [) (defn ok [] 42)";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let (prog, errors) = parser.parse_program_recovering();
        assert_eq!(errors.len(), 1, "Expected 1 error");
        assert_eq!(prog.decls.len(), 1, "Expected 1 successfully parsed decl");
    }

    #[test]
    fn test_parse_single_error_still_works() {
        // 1つだけエラーがある場合、従来通り単一のエラーが返ることを確認
        let source = "(unknown-form x)";
        let result = parse_result(source);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_no_error_unchanged() {
        // エラーがない場合は従来通り Ok が返ることを確認
        let source = "(defn foo [] 42) (defn bar [] 99)";
        let result = parse_result(source);
        assert!(result.is_ok());
        let prog = result.unwrap();
        assert_eq!(prog.decls.len(), 2);
    }

    // 深いネスト回帰テスト: parse_expr / parse_pattern / parse_type_expr が
    // 再帰で stack overflow しないことを検証する
    fn build_deep_if(depth: usize) -> String {
        let mut s = String::new();
        for _ in 0..depth {
            s.push_str("(if true ");
        }
        s.push('0');
        for _ in 0..depth {
            s.push_str(" 1)");
        }
        s
    }

    #[test]
    fn test_deep_nested_if_50() {
        let expr = parse_expr_str(&build_deep_if(50));
        // 壊れていないことだけ確認
        let _ = expr;
    }

    #[test]
    fn test_deep_nested_if_500() {
        let expr = parse_expr_str(&build_deep_if(500));
        let _ = expr;
    }

    #[test]
    fn test_gadt_variant_with_return_type() {
        // GADT: (: (IntLit Int) (Expr Int)) でバリアントの戻り型を指定
        let source = "(type (Expr a)
  (: (IntLit Int) (Expr Int))
  (: (BoolLit Bool) (Expr Bool)))";
        let prog = parse(source);
        assert_eq!(prog.decls.len(), 1);
        if let Decl::TypeDef { variants, .. } = &prog.decls[0] {
            assert_eq!(variants.len(), 2);
            assert!(
                variants[0].return_type.is_some(),
                "IntLit should have return_type"
            );
            assert!(
                variants[1].return_type.is_some(),
                "BoolLit should have return_type"
            );
        } else {
            panic!("expected TypeDef");
        }
    }

    #[test]
    fn test_deep_nested_if_2000() {
        // 2000 段は従来の再帰版だと overflow する深さ
        let expr = parse_expr_str(&build_deep_if(2000));
        let _ = expr;
    }
}

#[cfg(test)]
mod transitions_tests {
    use crate::ast::Decl;
    use crate::parse;

    #[test]
    fn test_transitions_metadata() {
        let prog = parse(
            r#"(defn open-door [door] :doc "Opens a door" :transitions [(Closed -> Open)] door)"#,
        )
        .unwrap();
        assert_eq!(prog.decls.len(), 1);
        if let Decl::Defn { metadata, .. } = &prog.decls[0] {
            let m = metadata.as_ref().unwrap();
            assert_eq!(m.transitions.len(), 1);
            assert_eq!(m.transitions[0], ("Closed".to_string(), "Open".to_string()));
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_multiple_transitions() {
        let prog = parse(
            r#"(defn toggle [state] :transitions [(Open -> Closed) (Closed -> Open)] state)"#,
        )
        .unwrap();
        if let Decl::Defn { metadata, .. } = &prog.decls[0] {
            let m = metadata.as_ref().unwrap();
            assert_eq!(m.transitions.len(), 2);
            assert_eq!(m.transitions[0], ("Open".to_string(), "Closed".to_string()));
            assert_eq!(m.transitions[1], ("Closed".to_string(), "Open".to_string()));
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_no_transitions() {
        let prog = parse(r#"(defn add [x y] :doc "add" (+ x y))"#).unwrap();
        if let Decl::Defn { metadata, .. } = &prog.decls[0] {
            let m = metadata.as_ref().unwrap();
            assert!(m.transitions.is_empty());
        } else {
            panic!("Expected Defn");
        }
    }
}

#[cfg(test)]
mod computation_tests {
    use super::Parser;
    use crate::ast::{ComputationStep, Decl, Expr};
    use crate::lexer::Lexer;
    use crate::parse;

    fn parse_expr_str(input: &str) -> Expr {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse_expr().unwrap()
    }

    #[test]
    fn test_computation_builder_decl() {
        let prog = parse("(computation-builder maybe maybe-bind maybe-return)").unwrap();
        assert_eq!(prog.decls.len(), 1);
        if let Decl::ComputationBuilder {
            name,
            bind_fn,
            return_fn,
            ..
        } = &prog.decls[0]
        {
            assert_eq!(name, "maybe");
            assert_eq!(bind_fn, "maybe-bind");
            assert_eq!(return_fn, "maybe-return");
        } else {
            panic!("Expected ComputationBuilder");
        }
    }

    #[test]
    fn test_computation_expr_basic() {
        let prog =
            parse("(defn test [] (computation maybe (let! x (get-value)) (return x)))").unwrap();
        if let Decl::Defn { body, .. } = &prog.decls[0] {
            if let Expr::Computation(_, builder, steps) = &body {
                assert_eq!(builder, "maybe");
                assert_eq!(steps.len(), 2);
                assert!(matches!(&steps[0], ComputationStep::LetBang(..)));
                assert!(matches!(&steps[1], ComputationStep::Return(..)));
            } else {
                panic!("Expected Computation expr, got: {body}");
            }
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_computation_expr_do_bang() {
        let prog = parse("(defn test [] (computation async (do! (print 1)) (return 42)))").unwrap();
        if let Decl::Defn { body, .. } = &prog.decls[0] {
            if let Expr::Computation(_, builder, steps) = &body {
                assert_eq!(builder, "async");
                assert_eq!(steps.len(), 2);
                assert!(matches!(&steps[0], ComputationStep::DoBang(..)));
                assert!(matches!(&steps[1], ComputationStep::Return(..)));
            } else {
                panic!("Expected Computation expr");
            }
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_computation_expr_with_plain_expr() {
        let prog =
            parse("(defn test [] (computation maybe (let! x (get-value)) (+ x 1)))").unwrap();
        if let Decl::Defn { body, .. } = &prog.decls[0] {
            if let Expr::Computation(_, _, steps) = &body {
                assert_eq!(steps.len(), 2);
                assert!(matches!(&steps[0], ComputationStep::LetBang(..)));
                assert!(matches!(&steps[1], ComputationStep::Expr(_)));
            } else {
                panic!("Expected Computation expr");
            }
        } else {
            panic!("Expected Defn");
        }
    }

    #[test]
    fn test_computation_display() {
        let prog =
            parse("(defn test [] (computation maybe (let! x (get-value)) (return x)))").unwrap();
        let display = format!("{}", prog.decls[0]);
        assert!(display.contains("maybe"));
    }

    // --- P10-1: Quote/Unquote テスト ---

    #[test]
    fn test_quote_simple() {
        let expr = parse_expr_str("'(+ 1 2)");
        assert!(matches!(expr, Expr::Quote(_, _)));
        assert_eq!(expr.to_string(), "'(+ 1 2)");
    }

    #[test]
    fn test_quote_symbol() {
        let expr = parse_expr_str("'x");
        assert!(matches!(expr, Expr::Quote(_, _)));
        assert_eq!(expr.to_string(), "'x");
    }

    #[test]
    fn test_unquote_simple() {
        let expr = parse_expr_str("~x");
        assert!(matches!(expr, Expr::Unquote(_, _)));
        assert_eq!(expr.to_string(), "~x");
    }

    #[test]
    fn test_splice_unquote() {
        let expr = parse_expr_str("~@args");
        assert!(matches!(expr, Expr::UnquoteSplice(_, _)));
        assert_eq!(expr.to_string(), "~@args");
    }

    #[test]
    fn test_quote_with_unquote() {
        // '(+ ~x 1) -- quote 内に unquote を含む
        let expr = parse_expr_str("'(+ ~x 1)");
        assert!(matches!(expr, Expr::Quote(_, _)));
        if let Expr::Quote(_, inner) = &expr {
            if let Expr::App(_, _, args) = inner.as_ref() {
                assert_eq!(args.len(), 2);
                assert!(matches!(&args[0], Expr::Unquote(_, _)));
            } else {
                panic!("Expected App inside Quote");
            }
        }
    }

    #[test]
    fn test_quote_with_splice_unquote() {
        // '(a ~@rest b)
        let expr = parse_expr_str("'(a ~@rest b)");
        assert!(matches!(expr, Expr::Quote(_, _)));
        if let Expr::Quote(_, inner) = &expr {
            if let Expr::App(_, _, args) = inner.as_ref() {
                assert!(args.iter().any(|a| matches!(a, Expr::UnquoteSplice(_, _))));
            } else {
                panic!("Expected App inside Quote");
            }
        }
    }

    // --- P10-2: DefMacro テスト ---

    #[test]
    fn test_defmacro_simple() {
        let prog = parse("(defmacro when [test body] '(if ~test ~body ()))").unwrap();
        assert_eq!(prog.decls.len(), 1);
        if let Decl::DefMacro { name, params, .. } = &prog.decls[0] {
            assert_eq!(name, "when");
            assert_eq!(params.len(), 2);
        } else {
            panic!("Expected DefMacro");
        }
    }

    #[test]
    fn test_defmacro_display() {
        let prog = parse("(defmacro unless [test body] '(if ~test () ~body))").unwrap();
        assert_eq!(
            prog.to_string(),
            "(defmacro unless [test body] '(if ~test () ~body))"
        );
    }
}
