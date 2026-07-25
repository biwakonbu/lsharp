use lsharp_syntax::ast::{Expr, Literal};
use lsharp_syntax::span::Span;

use crate::{
    Instruction,
    lower::{FuncCtx, Lower},
};

#[test]
fn record_module_preserves_field_order_for_gc_structs() {
    let mut lower = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());
    lower.record_type_indices.insert("Point".to_string(), 0);
    lower
        .record_fields
        .insert("Point".to_string(), vec!["x".to_string(), "y".to_string()]);
    let fields = vec![
        ("y".to_string(), Expr::Lit(Span::dummy(), Literal::Int(2))),
        ("x".to_string(), Expr::Lit(Span::dummy(), Literal::Int(1))),
    ];

    lower
        .lower_record_lit(&mut ctx, "Point", &fields)
        .expect("record literal should lower");

    assert!(matches!(
        ctx.instructions.last(),
        Some(Instruction::StructNew(0))
    ));
    assert!(matches!(
        ctx.instructions.first(),
        Some(Instruction::I64Const(1))
    ));
    assert!(matches!(
        ctx.instructions.get(1),
        Some(Instruction::I64Const(2))
    ));
}
