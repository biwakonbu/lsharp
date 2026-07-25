use lsharp_syntax::ast::{ComputationStep, Expr, Literal};
use lsharp_syntax::span::Span;

use crate::{
    Instruction,
    lower::{FuncCtx, Lower},
};

#[test]
fn computation_module_preserves_return_builder_call() {
    let mut lower = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());
    lower.func_indices.insert("return-int".to_string(), 7);
    lower.computation_builders.insert(
        "Option".to_string(),
        ("bind-option".to_string(), "return-int".to_string()),
    );
    let steps = vec![ComputationStep::Return(
        Span::dummy(),
        Expr::Lit(Span::dummy(), Literal::Int(9)),
    )];

    lower
        .lower_computation(&mut ctx, Span::dummy(), "Option", &steps)
        .expect("computation expression should lower");

    assert!(matches!(
        ctx.instructions.first(),
        Some(Instruction::I64Const(9))
    ));
    assert!(matches!(
        ctx.instructions.last(),
        Some(Instruction::Call(7))
    ));
}
