//! v0.2 intent/evidence graph の node identity と最小 AST。
//!
//! この slice は source parser や `validate` command へ接続する前の canonical
//! model を固定する。node の ID は source span や宣言順から導出せず、利用者が
//! 指定する namespace/key から wire value を決定的に組み立てる。したがって
//! formatter、宣言の並べ替え、別 target の実行で ID が変わらない。

use lsharp_syntax::span::Span;

/// intent/evidence graph の node 種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Intent,
    Claim,
    Assumption,
    OpenQuestion,
    Contract,
    Evidence,
    Review,
    Change,
}

impl NodeKind {
    /// stable ID の wire prefix。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Intent => "intent",
            Self::Claim => "claim",
            Self::Assumption => "assumption",
            Self::OpenQuestion => "open-question",
            Self::Contract => "contract",
            Self::Evidence => "evidence",
            Self::Review => "review",
            Self::Change => "change",
        }
    }
}

/// stable ID の構成要素が wire format に使えない場合のエラー。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StableIdError {
    #[error("stable ID の {field} は空でない ASCII segment にしてください: {value:?}")]
    InvalidSegment { field: &'static str, value: String },
}

/// node kind、namespace、key からなる cross-target stable ID。
///
/// wire format は `kind:namespace/key`。namespace/key は source span、宣言順、
/// hash に依存しないため、同じ project identity を指定した Rust/selfhost の
/// projection が同じ文字列になる。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StableId {
    kind: NodeKind,
    namespace: String,
    key: String,
    wire: String,
}

impl StableId {
    pub fn new(
        kind: NodeKind,
        namespace: impl Into<String>,
        key: impl Into<String>,
    ) -> Result<Self, StableIdError> {
        let namespace = namespace.into();
        let key = key.into();
        validate_segment("namespace", &namespace)?;
        validate_segment("key", &key)?;
        let wire = format!("{}:{namespace}/{key}", kind.as_str());
        Ok(Self {
            kind,
            namespace,
            key,
            wire,
        })
    }

    pub fn as_str(&self) -> &str {
        &self.wire
    }

    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn key(&self) -> &str {
        &self.key
    }
}

fn validate_segment(field: &'static str, value: &str) -> Result<(), StableIdError> {
    if value.is_empty()
        || value
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.')))
    {
        return Err(StableIdError::InvalidSegment {
            field,
            value: value.to_string(),
        });
    }
    Ok(())
}

macro_rules! define_id {
    ($name:ident, $kind:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub struct $name(StableId);

        impl $name {
            pub fn new(
                namespace: impl Into<String>,
                key: impl Into<String>,
            ) -> Result<Self, StableIdError> {
                StableId::new(NodeKind::$kind, namespace, key).map(Self)
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }

            pub fn kind(&self) -> NodeKind {
                self.0.kind()
            }

            pub fn namespace(&self) -> &str {
                self.0.namespace()
            }

            pub fn key(&self) -> &str {
                self.0.key()
            }

            pub fn stable_id(&self) -> &StableId {
                &self.0
            }
        }
    };
}

define_id!(IntentId, Intent);
define_id!(ClaimId, Claim);
define_id!(AssumptionId, Assumption);
define_id!(OpenQuestionId, OpenQuestion);
define_id!(ContractId, Contract);
define_id!(EvidenceId, Evidence);
define_id!(ReviewId, Review);
define_id!(ChangeId, Change);

/// AST node の説明文が空の場合のエラー。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NodeTextError {
    #[error("intent graph node の text は空にできません")]
    EmptyText,
}

macro_rules! define_node {
    ($name:ident, $id:ident, $kind:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name {
            id: $id,
            text: String,
            source_span: Span,
        }

        impl $name {
            pub fn new(
                id: $id,
                text: impl Into<String>,
                source_span: Span,
            ) -> Result<Self, NodeTextError> {
                let text = text.into();
                if text.trim().is_empty() {
                    return Err(NodeTextError::EmptyText);
                }
                Ok(Self {
                    id,
                    text,
                    source_span,
                })
            }

            pub fn id(&self) -> &$id {
                &self.id
            }

            pub fn text(&self) -> &str {
                &self.text
            }

            pub fn source_span(&self) -> Span {
                self.source_span
            }

            pub const fn kind(&self) -> NodeKind {
                NodeKind::$kind
            }
        }
    };
}

define_node!(Intent, IntentId, Intent);
define_node!(Claim, ClaimId, Claim);
define_node!(Assumption, AssumptionId, Assumption);
define_node!(OpenQuestion, OpenQuestionId, OpenQuestion);

/// M2-01 の最小 intent AST。edge/evidence は M2-02 で追加する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentNode {
    Intent(Intent),
    Claim(Claim),
    Assumption(Assumption),
    OpenQuestion(OpenQuestion),
}

impl IntentNode {
    pub fn kind(&self) -> NodeKind {
        match self {
            Self::Intent(node) => node.kind(),
            Self::Claim(node) => node.kind(),
            Self::Assumption(node) => node.kind(),
            Self::OpenQuestion(node) => node.kind(),
        }
    }

    pub fn stable_id(&self) -> &StableId {
        match self {
            Self::Intent(node) => node.id().stable_id(),
            Self::Claim(node) => node.id().stable_id(),
            Self::Assumption(node) => node.id().stable_id(),
            Self::OpenQuestion(node) => node.id().stable_id(),
        }
    }

    pub fn source_span(&self) -> Span {
        match self {
            Self::Intent(node) => node.source_span(),
            Self::Claim(node) => node.source_span(),
            Self::Assumption(node) => node.source_span(),
            Self::OpenQuestion(node) => node.source_span(),
        }
    }

    pub fn text(&self) -> &str {
        match self {
            Self::Intent(node) => node.text(),
            Self::Claim(node) => node.text(),
            Self::Assumption(node) => node.text(),
            Self::OpenQuestion(node) => node.text(),
        }
    }
}
