use lsharp_syntax::ast::{Expr, Literal};
use lsharp_syntax::span::Span;

use crate::{
    Instruction,
    lower::{FuncCtx, Lower},
};

#[test]
fn do_expr_module_preserves_intermediate_drop_order() {
    let mut lower = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());
    let exprs = vec![
        Expr::Lit(Span::dummy(), Literal::Int(1)),
        Expr::Lit(Span::dummy(), Literal::Int(2)),
    ];

    lower
        .lower_do(&mut ctx, &exprs)
        .expect("do expression should lower");

    assert!(matches!(
        ctx.instructions.first(),
        Some(Instruction::I64Const(1))
    ));
    assert!(matches!(ctx.instructions.get(1), Some(Instruction::Drop)));
    assert!(matches!(
        ctx.instructions.last(),
        Some(Instruction::I64Const(2))
    ));
}
