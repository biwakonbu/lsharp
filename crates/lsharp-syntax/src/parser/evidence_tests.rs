use crate::lexer::Lexer;
use crate::span::Span;

use super::Parser;

#[test]
fn evidence_module_exposes_record_parser() {
    let source = concat!(
        ":subject \"subject\" :method \"method\" :outcome \"pass\" ",
        ":runner \"runner\" :target \"target\" :source-commit \"commit\" ",
        ":artifact-digest \"digest\" :cases 1 :seed 2 :generator \"gen\" ",
        ":producer \"producer\" :tool-version \"tool\" :timestamp \"now\" ",
        ":independence \"independent\""
    );
    let tokens = Lexer::new(source).tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let (record, _) = parser
        .parse_evidence_form("evidence-1".to_string(), Span::new(0, 1))
        .unwrap();

    assert_eq!(record.id(), "evidence-1");
    assert_eq!(record.subject(), "subject");
}
