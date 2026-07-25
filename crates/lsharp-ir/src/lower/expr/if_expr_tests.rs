use lsharp_syntax::ast::{Expr, Literal};
use lsharp_syntax::span::Span;

use crate::{
    Instruction, IrType,
    lower::{FuncCtx, Lower},
};

#[test]
fn if_expr_module_preserves_condition_and_branch_order() {
    let mut lower = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());
    let condition = Expr::Lit(Span::dummy(), Literal::Bool(true));
    let then_branch = Expr::Lit(Span::dummy(), Literal::Int(1));
    let else_branch = Expr::Lit(Span::dummy(), Literal::Int(2));

    lower
        .lower_if(&mut ctx, &condition, &then_branch, &else_branch)
        .expect("if expression should lower");

    assert!(matches!(
        ctx.instructions.first(),
        Some(Instruction::I64Const(1))
    ));
    assert!(matches!(
        ctx.instructions.get(1),
        Some(Instruction::I32WrapI64)
    ));
    assert!(matches!(
        ctx.instructions.get(2),
        Some(Instruction::If(IrType::I64))
    ));
    assert!(matches!(
        ctx.instructions.get(3),
        Some(Instruction::I64Const(1))
    ));
    assert!(matches!(ctx.instructions.get(4), Some(Instruction::Else)));
    assert!(matches!(
        ctx.instructions.get(5),
        Some(Instruction::I64Const(2))
    ));
    assert!(matches!(ctx.instructions.last(), Some(Instruction::End)));
}
