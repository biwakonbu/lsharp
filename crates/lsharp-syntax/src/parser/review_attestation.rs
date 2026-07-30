use crate::span::Span;
use crate::token::TokenKind;

use super::{ParseError, Parser};

impl Parser {
    pub(super) fn parse_review_attestation_form(
        &mut self,
    ) -> Result<(crate::metadata::ReviewAttestationForm, Span), ParseError> {
        let mut review_id = None;
        let mut subject_digest = None;
        let mut source_commit = None;
        let mut provenance_digest = None;
        let mut provider = None;
        let mut key_id = None;
        let mut algorithm = None;
        let mut signature = None;
        let mut issued_at = None;
        let mut expires_at = None;
        let mut expires_at_seen = false;
        let mut sequence = None;
        let mut end_span = self.peek_span();

        while self.check(TokenKind::Colon) {
            let Some(TokenKind::Symbol(field)) = self.peek_at(1).map(|token| token.kind.clone())
            else {
                break;
            };
            if !Self::is_review_attestation_field(&field) {
                return Err(ParseError::Unexpected {
                    expected: "review-attestation named field".to_string(),
                    found: format!(":{field}"),
                    span: self.peek_span(),
                });
            }
            self.advance(); // :field
            self.advance(); // field name
            match field.as_str() {
                "review-id" => {
                    end_span = self.parse_review_attestation_string(&mut review_id, "review-id")?;
                }
                "subject-digest" => {
                    end_span = self
                        .parse_review_attestation_string(&mut subject_digest, "subject-digest")?;
                }
                "source-commit" => {
                    end_span =
                        self.parse_review_attestation_string(&mut source_commit, "source-commit")?;
                }
                "provenance-digest" => {
                    end_span = self.parse_review_attestation_string(
                        &mut provenance_digest,
                        "provenance-digest",
                    )?;
                }
                "provider" => {
                    end_span = self.parse_review_attestation_string(&mut provider, "provider")?;
                }
                "key-id" => {
                    end_span = self.parse_review_attestation_string(&mut key_id, "key-id")?;
                }
                "algorithm" => {
                    end_span = self.parse_review_attestation_string(&mut algorithm, "algorithm")?;
                }
                "signature" => {
                    end_span = self.parse_review_attestation_string(&mut signature, "signature")?;
                }
                "issued-at" => {
                    end_span = self.parse_review_attestation_string(&mut issued_at, "issued-at")?;
                }
                "expires-at" => {
                    if expires_at_seen {
                        return Err(ParseError::Unexpected {
                            expected: "one :review-attestation expires-at".to_string(),
                            found: "duplicate :expires-at".to_string(),
                            span: self.peek_span(),
                        });
                    }
                    expires_at_seen = true;
                    end_span =
                        self.parse_review_attestation_string(&mut expires_at, "expires-at")?;
                }
                "sequence" => {
                    end_span = self.parse_review_attestation_sequence(&mut sequence)?;
                }
                _ => unreachable!("is_review_attestation_field が未知 field を返した"),
            }
        }

        let attestation = crate::metadata::ReviewAttestationForm::new(
            Self::require_review_attestation_string(review_id, "review-id")?,
            Self::require_review_attestation_string(subject_digest, "subject-digest")?,
            Self::require_review_attestation_string(source_commit, "source-commit")?,
            Self::require_review_attestation_string(provenance_digest, "provenance-digest")?,
            Self::require_review_attestation_string(provider, "provider")?,
            Self::require_review_attestation_string(key_id, "key-id")?,
            Self::require_review_attestation_string(algorithm, "algorithm")?,
            Self::require_review_attestation_string(signature, "signature")?,
            Self::require_review_attestation_string(issued_at, "issued-at")?,
            expires_at,
            sequence.ok_or_else(|| ParseError::Unexpected {
                expected: ":review-attestation sequence".to_string(),
                found: "missing".to_string(),
                span: end_span,
            })?,
        );
        Ok((attestation, end_span))
    }

    fn is_review_attestation_field(name: &str) -> bool {
        matches!(
            name,
            "review-id"
                | "subject-digest"
                | "source-commit"
                | "provenance-digest"
                | "provider"
                | "key-id"
                | "algorithm"
                | "signature"
                | "issued-at"
                | "expires-at"
                | "sequence"
        )
    }

    fn parse_review_attestation_string(
        &mut self,
        slot: &mut Option<String>,
        field: &str,
    ) -> Result<Span, ParseError> {
        let (value, span) = self.expect_metadata_string(&format!(":review-attestation {field}"))?;
        if slot.replace(value).is_some() {
            return Err(ParseError::Unexpected {
                expected: format!("one :review-attestation {field}"),
                found: format!("duplicate :{field}"),
                span,
            });
        }
        Ok(span)
    }

    fn parse_review_attestation_sequence(
        &mut self,
        slot: &mut Option<u64>,
    ) -> Result<Span, ParseError> {
        let token = self.advance();
        let value = match token.kind {
            TokenKind::Int(value) if value >= 0 => {
                u64::try_from(value).map_err(|_| ParseError::Unexpected {
                    expected: ":review-attestation sequence".to_string(),
                    found: value.to_string(),
                    span: token.span,
                })?
            }
            kind => {
                return Err(ParseError::Unexpected {
                    expected: ":review-attestation sequence".to_string(),
                    found: kind.to_string(),
                    span: token.span,
                });
            }
        };
        if slot.replace(value).is_some() {
            return Err(ParseError::Unexpected {
                expected: "one :review-attestation sequence".to_string(),
                found: "duplicate :sequence".to_string(),
                span: token.span,
            });
        }
        Ok(token.span)
    }

    fn require_review_attestation_string(
        value: Option<String>,
        field: &str,
    ) -> Result<String, ParseError> {
        value.ok_or_else(|| ParseError::Unexpected {
            expected: format!(":review-attestation {field}"),
            found: "missing".to_string(),
            span: Span::dummy(),
        })
    }
}
