//! v0.3 review provenance attestation の canonical input model。
//!
//! M2 の opaque `ReviewRecord` を置き換えず、review が何に対して発行されたかを
//! cross-target で再現可能な bytes へ固定する。provider からの lifecycle 取得や
//! expiry/subject の current snapshot 判定は外部 boundary の責務だが、明示された
//! lifecycle snapshot と署名の canonical gate はこの model で共有する。

use super::review_lifecycle::{ReviewLifecycleRegistry, ReviewLifecycleState};
use super::review_trust_store::ReviewTrustStore;
use super::{ReviewId, StableIdError};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// 署名対象の domain separator。
pub const ATTESTATION_DOMAIN_SEPARATOR: &[u8] = b"lsharp.review-attestation.v1\0";

/// v0.3 で明示的に許可する署名アルゴリズム。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttestationAlgorithm {
    Ed25519,
}

impl AttestationAlgorithm {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ed25519 => "ed25519",
        }
    }

    /// provider の wire value を allowlist 付きで復元する。
    pub fn parse(value: impl Into<String>) -> Result<Self, AttestationError> {
        let value = value.into();
        match value.as_str() {
            "ed25519" => Ok(Self::Ed25519),
            _ => Err(AttestationError::UnsupportedAlgorithm { value }),
        }
    }
}

/// attestation input が不正である理由。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AttestationError {
    #[error("review attestation の必須 field が空です: {field}")]
    EmptyField { field: &'static str },
    #[error("review attestation の署名が空です")]
    EmptySignature,
    #[error("review attestation の review ID が不正です: {0}")]
    InvalidReviewId(#[from] StableIdError),
    #[error("review attestation の algorithm が未対応です: {value:?}")]
    UnsupportedAlgorithm { value: String },
}

/// trusted key に対する signature verification error。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AttestationVerificationError {
    #[error("review attestation の signature length が不正です: actual={actual}")]
    InvalidSignatureLength { actual: usize },
    #[error("review attestation の trusted public key が不正です")]
    InvalidPublicKey,
    #[error("review attestation の signature encoding が不正です")]
    InvalidSignatureEncoding,
    #[error("review attestation の signature が canonical bytes と一致しません")]
    SignatureMismatch,
}

/// review attestation の検証状態。
///
/// `Invalid` は入力診断として扱い、その他の状態は manifest/report の fact として
/// 投影できる。M2 の independent review gate は `Verified` だけを数える。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewVerificationState {
    Verified,
    Unverified,
    Stale,
    Revoked,
    Invalid,
}

impl ReviewVerificationState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unverified => "unverified",
            Self::Stale => "stale",
            Self::Revoked => "revoked",
            Self::Invalid => "invalid",
        }
    }
}

/// review がどの graph/source に対して発行されたかを固定する attestation。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewAttestation {
    review_id: ReviewId,
    subject_digest: String,
    source_commit: String,
    provenance_digest: String,
    provider: String,
    key_id: String,
    algorithm: AttestationAlgorithm,
    issued_at: String,
    expires_at: Option<String>,
    sequence: u64,
    signature: Vec<u8>,
}

impl ReviewAttestation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        review_id: impl Into<String>,
        subject_digest: impl Into<String>,
        source_commit: impl Into<String>,
        provenance_digest: impl Into<String>,
        provider: impl Into<String>,
        key_id: impl Into<String>,
        algorithm: AttestationAlgorithm,
        issued_at: impl Into<String>,
        expires_at: Option<String>,
        sequence: u64,
        signature: impl Into<Vec<u8>>,
    ) -> Result<Self, AttestationError> {
        let review_id = review_id.into();
        let subject_digest = subject_digest.into();
        let source_commit = source_commit.into();
        let provenance_digest = provenance_digest.into();
        let provider = provider.into();
        let key_id = key_id.into();
        let issued_at = issued_at.into();
        let signature = signature.into();

        validate_required("review_id", &review_id)?;
        validate_required("subject_digest", &subject_digest)?;
        validate_required("source_commit", &source_commit)?;
        validate_required("provenance_digest", &provenance_digest)?;
        validate_required("provider", &provider)?;
        validate_required("key_id", &key_id)?;
        validate_required("issued_at", &issued_at)?;
        if let Some(expires_at) = &expires_at {
            validate_required("expires_at", expires_at)?;
        }
        if signature.is_empty() {
            return Err(AttestationError::EmptySignature);
        }

        let review_id = ReviewId::parse(review_id).map_err(AttestationError::InvalidReviewId)?;
        Ok(Self {
            review_id,
            subject_digest,
            source_commit,
            provenance_digest,
            provider,
            key_id,
            algorithm,
            issued_at,
            expires_at,
            sequence,
            signature,
        })
    }

    pub fn review_id(&self) -> &ReviewId {
        &self.review_id
    }

    pub fn subject_digest(&self) -> &str {
        &self.subject_digest
    }

    pub fn source_commit(&self) -> &str {
        &self.source_commit
    }

    pub fn provenance_digest(&self) -> &str {
        &self.provenance_digest
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

    pub fn issued_at(&self) -> &str {
        &self.issued_at
    }

    pub fn expires_at(&self) -> Option<&str> {
        self.expires_at.as_deref()
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn signature(&self) -> &[u8] {
        &self.signature
    }

    /// 明示された trust store にある Ed25519 key だけで signature を検証する。
    ///
    /// key が存在しない場合は暗黙に成功させず `Unverified` を返す。存在する key の
    /// signature mismatch/破損は入力境界として error にし、caller が report を成功扱い
    /// しないようにする。
    pub fn verify(
        &self,
        trust_store: &ReviewTrustStore,
    ) -> Result<ReviewVerificationState, AttestationVerificationError> {
        let Some(key) = trust_store.get(self.provider(), self.key_id(), self.algorithm()) else {
            return Ok(ReviewVerificationState::Unverified);
        };
        if self.signature.len() != 64 {
            return Err(AttestationVerificationError::InvalidSignatureLength {
                actual: self.signature.len(),
            });
        }
        let public_key: [u8; 32] = key
            .public_key()
            .try_into()
            .map_err(|_| AttestationVerificationError::InvalidPublicKey)?;
        let verifying_key = VerifyingKey::from_bytes(&public_key)
            .map_err(|_| AttestationVerificationError::InvalidPublicKey)?;
        let signature = Signature::from_slice(&self.signature)
            .map_err(|_| AttestationVerificationError::InvalidSignatureEncoding)?;
        verifying_key
            .verify(&self.canonical_bytes(), &signature)
            .map_err(|_| AttestationVerificationError::SignatureMismatch)?;
        Ok(ReviewVerificationState::Verified)
    }

    /// signature と explicit lifecycle snapshot の両方を満たした場合だけ verified にする。
    ///
    /// lifecycle がない、active でない、または attestation sequence と一致しない場合は
    /// 暗黙の成功にせず、report fact として保持できる状態へ降格する。wire/signature の
    /// 破損は `verify` と同じく error のまま返す。
    pub fn verify_with_lifecycle(
        &self,
        trust_store: &ReviewTrustStore,
        lifecycle: &ReviewLifecycleRegistry,
    ) -> Result<ReviewVerificationState, AttestationVerificationError> {
        let signature_state = self.verify(trust_store)?;
        if signature_state != ReviewVerificationState::Verified {
            return Ok(signature_state);
        }
        Ok(self.lifecycle_state(lifecycle))
    }

    /// signature、lifecycle、current subject/source/provenance の三つを同時に検証する。
    ///
    /// 署名が valid でも対象 snapshot のいずれかが異なれば `stale` とし、別 manifest や
    /// source commit の review を `verified` として再利用できないようにする。
    pub fn verify_against(
        &self,
        trust_store: &ReviewTrustStore,
        lifecycle: &ReviewLifecycleRegistry,
        subject_digest: &str,
        source_commit: &str,
        provenance_digest: &str,
    ) -> Result<ReviewVerificationState, AttestationVerificationError> {
        let signature_state = self.verify(trust_store)?;
        if signature_state != ReviewVerificationState::Verified {
            return Ok(signature_state);
        }
        if self.subject_digest() != subject_digest
            || self.source_commit() != source_commit
            || self.provenance_digest() != provenance_digest
        {
            return Ok(ReviewVerificationState::Stale);
        }
        Ok(self.lifecycle_state(lifecycle))
    }

    fn lifecycle_state(&self, lifecycle: &ReviewLifecycleRegistry) -> ReviewVerificationState {
        let Some(event) = lifecycle.current_event_for(self.review_id().as_str()) else {
            return ReviewVerificationState::Unverified;
        };
        match event.state() {
            ReviewLifecycleState::Proposed => ReviewVerificationState::Unverified,
            ReviewLifecycleState::Superseded => ReviewVerificationState::Stale,
            ReviewLifecycleState::Revoked => ReviewVerificationState::Revoked,
            ReviewLifecycleState::Active if event.sequence() == self.sequence() => {
                ReviewVerificationState::Verified
            }
            ReviewLifecycleState::Active => ReviewVerificationState::Stale,
        }
    }

    /// provider が持つ署名 bytes を差し替える。
    ///
    /// 署名は canonical bytes に含めないため、検証 fixture の署名だけを更新できる。
    /// constructor と同じ required-field 境界を setter でも維持する。
    pub fn set_signature(&mut self, signature: impl Into<Vec<u8>>) -> Result<(), AttestationError> {
        let signature = signature.into();
        if signature.is_empty() {
            return Err(AttestationError::EmptySignature);
        }
        self.signature = signature;
        Ok(())
    }

    pub fn set_expires_at(&mut self, expires_at: Option<String>) -> Result<(), AttestationError> {
        if let Some(value) = &expires_at {
            validate_required("expires_at", value)?;
        }
        self.expires_at = expires_at;
        Ok(())
    }

    /// 署名対象の canonical bytes を返す。
    ///
    /// domain separator の後に、設計書で定めた順序の UTF-8 field を big-endian u64 の
    /// byte length 付きで連結する。signature 自体は含めないため、provider の署名形式や
    /// key rotation によって対象 bytes が変わらない。
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(ATTESTATION_DOMAIN_SEPARATOR.len() + 256);
        bytes.extend_from_slice(ATTESTATION_DOMAIN_SEPARATOR);
        append_field(&mut bytes, self.review_id.as_str());
        append_field(&mut bytes, &self.subject_digest);
        append_field(&mut bytes, &self.source_commit);
        append_field(&mut bytes, &self.provenance_digest);
        append_field(&mut bytes, &self.provider);
        append_field(&mut bytes, &self.key_id);
        append_field(&mut bytes, self.algorithm.as_str());
        append_field(&mut bytes, &self.issued_at);
        append_field(&mut bytes, self.expires_at.as_deref().unwrap_or(""));
        append_field(&mut bytes, &self.sequence.to_string());
        bytes
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), AttestationError> {
    if value.trim().is_empty() {
        return Err(AttestationError::EmptyField { field });
    }
    Ok(())
}

fn append_field(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
