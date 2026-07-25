use lsharp_syntax::ast::{Expr, Literal};
use lsharp_syntax::span::Span;

use crate::{
    Instruction,
    lower::{FuncCtx, Lower},
};

#[test]
fn application_module_preserves_binary_operator_lowering() {
    let mut lower = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());
    let func = Box::new(Expr::Var(Span::dummy(), "+".to_string()));
    let args = vec![
        Expr::Lit(Span::dummy(), Literal::Int(1)),
        Expr::Lit(Span::dummy(), Literal::Int(2)),
    ];

    lower
        .lower_app(&mut ctx, Span::dummy(), &func, &args)
        .expect("binary operator application should lower");

    assert!(matches!(ctx.instructions.last(), Some(Instruction::I64Add)));
}
