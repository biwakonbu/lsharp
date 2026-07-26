use crate::types::{Kind, Type, TypeVarId};
use lsharp_syntax::span::Span;

/// 型推論エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum TypeError {
    #[error("[{error_code}] 型の不一致: expected {expected}, found {found} ({span})")]
    Mismatch {
        expected: Type,
        found: Type,
        span: Span,
        /// エラーコード (E0002=if条件, E0003=分岐不一致, E0004=引数不一致, E0006=一般)
        error_code: TypeErrorCode,
    },

    #[error("[E0005] 無限型 (infinite type): t{var} は {ty} に出現します ({span})")]
    InfiniteType {
        var: TypeVarId,
        ty: Type,
        span: Span,
    },

    #[error("[E0001] 未定義の変数 (undefined): {name} ({span})")]
    UndefinedVar { name: String, span: Span },

    #[error("[E0001] 未定義のコンストラクタ: {name} ({span})")]
    UndefinedConstructor { name: String, span: Span },

    #[error("[E0006] 引数の数が不一致: 期待 {expected}, 実際 {found} ({span})")]
    ArityMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("[E0001] 未定義のレコード型: {name} ({span})")]
    UndefinedRecord { name: String, span: Span },

    #[error("[E0001] 未定義のフィールド: {record_name}.{field_name} ({span})")]
    UndefinedField {
        record_name: String,
        field_name: String,
        span: Span,
    },

    #[error("[E0006] 再帰的な型エイリアス: {name} ({span})")]
    RecursiveAlias { name: String, span: Span },

    #[error("[E0001] 未定義の型エイリアス: {name} ({span})")]
    UndefinedAlias { name: String, span: Span },

    #[error("[E0001] 未定義のトレイト: {name} ({span})")]
    UndefinedTrait { name: String, span: Span },

    #[error("[E0006] トレイト {trait_name} の実装が見つかりません: {type_name} ({span})")]
    MissingImpl {
        trait_name: String,
        type_name: String,
        span: Span,
    },

    #[error(
        "[{error_code}] 型の不一致 (mismatch): expected {expected}, found {found} (エイリアス '{alias_name}' は {expanded} に展開) ({span})"
    )]
    MismatchWithAlias {
        expected: Type,
        found: Type,
        alias_name: String,
        expanded: Type,
        span: Span,
        error_code: TypeErrorCode,
    },

    #[error(
        "[E0006] Kind の不一致: {type_name} は {actual_kind} ですが、トレイト {trait_name} は {expected_kind} を要求します ({span})"
    )]
    KindMismatch {
        type_name: String,
        trait_name: String,
        expected_kind: Kind,
        actual_kind: Kind,
        span: Span,
    },
}

impl TypeError {
    /// 型推論エラーを公開診断コードへ変換する。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Mismatch { error_code, .. } | Self::MismatchWithAlias { error_code, .. } => {
                match error_code {
                    TypeErrorCode::IfCondition
                    | TypeErrorCode::IfBranch
                    | TypeErrorCode::General => "LS1002",
                    TypeErrorCode::ArgMismatch => "LS1004",
                }
            }
            Self::InfiniteType { .. } => "LS1003",
            Self::UndefinedVar { .. } => "LS1001",
            Self::UndefinedConstructor { .. } => "LS1005",
            Self::ArityMismatch { .. } => "LS1004",
            Self::UndefinedRecord { .. } => "LS1006",
            Self::UndefinedField { .. } => "LS1007",
            Self::RecursiveAlias { .. } => "LS1008",
            Self::UndefinedAlias { .. } => "LS1009",
            Self::UndefinedTrait { .. } => "LS1010",
            Self::MissingImpl { .. } => "LS1011",
            Self::KindMismatch { .. } => "LS1013",
        }
    }

    pub fn span(&self) -> Option<Span> {
        Some(match self {
            Self::Mismatch { span, .. }
            | Self::InfiniteType { span, .. }
            | Self::UndefinedVar { span, .. }
            | Self::UndefinedConstructor { span, .. }
            | Self::ArityMismatch { span, .. }
            | Self::UndefinedRecord { span, .. }
            | Self::UndefinedField { span, .. }
            | Self::RecursiveAlias { span, .. }
            | Self::UndefinedAlias { span, .. }
            | Self::UndefinedTrait { span, .. }
            | Self::MissingImpl { span, .. }
            | Self::MismatchWithAlias { span, .. }
            | Self::KindMismatch { span, .. } => *span,
        })
    }
}

/// 型エラーコード (E0001 形式)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeErrorCode {
    /// E0002: if 条件が Bool でない
    IfCondition,
    /// E0003: if 分岐の型不一致
    IfBranch,
    /// E0004: 関数引数の型不一致
    ArgMismatch,
    /// E0006: 一般的な型不一致
    General,
}

impl std::fmt::Display for TypeErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeErrorCode::IfCondition => write!(f, "E0002"),
            TypeErrorCode::IfBranch => write!(f, "E0003"),
            TypeErrorCode::ArgMismatch => write!(f, "E0004"),
            TypeErrorCode::General => write!(f, "E0006"),
        }
    }
}

impl std::error::Error for TypeErrorCode {}
