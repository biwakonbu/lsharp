use lsharp_syntax::ast::Expr;
use lsharp_syntax::span::Span;

use crate::{
    Instruction,
    lower::{FuncCtx, Lower, LowerBackend},
};

#[test]
fn lambda_module_preserves_wasmgc_funcref_lifting() {
    let mut lower = Lower::with_backend(LowerBackend::WasmGc);
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());
    let body = Expr::Lit(Span::dummy(), lsharp_syntax::ast::Literal::Int(1));

    lower
        .lower_lambda(&mut ctx, Span::dummy(), &[], &body)
        .expect("WasmGC lambda should lift");

    assert!(matches!(
        ctx.instructions.last(),
        Some(Instruction::RefFunc(_))
    ));
    assert_eq!(lower.lifted_functions.len(), 1);
}
