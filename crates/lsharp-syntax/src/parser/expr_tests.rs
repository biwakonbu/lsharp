use crate::ast::Expr;
use crate::lexer::Lexer;

use super::Parser;

#[test]
fn expr_module_exposes_expression_parser() {
    let tokens = Lexer::new("(if true 1 2)").tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let parsed = parser.parse_expr().unwrap();

    assert!(matches!(parsed, Expr::If(_, _, _, _)));
}
