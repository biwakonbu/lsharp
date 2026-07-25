use lsharp_syntax::ast::{Expr, Literal};
use lsharp_syntax::span::Span;

use crate::{
    Instruction,
    lower::{FuncCtx, Lower},
};

#[test]
fn ann_expr_module_preserves_inner_expression_lowering() {
    let mut lower = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());
    let inner = Expr::Lit(Span::dummy(), Literal::Int(11));

    lower
        .lower_ann(&mut ctx, &inner)
        .expect("annotated expression should lower");

    assert!(matches!(
        ctx.instructions.first(),
        Some(Instruction::I64Const(11))
    ));
}
