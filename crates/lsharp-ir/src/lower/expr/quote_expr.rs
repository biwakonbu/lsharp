use lsharp_syntax::span::Span;

use super::{Lower, LowerError};

impl Lower {
    pub(super) fn lower_quote(&mut self, expr_span: Span) -> Result<(), LowerError> {
        // P10-1: Quote/Unquote/UnquoteSplice はマクロ展開後には残らない
        Err(LowerError::Unsupported {
            msg: "quote/unquote はマクロ展開後に使用できません".to_string(),
            span: Some(expr_span),
        })
    }
}
