use crate::span::Span;

/// P10-3: マクロ展開トレースの1エントリ
#[derive(Debug, Clone)]
pub struct MacroExpansionStep {
    /// 展開されたマクロ名
    pub macro_name: String,
    /// 呼び出し元のスパン
    pub call_span: Span,
    /// 展開の深さ
    pub depth: usize,
}

/// マクロ展開エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum MacroExpandError {
    #[error("未定義マクロ: {name}")]
    UndefinedMacro { name: String, span: Span },
    #[error(
        "マクロ引数の数が一致しません: {name} は {expected} 個の引数を受け取りますが、{actual} 個が渡されました"
    )]
    ArityMismatch {
        name: String,
        expected: usize,
        actual: usize,
        span: Span,
    },
    #[error("マクロ展開の再帰制限 ({limit}) を超えました: {name}")]
    RecursionLimit {
        name: String,
        limit: usize,
        span: Span,
    },
    #[error("unquote-splicing (~@) はリストコンテキストでのみ使用できます")]
    SpliceOutsideList { span: Span },
    /// P10-3: 展開トレースバック付きエラー
    #[error("マクロ展開エラー (トレースバック付き): {inner}")]
    WithTrace {
        inner: Box<MacroExpandError>,
        /// 展開トレース (呼び出し順)
        trace: Vec<MacroExpansionStep>,
    },
}

impl MacroExpandError {
    /// 利用者向けの安定した診断コードを返す。
    pub fn code(&self) -> &'static str {
        "LS0201"
    }

    /// 診断に対応する source span を返す。
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::UndefinedMacro { span, .. }
            | Self::ArityMismatch { span, .. }
            | Self::RecursionLimit { span, .. }
            | Self::SpliceOutsideList { span } => Some(*span),
            Self::WithTrace { inner, .. } => inner.span(),
        }
    }

    /// P10-3: トレースバックを付与してエラーを返す
    pub fn with_trace(self, trace: Vec<MacroExpansionStep>) -> Self {
        if trace.is_empty() {
            return self;
        }
        MacroExpandError::WithTrace {
            inner: Box::new(self),
            trace,
        }
    }

    /// P10-3: トレースバックのフォーマット済み文字列を取得
    pub fn format_traceback(&self) -> String {
        match self {
            MacroExpandError::WithTrace { inner, trace } => {
                let mut msg = format!("{inner}\n\nマクロ展開トレースバック:");
                for (i, step) in trace.iter().enumerate() {
                    msg.push_str(&format!(
                        "\n  [{i}] {} (depth={}, span={}..{})",
                        step.macro_name, step.depth, step.call_span.start, step.call_span.end
                    ));
                }
                msg
            }
            other => format!("{other}"),
        }
    }
}
