use crate::ast::Metadata;
use crate::metadata::{
    CaseExpectation, MetadataForm, MetadataFormKind, PropertyBinder, PropertyForm,
};
use crate::token::TokenKind;

use super::{ParseError, Parser, type_expr_span};

impl Parser {
    /// メタデータのパース試行
    /// :doc "..." :params [(x "desc") ...] :returns "desc" など
    pub(super) fn try_parse_metadata(&mut self) -> Result<Option<Metadata>, ParseError> {
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
                        "intent" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // intent
                            let (id, _) = self.expect_metadata_string("intent stable ID")?;
                            let (text, text_span) = self.expect_metadata_string("intent text")?;
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(text_span),
                                MetadataFormKind::Intent { id, text },
                            ));
                            found = true;
                        }
                        "claim" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // claim
                            let (id, _) = self.expect_metadata_string("claim stable ID")?;
                            let (text, text_span) = self.expect_metadata_string("claim text")?;
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(text_span),
                                MetadataFormKind::Claim { id, text },
                            ));
                            found = true;
                        }
                        "assumption" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // assumption
                            let (id, _) = self.expect_metadata_string("assumption stable ID")?;
                            let (text, text_span) =
                                self.expect_metadata_string("assumption text")?;
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(text_span),
                                MetadataFormKind::Assumption { id, text },
                            ));
                            found = true;
                        }
                        "open-question" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // open-question
                            let (id, _) = self.expect_metadata_string("open-question stable ID")?;
                            let (text, text_span) =
                                self.expect_metadata_string("open-question text")?;
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(text_span),
                                MetadataFormKind::OpenQuestion { id, text },
                            ));
                            found = true;
                        }
                        "motivates" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // motivates
                            let (intent, _) = self.expect_metadata_string("motivates intent ID")?;
                            let (claim, claim_span) =
                                self.expect_metadata_string("motivates claim ID")?;
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(claim_span),
                                MetadataFormKind::Motivates { intent, claim },
                            ));
                            found = true;
                        }
                        "constrained-by" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // constrained-by
                            let (claim, _) =
                                self.expect_metadata_string("constrained-by claim ID")?;
                            let (assumption, assumption_span) =
                                self.expect_metadata_string("constrained-by assumption ID")?;
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(assumption_span),
                                MetadataFormKind::ConstrainedBy { claim, assumption },
                            ));
                            found = true;
                        }
                        "tested-by" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // tested-by
                            let (claim, _) = self.expect_metadata_string("tested-by claim ID")?;
                            let (contract, contract_span) =
                                self.expect_metadata_string("tested-by contract ID")?;
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(contract_span),
                                MetadataFormKind::TestedBy { claim, contract },
                            ));
                            found = true;
                        }
                        "supports" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // supports
                            let (observation, _) =
                                self.expect_metadata_string("supports observation ID")?;
                            let (claim, claim_span) =
                                self.expect_metadata_string("supports claim ID")?;
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(claim_span),
                                MetadataFormKind::Supports { observation, claim },
                            ));
                            found = true;
                        }
                        "contradicts" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // contradicts
                            let (observation, _) =
                                self.expect_metadata_string("contradicts observation ID")?;
                            let (claim, claim_span) =
                                self.expect_metadata_string("contradicts claim ID")?;
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(claim_span),
                                MetadataFormKind::Contradicts { observation, claim },
                            ));
                            found = true;
                        }
                        "evidence" => {
                            let form_start = self.peek_span();
                            self.advance(); // :
                            self.advance(); // evidence
                            let (id, id_span) =
                                self.expect_metadata_string("evidence stable ID")?;
                            let (record, end_span) = self.parse_evidence_form(id, id_span)?;
                            metadata.forms.push(MetadataForm::new(
                                form_start.merge(end_span),
                                MetadataFormKind::Evidence {
                                    record: Box::new(record),
                                },
                            ));
                            found = true;
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
}
