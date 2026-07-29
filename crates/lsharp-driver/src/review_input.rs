//! v0.3 review trust/lifecycle input の explicit file boundary。
//!
//! `validate` の review verification input は caller が指定した project-relative file だけを
//! 読む。current manifest、環境変数、暗黙の default から trust root を補わず、symlink を含む
//! project root 外の path と review wire の schema violation を fail-closed にする。

use lsharp_types::intent::review_lifecycle::ReviewLifecycleRegistry;
use lsharp_types::intent::review_trust_store::ReviewTrustStore;
use lsharp_types::intent::review_wire::parse_review_wire;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct ReviewInputs {
    pub trust_store: Option<ReviewTrustStore>,
    pub lifecycle: Option<ReviewLifecycleRegistry>,
}

impl ReviewInputs {
    pub fn explicit_count(&self) -> usize {
        usize::from(self.trust_store.is_some()) + usize::from(self.lifecycle.is_some())
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
}

/// 明示 review input を project root 内の通常ファイルへ解決して parse する。
pub fn load_review_inputs(
    project_root: &Path,
    trust_store_path: Option<&Path>,
    lifecycle_path: Option<&Path>,
) -> Result<ReviewInputs, ReviewInputError> {
    let trust_store = trust_store_path
        .map(|path| load_trust_store(project_root, path))
        .transpose()?;
    let lifecycle = lifecycle_path
        .map(|path| load_lifecycle(project_root, path))
        .transpose()?;
    Ok(ReviewInputs {
        trust_store,
        lifecycle,
    })
}

fn load_trust_store(
    project_root: &Path,
    configured: &Path,
) -> Result<ReviewTrustStore, ReviewInputError> {
    let resolved = resolve_review_input_path(project_root, configured, "trust store")?;
    let document = read_wire(&resolved, "trust store")?;
    document
        .trust_store()
        .cloned()
        .ok_or_else(|| ReviewInputError::MissingTrustStore {
            path: resolved.display().to_string(),
        })
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
