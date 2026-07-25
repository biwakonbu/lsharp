use crate::ast::Pattern;
use crate::lexer::Lexer;

use super::Parser;

#[test]
fn pattern_module_exposes_pattern_parser() {
    let tokens = Lexer::new("(Some x)").tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let parsed = parser.parse_pattern().unwrap();

    assert!(matches!(
        parsed,
        Pattern::Constructor(_, ref name, ref fields) if name == "Some" && fields.len() == 1
    ));
}
