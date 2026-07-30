//! v0.3 review trust/lifecycle input の explicit file boundary。
//!
//! `validate` の review verification input は caller が指定した project-relative file だけを
//! 読む。current manifest、環境変数、暗黙の default から trust root を補わず、symlink を含む
//! project root 外の path と review wire の schema violation を fail-closed にする。

use lsharp_types::intent::review_attestation::{
    AttestationVerificationError, ReviewAttestation, ReviewVerificationState,
};
use lsharp_types::intent::review_lifecycle::ReviewLifecycleRegistry;
use lsharp_types::intent::review_trust_store::ReviewTrustStore;
use lsharp_types::intent::review_wire::parse_review_wire;
use lsharp_types::validation::{ReviewVerificationFact, ReviewVerificationProjectionError};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct ReviewInputs {
    pub trust_store: Option<ReviewTrustStore>,
    pub lifecycle: Option<ReviewLifecycleRegistry>,
    /// `--trust-store` が指す version 1 wire に明示された attestation。
    ///
    /// trust root と同じ project-relative input から読み取るが、検証 state の生成は
    /// current graph/source/clock を持つ後続 caller の責務とする。
    pub attestations: Vec<ReviewAttestation>,
}

impl ReviewInputs {
    /// 明示 input から report/manifest 共通の verification fact を生成する。
    ///
    /// trust store または lifecycle snapshot が欠ける場合は、署名を暗黙に成功扱いせず
    /// `unverified` とする。provider、environment、system clock から補完しない。
    pub fn verification_facts(&self) -> Result<Vec<ReviewVerificationFact>, ReviewInputError> {
        self.attestations
            .iter()
            .map(|attestation| {
                let state = match (&self.trust_store, &self.lifecycle) {
                    (Some(trust_store), Some(lifecycle)) => attestation
                        .verify_with_lifecycle(trust_store, lifecycle)
                        .map_err(|source| ReviewInputError::Verification {
                            review_id: attestation.review_id().as_str().to_string(),
                            source,
                        })?,
                    (Some(trust_store), None) => {
                        let signature_state =
                            attestation.verify(trust_store).map_err(|source| {
                                ReviewInputError::Verification {
                                    review_id: attestation.review_id().as_str().to_string(),
                                    source,
                                }
                            })?;
                        if signature_state == ReviewVerificationState::Verified {
                            ReviewVerificationState::Unverified
                        } else {
                            signature_state
                        }
                    }
                    _ => ReviewVerificationState::Unverified,
                };
                ReviewVerificationFact::new(attestation.review_id().clone(), state)
                    .map_err(ReviewInputError::Projection)
            })
            .collect()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ReviewInputError {
    #[error("review {kind} path の読み込みに失敗しました: {path}: {message}")]
    Path {
        kind: &'static str,
        path: String,
        message: String,
    },
    #[error("review {kind} wire の読み込みに失敗しました: {path}: {message}")]
    Wire {
        kind: &'static str,
        path: String,
        message: String,
    },
    #[error("review trust store input に trust_store field がありません: {path}")]
    MissingTrustStore { path: String },
    #[error("review attestation の検証に失敗しました: id={review_id}: {source}")]
    Verification {
        review_id: String,
        #[source]
        source: AttestationVerificationError,
    },
    #[error("review verification fact の projection に失敗しました: {0}")]
    Projection(#[from] ReviewVerificationProjectionError),
}

/// 明示 review input を project root 内の通常ファイルへ解決して parse する。
pub fn load_review_inputs(
    project_root: &Path,
    trust_store_path: Option<&Path>,
    lifecycle_path: Option<&Path>,
) -> Result<ReviewInputs, ReviewInputError> {
    let (trust_store, attestations) = match trust_store_path {
        Some(path) => {
            let (trust_store, attestations) = load_trust_store(project_root, path)?;
            (Some(trust_store), attestations)
        }
        None => (None, Vec::new()),
    };
    let lifecycle = lifecycle_path
        .map(|path| load_lifecycle(project_root, path))
        .transpose()?;
    Ok(ReviewInputs {
        trust_store,
        lifecycle,
        attestations,
    })
}

fn load_trust_store(
    project_root: &Path,
    configured: &Path,
) -> Result<(ReviewTrustStore, Vec<ReviewAttestation>), ReviewInputError> {
    let resolved = resolve_review_input_path(project_root, configured, "trust store")?;
    let document = read_wire(&resolved, "trust store")?;
    let trust_store =
        document
            .trust_store()
            .cloned()
            .ok_or_else(|| ReviewInputError::MissingTrustStore {
                path: resolved.display().to_string(),
            })?;
    Ok((trust_store, document.attestations().to_vec()))
}

fn load_lifecycle(
    project_root: &Path,
    configured: &Path,
) -> Result<ReviewLifecycleRegistry, ReviewInputError> {
    let resolved = resolve_review_input_path(project_root, configured, "lifecycle")?;
    Ok(read_wire(&resolved, "lifecycle")?.lifecycle().clone())
}

fn read_wire(
    path: &Path,
    kind: &'static str,
) -> Result<lsharp_types::intent::review_wire::ReviewWireDocument, ReviewInputError> {
    let source = std::fs::read_to_string(path).map_err(|error| ReviewInputError::Wire {
        kind,
        path: path.display().to_string(),
        message: error.to_string(),
    })?;
    parse_review_wire(&source).map_err(|error| ReviewInputError::Wire {
        kind,
        path: path.display().to_string(),
        message: error.to_string(),
    })
}

fn resolve_review_input_path(
    project_root: &Path,
    configured: &Path,
    kind: &'static str,
) -> Result<PathBuf, ReviewInputError> {
    if configured.as_os_str().is_empty() {
        return Err(ReviewInputError::Path {
            kind,
            path: configured.display().to_string(),
            message: "空 path は指定できません".to_string(),
        });
    }
    if configured.is_absolute() {
        return Err(ReviewInputError::Path {
            kind,
            path: configured.display().to_string(),
            message: "project-relative path が必要です".to_string(),
        });
    }
    if configured
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(ReviewInputError::Path {
            kind,
            path: configured.display().to_string(),
            message: "project root 外への '..' は指定できません".to_string(),
        });
    }

    let project_root = project_root
        .canonicalize()
        .map_err(|error| ReviewInputError::Path {
            kind,
            path: project_root.display().to_string(),
            message: format!("project root の解決に失敗しました: {error}"),
        })?;
    let candidate = project_root.join(configured);
    let resolved = candidate
        .canonicalize()
        .map_err(|error| ReviewInputError::Path {
            kind,
            path: candidate.display().to_string(),
            message: format!("file が見つかりません: {error}"),
        })?;
    if !resolved.starts_with(&project_root) {
        return Err(ReviewInputError::Path {
            kind,
            path: configured.display().to_string(),
            message: "project root 外を指せません".to_string(),
        });
    }
    if !resolved.is_file() {
        return Err(ReviewInputError::Path {
            kind,
            path: resolved.display().to_string(),
            message: "通常の file を指定してください".to_string(),
        });
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn wire_with_attestation() -> String {
        r#"{
          "schema_version": 1,
          "attestations": [{
            "review_id": "review:checkout/reviewer-001",
            "subject_digest": "sha256:graph",
            "source_commit": "commit-1",
            "provenance_digest": "sha256:review",
            "provider": "github",
            "key_id": "org/reviews-2026",
            "algorithm": "ed25519",
            "signature": "AQID",
            "issued_at": "2026-07-29T00:00:00Z",
            "sequence": 1
          }],
          "lifecycle": [],
          "trust_store": [{
            "provider": "github",
            "key_id": "org/reviews-2026",
            "algorithm": "ed25519",
            "public_key": "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc"
          }]
        }"#
        .to_string()
    }

    #[test]
    fn explicit_wire_attestations_are_retained_for_verification_projection() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!("lsharp-review-input-test-{nonce}"));
        std::fs::create_dir_all(&project_root).expect("project root should be writable");
        let wire_path = project_root.join("trust.json");
        std::fs::write(&wire_path, wire_with_attestation()).expect("wire should be writable");

        let inputs = load_review_inputs(&project_root, Some(Path::new("trust.json")), None)
            .expect("explicit attestation wire should load");
        assert_eq!(inputs.attestations.len(), 1);
        assert_eq!(
            inputs.attestations[0].review_id().as_str(),
            "review:checkout/reviewer-001"
        );

        std::fs::remove_dir_all(project_root).ok();
    }
}
