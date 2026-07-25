use crate::span::Span;
use crate::token::TokenKind;

use super::{ParseError, Parser};

impl Parser {
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
}
