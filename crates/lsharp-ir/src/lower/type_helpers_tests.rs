use lsharp_syntax::ast::TypeExpr;
use lsharp_syntax::span::Span;
use lsharp_types::types::Type;

use crate::IrType;

use super::{is_heap_like_type_name, type_expr_to_ir, type_expr_to_name, type_to_ir, type_to_name};

#[test]
fn type_helper_module_preserves_conversion_and_name_contracts() {
    assert_eq!(type_to_ir(&Type::Con("Int".to_string())), IrType::I64);
    assert_eq!(type_to_ir(&Type::Con("Float".to_string())), IrType::F64);
    assert_eq!(
        type_to_name(&Type::App("Option".to_string(), vec![])),
        Some("Option".to_string())
    );
    assert_eq!(
        type_expr_to_name(&TypeExpr::Named(Span::dummy(), "User".to_string())),
        Some("User".to_string())
    );
    assert_eq!(
        type_expr_to_ir(&TypeExpr::Named(Span::dummy(), "String".to_string())),
        IrType::I64
    );
    assert!(!is_heap_like_type_name("Int"));
    assert!(is_heap_like_type_name("String"));
}
