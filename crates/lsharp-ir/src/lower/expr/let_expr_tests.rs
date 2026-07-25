use lsharp_syntax::ast::{Expr, Literal, Pattern};
use lsharp_syntax::span::Span;

use crate::{
    Instruction,
    lower::{FuncCtx, Lower},
};

#[test]
fn let_expr_module_preserves_scoped_binding_restore() {
    let mut lower = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());
    let bindings = vec![(
        Pattern::Var(Span::dummy(), "x".to_string()),
        Expr::Lit(Span::dummy(), Literal::Int(1)),
    )];
    let body = Expr::Var(Span::dummy(), "x".to_string());

    lower
        .lower_let(&mut ctx, &bindings, &body)
        .expect("let expression should lower");

    assert!(matches!(
        ctx.instructions.first(),
        Some(Instruction::I64Const(1))
    ));
    assert!(matches!(
        ctx.instructions.get(1),
        Some(Instruction::LocalSet(0))
    ));
    assert!(matches!(
        ctx.instructions.last(),
        Some(Instruction::LocalGet(0))
    ));
    assert!(!ctx.locals_map.contains_key("x"));
}
