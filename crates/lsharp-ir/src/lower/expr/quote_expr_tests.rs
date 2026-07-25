use lsharp_syntax::span::Span;

use crate::lower::{Lower, LowerError};

#[test]
fn quote_expr_module_preserves_explicit_unsupported_boundary() {
    let mut lower = Lower::new();
    let span = Span::dummy();

    let error = lower
        .lower_quote(span)
        .expect_err("quote expression should remain unsupported after macro expansion");

    match error {
        LowerError::Unsupported {
            msg,
            span: Some(error_span),
        } => {
            assert_eq!(msg, "quote/unquote はマクロ展開後に使用できません");
            assert_eq!(error_span, span);
        }
        other => panic!("unexpected quote lowering error: {other:?}"),
    }
}
