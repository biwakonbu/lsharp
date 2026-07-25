use crate::lexer::Lexer;

use super::Parser;

#[test]
fn metadata_module_exposes_directive_parser() {
    let tokens = Lexer::new(":doc \"hello\"").tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let metadata = parser
        .try_parse_metadata()
        .unwrap()
        .expect("metadata directive should be parsed");

    assert_eq!(metadata.doc.as_deref(), Some("hello"));
}
