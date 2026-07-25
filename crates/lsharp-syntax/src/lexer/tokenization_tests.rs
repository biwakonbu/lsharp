use super::{Lexer, TokenKind};

#[test]
fn tokenization_module_exposes_number_and_symbol_scanners() {
    let mut number_lexer = Lexer::new("41");
    let number = super::tokenization::lex_number(&mut number_lexer).unwrap();
    assert_eq!(number.kind, TokenKind::Int(41));

    let mut symbol_lexer = Lexer::new("defn");
    let symbol = super::tokenization::lex_symbol(&mut symbol_lexer).unwrap();
    assert_eq!(symbol.kind, TokenKind::Defn);
}
