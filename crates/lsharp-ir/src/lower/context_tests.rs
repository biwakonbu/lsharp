use crate::IrType;

use super::FuncCtx;

#[test]
fn context_module_exposes_local_allocator() {
    let mut ctx = FuncCtx::with_type_scope("f".to_string(), "f".to_string());

    assert_eq!(ctx.alloc_local_typed("x".to_string(), IrType::I64), 0);
    assert_eq!(ctx.alloc_local_typed("x".to_string(), IrType::I64), 0);
    assert_eq!(ctx.alloc_local_typed("_tmp".to_string(), IrType::I64), 1);
}
