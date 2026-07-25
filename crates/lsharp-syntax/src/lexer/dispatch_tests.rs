use crate::token::TokenKind;

use super::Lexer;

#[test]
fn dispatch_module_exposes_delimiter_and_operator_scanner() {
    let mut lexer = Lexer::new("(|> -> ~@)");
    let first = lexer.next_token().unwrap();
    let second = lexer.next_token().unwrap();
    let third = lexer.next_token().unwrap();
    let fourth = lexer.next_token().unwrap();

    assert!(matches!(first.kind, TokenKind::LParen));
    assert!(matches!(second.kind, TokenKind::Symbol(ref name) if name == "|>"));
    assert!(matches!(third.kind, TokenKind::Arrow));
    assert!(matches!(fourth.kind, TokenKind::SpliceUnquote));
}
