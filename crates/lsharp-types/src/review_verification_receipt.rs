//! Rust の署名検証結果を native/provider 境界へ渡す explicit receipt。
//!
//! receipt は署名そのものを再検証する代替ではない。Rust が明示 trust store で署名を
//! `verified` とした事実と、その対象 attestation/trust snapshot/clock を native 側が
//! 取り違えずに受け取るための canonical handoff である。

use super::review_attestation::{
    validate_canonical_timestamp, AttestationAlgorithm, AttestationVerificationError,
    ReviewAttestation, ReviewVerificationState,
};
use super::review_trust_store::ReviewTrustStore;
use super::{ReviewId, StableIdError};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// verification receipt の署名対象 domain separator。
pub const VERIFICATION_RECEIPT_DOMAIN_SEPARATOR: &[u8] = b"lsharp.review-verification-receipt.v1\0";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReceiptError {
    #[error("review verification receipt の必須 field が空です: {field}")]
    EmptyField { field: &'static str },
    #[error("review verification receipt の review ID が不正です: {0}")]
    InvalidReviewId(#[from] StableIdError),
    #[error("review verification receipt の digest が不正です: field={field}, value={value:?}")]
    InvalidDigest { field: &'static str, value: String },
    #[error("review verification receipt の timestamp が不正です: value={value:?}")]
    InvalidTimestamp { value: String },
    #[error("review verification receipt は trusted key なしでは生成できません")]
    UntrustedKey,
    #[error(transparent)]
    Verification(#[from] AttestationVerificationError),
    #[error("review verification receipt の JSON projection に失敗しました: {message}")]
    Serialization { message: String },
}

/// Rust の verified signature fact と対象 snapshot identity を束ねる receipt。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewVerificationReceipt {
    review_id: ReviewId,
    provider: String,
    key_id: String,
    algorithm: AttestationAlgorithm,
    attestation_digest: String,
    trust_store_digest: String,
    verification_now: String,
}

impl ReviewVerificationReceipt {
    pub fn new(
        review_id: impl Into<String>,
        provider: impl Into<String>,
        key_id: impl Into<String>,
        algorithm: AttestationAlgorithm,
        attestation_digest: impl Into<String>,
        trust_store_digest: impl Into<String>,
        verification_now: impl Into<String>,
    ) -> Result<Self, ReceiptError> {
        let review_id = review_id.into();
        let provider = provider.into();
        let key_id = key_id.into();
        let attestation_digest = attestation_digest.into();
        let trust_store_digest = trust_store_digest.into();
        let verification_now = verification_now.into();
        validate_required("provider", &provider)?;
        validate_required("key_id", &key_id)?;
        validate_digest("attestation_digest", &attestation_digest)?;
        validate_digest("trust_store_digest", &trust_store_digest)?;
        validate_canonical_timestamp("verification_now", &verification_now).map_err(|_| {
            ReceiptError::InvalidTimestamp {
                value: verification_now.clone(),
            }
        })?;
        Ok(Self {
            review_id: ReviewId::parse(review_id).map_err(ReceiptError::InvalidReviewId)?,
            provider,
            key_id,
            algorithm,
            attestation_digest,
            trust_store_digest,
            verification_now,
        })
    }

    /// Rust が明示 trust store で署名を検証した後だけ receipt を作る。
    pub fn from_verified_signature(
        attestation: &ReviewAttestation,
        trust_store: &ReviewTrustStore,
        trust_store_digest: impl Into<String>,
        verification_now: impl Into<String>,
    ) -> Result<Self, ReceiptError> {
        match attestation.verify(trust_store)? {
            ReviewVerificationState::Verified => {}
            ReviewVerificationState::Unverified => return Err(ReceiptError::UntrustedKey),
            _ => return Err(ReceiptError::UntrustedKey),
        }
        let attestation_digest = digest_bytes(&attestation.canonical_bytes());
        Self::new(
            attestation.review_id().as_str(),
            attestation.provider(),
            attestation.key_id(),
            attestation.algorithm(),
            attestation_digest,
            trust_store_digest,
            verification_now,
        )
    }

    pub fn review_id(&self) -> &str {
        self.review_id.as_str()
    }

    pub fn provider(&self) -> &str {
        &self.provider
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub const fn algorithm(&self) -> AttestationAlgorithm {
        self.algorithm
    }

    pub const fn state(&self) -> &'static str {
        "verified"
    }

    pub fn attestation_digest(&self) -> &str {
        &self.attestation_digest
    }

    pub fn trust_store_digest(&self) -> &str {
        &self.trust_store_digest
    }

    pub fn verification_now(&self) -> &str {
        &self.verification_now
    }

    /// Rust/native が共有する deterministic receipt bytes。
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(VERIFICATION_RECEIPT_DOMAIN_SEPARATOR.len() + 256);
        bytes.extend_from_slice(VERIFICATION_RECEIPT_DOMAIN_SEPARATOR);
        for field in [
            self.review_id(),
            self.state(),
            self.provider(),
            self.key_id(),
            self.algorithm().as_str(),
            self.attestation_digest(),
            self.trust_store_digest(),
            self.verification_now(),
        ] {
            append_field(&mut bytes, field);
        }
        bytes
    }

    pub fn to_json_value(&self) -> Value {
        json!({
            "algorithm": self.algorithm().as_str(),
            "attestation_digest": self.attestation_digest(),
            "key_id": self.key_id(),
            "provider": self.provider(),
            "review_id": self.review_id(),
            "state": self.state(),
            "trust_store_digest": self.trust_store_digest(),
            "verification_now": self.verification_now(),
        })
    }

    pub fn to_json_string(&self) -> Result<String, ReceiptError> {
        serde_json::to_string(&self.to_json_value()).map_err(|error| ReceiptError::Serialization {
            message: error.to_string(),
        })
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), ReceiptError> {
    if value.trim().is_empty() {
        return Err(ReceiptError::EmptyField { field });
    }
    Ok(())
}

fn validate_digest(field: &'static str, value: &str) -> Result<(), ReceiptError> {
    let valid = value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
        && value[7..]
            .chars()
            .all(|character| !character.is_ascii_uppercase());
    if !valid {
        return Err(ReceiptError::InvalidDigest {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

fn digest_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::from("sha256:");
    for byte in digest {
        use std::fmt::Write;
        write!(&mut output, "{byte:02x}").expect("String は write 可能");
    }
    output
}

fn append_field(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
