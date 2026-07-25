use lsharp_syntax::span::Span;

use crate::{
    Instruction,
    lower::{FuncCtx, Lower, LowerError},
};

#[test]
fn var_expr_module_preserves_local_lookup() {
    let mut lower = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());
    ctx.locals_map.insert("value".to_string(), 3);

    lower
        .lower_var(&mut ctx, Span::dummy(), "value")
        .expect("local variable should lower");

    assert!(matches!(
        ctx.instructions.first(),
        Some(Instruction::LocalGet(3))
    ));
}

#[test]
fn var_expr_module_preserves_undefined_name_diagnostic() {
    let mut lower = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());
    let span = Span::dummy();

    let error = lower
        .lower_var(&mut ctx, span, "missing")
        .expect_err("unknown variable should remain an error");

    assert!(matches!(
        error,
        LowerError::UndefinedFunction { name, span: Some(error_span) }
            if name == "missing" && error_span == span
    ));
}
