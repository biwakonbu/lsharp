use serde::Serialize;

/// review verification を再現するために caller が明示した evidence identity。
///
/// source/manifest の内容から暗黙に artifact や trust root を推測せず、実行時に渡された
/// digest だけを report の fact として残す。trust/lifecycle が省略された場合も `None` を
/// wire へ明示し、未検証の入力を verified と誤認させない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewEvidenceIdentity {
    subject_digest: String,
    source_commit: String,
    artifact_digest: String,
    now: String,
    trust_store_digest: Option<String>,
    lifecycle_digest: Option<String>,
}

impl ReviewEvidenceIdentity {
    pub fn new(
        subject_digest: impl Into<String>,
        source_commit: impl Into<String>,
        artifact_digest: impl Into<String>,
        now: impl Into<String>,
        trust_store_digest: Option<String>,
        lifecycle_digest: Option<String>,
    ) -> Result<Self, ReviewEvidenceIdentityError> {
        let subject_digest = subject_digest.into();
        let source_commit = source_commit.into();
        let artifact_digest = artifact_digest.into();
        let now = now.into();
        validate_identity_field("subject_digest", &subject_digest)?;
        validate_identity_field("source_commit", &source_commit)?;
        validate_identity_field("artifact_digest", &artifact_digest)?;
        validate_identity_field("now", &now)?;
        if let Some(value) = &trust_store_digest {
            validate_identity_field("trust_store_digest", value)?;
        }
        if let Some(value) = &lifecycle_digest {
            validate_identity_field("lifecycle_digest", value)?;
        }
        Ok(Self {
            subject_digest,
            source_commit,
            artifact_digest,
            now,
            trust_store_digest,
            lifecycle_digest,
        })
    }

    pub fn subject_digest(&self) -> &str {
        &self.subject_digest
    }

    pub fn source_commit(&self) -> &str {
        &self.source_commit
    }

    pub fn artifact_digest(&self) -> &str {
        &self.artifact_digest
    }

    pub fn now(&self) -> &str {
        &self.now
    }

    pub fn trust_store_digest(&self) -> Option<&str> {
        self.trust_store_digest.as_deref()
    }

    pub fn lifecycle_digest(&self) -> Option<&str> {
        self.lifecycle_digest.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReviewEvidenceIdentityError {
    #[error("review evidence identity の必須 field が空です: {field}")]
    EmptyField { field: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReviewEvidenceIdentityProjectionError {
    #[error("review evidence identity が既存 manifest と一致しません")]
    Conflict,
}

fn validate_identity_field(
    field: &'static str,
    value: &str,
) -> Result<(), ReviewEvidenceIdentityError> {
    if value.trim().is_empty() {
        return Err(ReviewEvidenceIdentityError::EmptyField { field });
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub(crate) struct ReviewEvidenceIdentityWire {
    subject_digest: String,
    source_commit: String,
    artifact_digest: String,
    trust_store_digest: Option<String>,
    lifecycle_digest: Option<String>,
    now: String,
}

impl ReviewEvidenceIdentityWire {
    pub(crate) fn from_identity(identity: &ReviewEvidenceIdentity) -> Self {
        Self {
            subject_digest: identity.subject_digest().to_string(),
            source_commit: identity.source_commit().to_string(),
            artifact_digest: identity.artifact_digest().to_string(),
            trust_store_digest: identity.trust_store_digest().map(ToOwned::to_owned),
            lifecycle_digest: identity.lifecycle_digest().map(ToOwned::to_owned),
            now: identity.now().to_string(),
        }
    }
}
