use lsharp_syntax::ast::Literal;
use lsharp_syntax::span::Span;

use crate::{
    Instruction,
    lower::{FuncCtx, Lower, LowerBackend},
};

#[test]
fn literal_expr_module_preserves_scalar_lowering() {
    let mut lower = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());

    lower
        .lower_lit(&mut ctx, Span::dummy(), &Literal::Int(42))
        .expect("integer literal should lower");

    assert!(matches!(
        ctx.instructions.first(),
        Some(Instruction::I64Const(42))
    ));
}

#[test]
fn literal_expr_module_preserves_wasmgc_string_array_lowering() {
    let mut lower = Lower::with_backend(LowerBackend::WasmGc);
    lower.string_array_type_index = Some(4);
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());

    lower
        .lower_lit(&mut ctx, Span::dummy(), &Literal::String("hi".to_string()))
        .expect("WasmGC string literal should lower");

    assert!(matches!(
        ctx.instructions.as_slice(),
        [
            Instruction::I32Const(104),
            Instruction::I32Const(105),
            Instruction::ArrayNewFixed(4, 2),
        ]
    ));
}

#[test]
fn literal_expr_module_preserves_linear_string_allocation_boundary() {
    let mut lower = Lower::new();
    lower.func_indices.insert("__alloc".to_string(), 7);
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());

    lower
        .lower_lit(&mut ctx, Span::dummy(), &Literal::String("hi".to_string()))
        .expect("linear-memory string literal should lower");

    assert!(matches!(
        ctx.instructions.as_slice(),
        [Instruction::I64Const(10), Instruction::Call(7), ..]
    ));
    assert_eq!(
        lower.string_data,
        vec![("$str0".to_string(), b"hi".to_vec())]
    );
}
