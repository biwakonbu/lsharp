use lsharp_syntax::ast::{Expr, Literal, MatchArm, Pattern};
use lsharp_syntax::span::Span;

use crate::{
    Instruction,
    lower::{FuncCtx, Lower},
};

#[test]
fn match_expr_module_preserves_scrutinee_localization() {
    let mut lower = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());
    let scrutinee = Expr::Lit(Span::dummy(), Literal::Int(7));
    let arms = vec![MatchArm {
        span: Span::dummy(),
        pattern: Pattern::Wildcard(Span::dummy()),
        guard: None,
        body: Expr::Lit(Span::dummy(), Literal::Int(42)),
    }];

    lower
        .lower_match_expr(&mut ctx, &scrutinee, &arms)
        .expect("match expression should lower");

    assert!(matches!(
        ctx.instructions.first(),
        Some(Instruction::I64Const(7))
    ));
    assert!(matches!(
        ctx.instructions.get(1),
        Some(Instruction::LocalSet(0))
    ));
    assert!(matches!(
        ctx.instructions.last(),
        Some(Instruction::I64Const(42))
    ));
}
