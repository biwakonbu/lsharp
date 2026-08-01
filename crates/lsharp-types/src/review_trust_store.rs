//! v0.3 review attestation の explicit trust-store model。
//!
//! trust root は manifest/source に暗黙に埋め込まず、caller が明示的に渡した registry と
//! して扱う。ここでは key の identity/shape と duplicate を検査するが、署名検証や provider
//! からの取得は行わない。

use super::review_attestation::AttestationAlgorithm;
use std::collections::BTreeMap;

/// trusted public key の入力エラー。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TrustStoreError {
    #[error("review trust store の必須 field が空です: {field}")]
    EmptyField { field: &'static str },
    #[error("Ed25519 public key の長さが不正です: expected={expected}, actual={actual}")]
    InvalidPublicKeyLength { expected: usize, actual: usize },
    #[error(
        "review trust store に key が重複しています: provider={provider:?}, key_id={key_id:?}, algorithm={algorithm:?}"
    )]
    DuplicateKey {
        provider: String,
        key_id: String,
        algorithm: String,
    },
    #[error(
        "review trust store に active key が複数あります: provider={provider:?}, algorithm={algorithm:?}, existing_key_id={existing_key_id:?}, key_id={key_id:?}"
    )]
    MultipleActiveKeys {
        provider: String,
        algorithm: String,
        existing_key_id: String,
        key_id: String,
    },
}

/// provider/key ID に束ねた Ed25519 public key。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewTrustKey {
    provider: String,
    key_id: String,
    algorithm: AttestationAlgorithm,
    public_key: Vec<u8>,
    active: bool,
}

impl ReviewTrustKey {
    pub fn new(
        provider: impl Into<String>,
        key_id: impl Into<String>,
        algorithm: AttestationAlgorithm,
        public_key: impl Into<Vec<u8>>,
    ) -> Result<Self, TrustStoreError> {
        let provider = provider.into();
        let key_id = key_id.into();
        let public_key = public_key.into();
        validate_required("provider", &provider)?;
        validate_required("key_id", &key_id)?;
        if public_key.len() != 32 {
            return Err(TrustStoreError::InvalidPublicKeyLength {
                expected: 32,
                actual: public_key.len(),
            });
        }
        Ok(Self {
            provider,
            key_id,
            algorithm,
            public_key,
            active: true,
        })
    }

    /// key rotation中の retired key を明示する。
    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
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

    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }

    pub const fn is_active(&self) -> bool {
        self.active
    }
}

/// caller が明示的に渡した trusted key の deterministic registry。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewTrustStore {
    keys: BTreeMap<(String, String, String), ReviewTrustKey>,
}

impl ReviewTrustStore {
    pub fn add_key(&mut self, key: ReviewTrustKey) -> Result<(), TrustStoreError> {
        let identity = (
            key.provider().to_string(),
            key.key_id().to_string(),
            key.algorithm().as_str().to_string(),
        );
        if self.keys.contains_key(&identity) {
            return Err(TrustStoreError::DuplicateKey {
                provider: identity.0,
                key_id: identity.1,
                algorithm: identity.2,
            });
        }
        if key.is_active() {
            if let Some(existing) = self.keys.values().find(|existing| {
                existing.is_active()
                    && existing.provider() == key.provider()
                    && existing.algorithm() == key.algorithm()
            }) {
                return Err(TrustStoreError::MultipleActiveKeys {
                    provider: key.provider().to_string(),
                    algorithm: key.algorithm().as_str().to_string(),
                    existing_key_id: existing.key_id().to_string(),
                    key_id: key.key_id().to_string(),
                });
            }
        }
        self.keys.insert(identity, key);
        Ok(())
    }

    pub fn contains(&self, provider: &str, key_id: &str, algorithm: AttestationAlgorithm) -> bool {
        self.keys.contains_key(&(
            provider.to_string(),
            key_id.to_string(),
            algorithm.as_str().to_string(),
        ))
    }

    pub fn get(
        &self,
        provider: &str,
        key_id: &str,
        algorithm: AttestationAlgorithm,
    ) -> Option<&ReviewTrustKey> {
        self.keys.get(&(
            provider.to_string(),
            key_id.to_string(),
            algorithm.as_str().to_string(),
        ))
    }

    /// provider/algorithmごとの現在の active key を deterministic に選択する。
    pub fn active_key(
        &self,
        provider: &str,
        algorithm: AttestationAlgorithm,
    ) -> Option<&ReviewTrustKey> {
        self.keys.values().find(|key| {
            key.is_active() && key.provider() == provider && key.algorithm() == algorithm
        })
    }

    pub fn entries(&self) -> Vec<&ReviewTrustKey> {
        self.keys.values().collect()
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), TrustStoreError> {
    if value.trim().is_empty() {
        return Err(TrustStoreError::EmptyField { field });
    }
    Ok(())
}
