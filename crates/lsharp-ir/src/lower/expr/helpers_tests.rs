use lsharp_syntax::span::Span;

use crate::{
    Instruction,
    lower::{FuncCtx, Lower},
};

#[test]
fn expr_helper_module_preserves_binop_emission_contract() {
    let mut lower = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());

    lower
        .emit_binop(&mut ctx, "+", Span::dummy())
        .expect("known binary operator should lower");

    assert_eq!(ctx.instructions.len(), 1);
    assert!(matches!(ctx.instructions[0], Instruction::I64Add));
}
