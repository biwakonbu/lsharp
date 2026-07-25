use crate::ast::TypeExpr;
use crate::lexer::Lexer;

use super::Parser;

#[test]
fn type_expr_module_exposes_type_parser() {
    let tokens = Lexer::new("(-> Int a)").tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let parsed = parser.parse_type_expr().unwrap();

    let TypeExpr::Fun(_, params, return_type) = parsed else {
        panic!("expected a function type");
    };
    assert_eq!(params.len(), 1);
    assert!(matches!(*return_type, TypeExpr::Var(_, ref name) if name == "a"));
}
