use lsharp_syntax::ast::Expr;
use lsharp_syntax::span::Span;

use crate::lower::Lower;

#[test]
fn wasmgc_lambda_module_preserves_free_variable_filtering() {
    let lower = Lower::new();
    let body = Expr::Var(Span::dummy(), "captured".to_string());

    assert_eq!(
        lower.wasmgc_lambda_free_vars(&[], &body),
        vec!["captured".to_string()]
    );
}
