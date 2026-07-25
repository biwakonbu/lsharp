use crate::ast::Decl;
use crate::lexer::Lexer;

use super::Parser;

#[test]
fn declaration_module_exposes_top_level_parser() {
    let tokens = Lexer::new("(defn identity [x] x)").tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let parsed = parser.parse_decl().unwrap();

    assert!(matches!(parsed, Decl::Defn { ref name, .. } if name == "identity"));
}
