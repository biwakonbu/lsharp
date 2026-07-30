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
use lsharp_types::validation::{
    ReviewEvidenceIdentity, ReviewEvidenceIdentityError, ReviewVerificationFact,
    ReviewVerificationProjectionError,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use lsharp_types::intent::ReviewId;

/// attestation を current graph/source snapshot と明示 clock に結び付ける context。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewVerificationContext {
    subject_digest: String,
    source_commit: String,
    artifact_digest: Option<String>,
    now: String,
}

impl ReviewVerificationContext {
    /// CLI/MCP の optional fields を all-or-none の context へ変換する。
    pub fn from_options(
        subject_digest: Option<&str>,
        source_commit: Option<&str>,
        now: Option<&str>,
    ) -> Result<Option<Self>, ReviewInputError> {
        Self::from_options_internal(subject_digest, source_commit, None, now, false)
    }

    /// artifact identity を含む release/evidence gate 用の explicit context を作る。
    pub fn from_options_with_artifact(
        subject_digest: Option<&str>,
        source_commit: Option<&str>,
        artifact_digest: Option<&str>,
        now: Option<&str>,
    ) -> Result<Option<Self>, ReviewInputError> {
        Self::from_options_internal(subject_digest, source_commit, artifact_digest, now, true)
    }

    fn from_options_internal(
        subject_digest: Option<&str>,
        source_commit: Option<&str>,
        artifact_digest: Option<&str>,
        now: Option<&str>,
        require_artifact: bool,
    ) -> Result<Option<Self>, ReviewInputError> {
        if subject_digest.is_none() && source_commit.is_none() && now.is_none() {
            if artifact_digest.is_some() {
                return Err(ReviewInputError::Context {
                    message: "不足している field: review_subject_digest, review_source_commit, review_now"
                        .to_string(),
                });
            }
            return Ok(None);
        }
        let mut missing = [
            ("review_subject_digest", subject_digest),
            ("review_source_commit", source_commit),
            ("review_now", now),
        ]
        .into_iter()
        .filter_map(|(name, value)| value.is_none().then_some(name))
        .collect::<Vec<_>>();
        if !missing.is_empty() {
            if require_artifact && artifact_digest.is_none() {
                missing.push("review_artifact_digest");
            }
            return Err(ReviewInputError::Context {
                message: format!("不足している field: {}", missing.join(", ")),
            });
        }

        if require_artifact && artifact_digest.is_none() {
            return Err(ReviewInputError::Context {
                message: "不足している field: review_artifact_digest".to_string(),
            });
        }

        let subject_digest = subject_digest.expect("all context fields were present");
        let source_commit = source_commit.expect("all context fields were present");
        let now = now.expect("all context fields were present");
        for (field, value) in [
            ("review_subject_digest", subject_digest),
            ("review_source_commit", source_commit),
            ("review_now", now),
        ] {
            if value.trim().is_empty() {
                return Err(ReviewInputError::Context {
                    message: format!("{field} は空にできません"),
                });
            }
        }
        if let Some(value) = artifact_digest
            && value.trim().is_empty()
        {
            return Err(ReviewInputError::Context {
                message: "review_artifact_digest は空にできません".to_string(),
            });
        }

        Ok(Some(Self {
            subject_digest: subject_digest.to_string(),
            source_commit: source_commit.to_string(),
            artifact_digest: artifact_digest.map(str::to_string),
            now: now.to_string(),
        }))
    }

    pub fn subject_digest(&self) -> &str {
        &self.subject_digest
    }

    pub fn source_commit(&self) -> &str {
        &self.source_commit
    }

    pub fn artifact_digest(&self) -> Option<&str> {
        self.artifact_digest.as_deref()
    }

    pub fn now(&self) -> &str {
        &self.now
    }
}

#[derive(Debug, Clone, Default)]
pub struct ReviewInputs {
    pub trust_store: Option<ReviewTrustStore>,
    pub lifecycle: Option<ReviewLifecycleRegistry>,
    /// `--trust-store` が指す version 1 wire に明示された attestation。
    ///
    /// trust root と同じ project-relative input から読み取るが、検証 state の生成は
    /// current graph/source/clock を持つ後続 caller の責務とする。
    pub attestations: Vec<ReviewAttestation>,
    trust_store_digest: Option<String>,
    lifecycle_digest: Option<String>,
}

impl ReviewInputs {
    pub fn trust_store_digest(&self) -> Option<&str> {
        self.trust_store_digest.as_deref()
    }

    pub fn lifecycle_digest(&self) -> Option<&str> {
        self.lifecycle_digest.as_deref()
    }

    /// 明示的な review verification 入力が存在するかを返す。
    pub fn has_explicit_verification_input(&self) -> bool {
        self.trust_store.is_some() || self.lifecycle.is_some()
    }

    /// registry にだけ存在する review を `unverified` fact として補完する。
    ///
    /// M2 の既存入力（review input/context がない場合）は後方互換のため変更せず、
    /// 明示的な verification input がある場合だけ attestation 欠落を成功扱いにしない。
    pub fn complete_verification_facts<'a>(
        &self,
        mut facts: Vec<ReviewVerificationFact>,
        review_ids: impl IntoIterator<Item = &'a ReviewId>,
        explicit_context: bool,
    ) -> Vec<ReviewVerificationFact> {
        if !explicit_context && !self.has_explicit_verification_input() {
            return facts;
        }

        let known = facts
            .iter()
            .map(|fact| fact.review_id().as_str().to_string())
            .collect::<BTreeSet<_>>();
        for review_id in review_ids {
            if known.contains(review_id.as_str()) {
                continue;
            }
            facts.push(
                ReviewVerificationFact::new(review_id.clone(), ReviewVerificationState::Unverified)
                    .expect("unverified review fact は常に projection 可能"),
            );
        }
        facts
    }

    /// artifact を含む explicit context と parsed input digest を report identity へ束ねる。
    pub fn review_evidence_identity(
        &self,
        context: &ReviewVerificationContext,
    ) -> Result<Option<ReviewEvidenceIdentity>, ReviewInputError> {
        let Some(artifact_digest) = context.artifact_digest() else {
            return Ok(None);
        };
        ReviewEvidenceIdentity::new(
            context.subject_digest(),
            context.source_commit(),
            artifact_digest,
            context.now(),
            self.trust_store_digest().map(str::to_string),
            self.lifecycle_digest().map(str::to_string),
        )
        .map(Some)
        .map_err(ReviewInputError::Identity)
    }

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

    /// current identity と明示 clock を含む context で verification fact を生成する。
    ///
    /// registry に存在しない review は provenance digest を比較できないため
    /// `unverified` に留める。既知 key の署名破損は registry の有無にかかわらず診断する。
    pub fn verification_facts_with_context(
        &self,
        context: &ReviewVerificationContext,
        provenance_digests: &BTreeMap<String, String>,
    ) -> Result<Vec<ReviewVerificationFact>, ReviewInputError> {
        self.attestations
            .iter()
            .map(|attestation| {
                let state = match (&self.trust_store, &self.lifecycle) {
                    (Some(trust_store), Some(lifecycle)) => {
                        let Some(provenance_digest) =
                            provenance_digests.get(attestation.review_id().as_str())
                        else {
                            let signature_state =
                                attestation.verify(trust_store).map_err(|source| {
                                    ReviewInputError::Verification {
                                        review_id: attestation.review_id().as_str().to_string(),
                                        source,
                                    }
                                })?;
                            return ReviewVerificationFact::new(
                                attestation.review_id().clone(),
                                if signature_state == ReviewVerificationState::Verified {
                                    ReviewVerificationState::Unverified
                                } else {
                                    signature_state
                                },
                            )
                            .map_err(ReviewInputError::Projection);
                        };
                        attestation
                            .verify_against_at(
                                trust_store,
                                lifecycle,
                                context.subject_digest(),
                                context.source_commit(),
                                provenance_digest,
                                context.now(),
                            )
                            .map_err(|source| ReviewInputError::Verification {
                                review_id: attestation.review_id().as_str().to_string(),
                                source,
                            })?
                    }
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
    #[error("review evidence identity の生成に失敗しました: {0}")]
    Identity(#[from] ReviewEvidenceIdentityError),
    #[error("review verification context が不完全です: {message}")]
    Context { message: String },
}

/// 明示 review input を project root 内の通常ファイルへ解決して parse する。
pub fn load_review_inputs(
    project_root: &Path,
    trust_store_path: Option<&Path>,
    lifecycle_path: Option<&Path>,
) -> Result<ReviewInputs, ReviewInputError> {
    let (trust_store, attestations, trust_store_digest) = match trust_store_path {
        Some(path) => {
            let (trust_store, attestations, digest) = load_trust_store(project_root, path)?;
            (Some(trust_store), attestations, Some(digest))
        }
        None => (None, Vec::new(), None),
    };
    let (lifecycle, lifecycle_digest) = match lifecycle_path {
        Some(path) => {
            let (lifecycle, digest) = load_lifecycle(project_root, path)?;
            (Some(lifecycle), Some(digest))
        }
        None => (None, None),
    };
    Ok(ReviewInputs {
        trust_store,
        lifecycle,
        attestations,
        trust_store_digest,
        lifecycle_digest,
    })
}

fn load_trust_store(
    project_root: &Path,
    configured: &Path,
) -> Result<(ReviewTrustStore, Vec<ReviewAttestation>, String), ReviewInputError> {
    let resolved = resolve_review_input_path(project_root, configured, "trust store")?;
    let document = read_wire(&resolved, "trust store")?;
    let digest = wire_component_digest(&document, "trust_store");
    let trust_store =
        document
            .trust_store()
            .cloned()
            .ok_or_else(|| ReviewInputError::MissingTrustStore {
                path: resolved.display().to_string(),
            })?;
    Ok((trust_store, document.attestations().to_vec(), digest))
}

fn load_lifecycle(
    project_root: &Path,
    configured: &Path,
) -> Result<(ReviewLifecycleRegistry, String), ReviewInputError> {
    let resolved = resolve_review_input_path(project_root, configured, "lifecycle")?;
    let document = read_wire(&resolved, "lifecycle")?;
    let digest = wire_component_digest(&document, "lifecycle");
    Ok((document.lifecycle().clone(), digest))
}

fn wire_component_digest(
    document: &lsharp_types::intent::review_wire::ReviewWireDocument,
    component: &str,
) -> String {
    let wire = document.to_json_value();
    let component_value = wire.get(component).cloned().unwrap_or(Value::Null);
    let mut canonical = serde_json::Map::new();
    canonical.insert(
        "schema_version".to_string(),
        serde_json::json!(document.schema_version()),
    );
    canonical.insert(component.to_string(), component_value);
    let bytes = serde_json::to_vec(&Value::Object(canonical))
        .expect("review wire component は serializable");
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut hex, "{byte:02x}").expect("String は write 可能");
    }
    format!("sha256:{hex}")
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

    #[test]
    fn explicit_review_input_digests_are_stable_and_identity_is_complete_when_artifact_is_given() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos();
        let project_root =
            std::env::temp_dir().join(format!("lsharp-review-identity-test-{nonce}"));
        std::fs::create_dir_all(&project_root).expect("project root should be writable");
        let first = project_root.join("trust-first.json");
        let second = project_root.join("trust-second.json");
        let lifecycle = project_root.join("lifecycle.json");
        std::fs::write(&first, wire_with_attestation()).expect("first wire should be writable");
        std::fs::write(
            &second,
            r#"{
              "trust_store": [{
                "public_key": "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc",
                "algorithm": "ed25519",
                "key_id": "org/reviews-2026",
                "provider": "github"
              }],
              "lifecycle": [],
              "attestations": [],
              "schema_version": 1
            }"#,
        )
        .expect("reordered wire should be writable");
        std::fs::write(
            &lifecycle,
            r#"{
              "schema_version": 1,
              "attestations": [],
              "lifecycle": [{
                "review_id": "review:checkout/reviewer-001",
                "sequence": 1,
                "state": "proposed",
                "effective_at": "2026-07-29T00:00:00Z"
              }]
            }"#,
        )
        .expect("lifecycle wire should be writable");

        let first_inputs = load_review_inputs(
            &project_root,
            Some(Path::new("trust-first.json")),
            Some(Path::new("lifecycle.json")),
        )
        .expect("first explicit inputs should load");
        let second_inputs =
            load_review_inputs(&project_root, Some(Path::new("trust-second.json")), None)
                .expect("reordered explicit input should load");
        assert_eq!(
            first_inputs.trust_store_digest(),
            second_inputs.trust_store_digest(),
            "trust-store digest must ignore JSON object/array input order"
        );
        assert!(first_inputs.lifecycle_digest().is_some());

        let context = ReviewVerificationContext::from_options_with_artifact(
            Some("sha256:graph"),
            Some("commit-1"),
            Some("sha256:artifact"),
            Some("2026-08-15T00:00:00Z"),
        )
        .expect("complete review context should load")
        .expect("complete review context should be present");
        let identity = first_inputs
            .review_evidence_identity(&context)
            .expect("identity projection should succeed")
            .expect("artifact-bearing context should produce identity");
        assert_eq!(identity.subject_digest(), "sha256:graph");
        assert_eq!(identity.source_commit(), "commit-1");
        assert_eq!(identity.artifact_digest(), "sha256:artifact");
        assert_eq!(identity.now(), "2026-08-15T00:00:00Z");
        assert!(identity.trust_store_digest().is_some());
        assert!(identity.lifecycle_digest().is_some());

        std::fs::remove_dir_all(project_root).ok();
    }

    #[test]
    fn artifact_identity_context_rejects_missing_required_context() {
        let error = ReviewVerificationContext::from_options_with_artifact(
            Some("sha256:graph"),
            Some("commit-1"),
            None,
            Some("2026-08-15T00:00:00Z"),
        )
        .expect_err("artifact identity requires an explicit artifact digest");
        assert!(error.to_string().contains("review_artifact_digest"));
    }
}
