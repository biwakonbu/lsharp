use super::*;

fn lex(input: &str) -> Vec<TokenKind> {
    let mut lexer = Lexer::new(input);
    lexer
        .tokenize()
        .unwrap()
        .into_iter()
        .map(|t| t.kind)
        .collect()
}

#[test]
fn test_simple_addition() {
    let tokens = lex("(+ 1 2)");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LParen,
            TokenKind::Symbol("+".to_string()),
            TokenKind::Int(1),
            TokenKind::Int(2),
            TokenKind::RParen,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_nested_expr() {
    let tokens = lex("(+ (* 2 3) 4)");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LParen,
            TokenKind::Symbol("+".to_string()),
            TokenKind::LParen,
            TokenKind::Symbol("*".to_string()),
            TokenKind::Int(2),
            TokenKind::Int(3),
            TokenKind::RParen,
            TokenKind::Int(4),
            TokenKind::RParen,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_defn() {
    let tokens = lex("(defn add [x y] (+ x y))");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LParen,
            TokenKind::Defn,
            TokenKind::Symbol("add".to_string()),
            TokenKind::LBracket,
            TokenKind::Symbol("x".to_string()),
            TokenKind::Symbol("y".to_string()),
            TokenKind::RBracket,
            TokenKind::LParen,
            TokenKind::Symbol("+".to_string()),
            TokenKind::Symbol("x".to_string()),
            TokenKind::Symbol("y".to_string()),
            TokenKind::RParen,
            TokenKind::RParen,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_string_literal() {
    let tokens = lex(r#""hello world""#);
    assert_eq!(
        tokens,
        vec![TokenKind::String("hello world".to_string()), TokenKind::Eof]
    );
}

#[test]
fn test_invalid_utf8_after_escape_returns_error_without_panic() {
    let source = String::from_utf8_lossy(&[b'"', b'\\', 0x80]);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut lexer = Lexer::new(&source);
        lexer.tokenize()
    }));

    assert!(result.is_ok(), "invalid UTF-8 escape must not panic");
    assert!(
        result.unwrap().is_err(),
        "unterminated string must be rejected"
    );
}

#[test]
fn test_float() {
    let tokens = lex("3.25");
    assert_eq!(tokens, vec![TokenKind::Float(3.25), TokenKind::Eof]);
}

#[test]
fn test_negative_number() {
    let tokens = lex("-42");
    assert_eq!(tokens, vec![TokenKind::Int(-42), TokenKind::Eof]);
}

#[test]
fn test_comment() {
    let tokens = lex("; これはコメント\n42");
    assert_eq!(tokens, vec![TokenKind::Int(42), TokenKind::Eof]);
}

#[test]
fn test_bool() {
    let tokens = lex("true false");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Bool(true),
            TokenKind::Bool(false),
            TokenKind::Eof
        ]
    );
}

#[test]
fn test_arrow() {
    let tokens = lex("->");
    assert_eq!(tokens, vec![TokenKind::Arrow, TokenKind::Eof]);
}

#[test]
fn test_type_annotation() {
    let tokens = lex("(: x Int)");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LParen,
            TokenKind::Colon,
            TokenKind::Symbol("x".to_string()),
            TokenKind::Symbol("Int".to_string()),
            TokenKind::RParen,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_record_keyword() {
    let tokens = lex("(type Point (record (: x Float) (: y Float)))");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LParen,
            TokenKind::Type,
            TokenKind::Symbol("Point".to_string()),
            TokenKind::LParen,
            TokenKind::Record,
            TokenKind::LParen,
            TokenKind::Colon,
            TokenKind::Symbol("x".to_string()),
            TokenKind::Symbol("Float".to_string()),
            TokenKind::RParen,
            TokenKind::LParen,
            TokenKind::Colon,
            TokenKind::Symbol("y".to_string()),
            TokenKind::Symbol("Float".to_string()),
            TokenKind::RParen,
            TokenKind::RParen,
            TokenKind::RParen,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_trait_impl_keywords() {
    let tokens = lex("trait impl where");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Trait,
            TokenKind::Impl,
            TokenKind::Where,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_type_alias_keyword() {
    let tokens = lex("type-alias");
    assert_eq!(tokens, vec![TokenKind::TypeAlias, TokenKind::Eof]);
}

#[test]
fn test_pipe_token() {
    let tokens = lex("{x | a 1}");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LBrace,
            TokenKind::Symbol("x".to_string()),
            TokenKind::Pipe,
            TokenKind::Symbol("a".to_string()),
            TokenKind::Int(1),
            TokenKind::RBrace,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_dot_token() {
    let tokens = lex("(. point x)");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LParen,
            TokenKind::Dot,
            TokenKind::Symbol("point".to_string()),
            TokenKind::Symbol("x".to_string()),
            TokenKind::RParen,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_type_constrained_keyword() {
    let tokens = lex("type-constrained");
    assert_eq!(tokens, vec![TokenKind::TypeConstrained, TokenKind::Eof]);
}

#[test]
fn test_constraints_keyword() {
    let tokens = lex("constraints");
    assert_eq!(tokens, vec![TokenKind::Constraints, TokenKind::Eof]);
}

#[test]
fn test_private_keyword() {
    let tokens = lex("private");
    assert_eq!(tokens, vec![TokenKind::Private, TokenKind::Eof]);
}

// P10-1: Quote/Unquote トークンテスト

#[test]
fn test_quote_token() {
    let tokens = lex("'(+ 1 2)");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Quote,
            TokenKind::LParen,
            TokenKind::Symbol("+".to_string()),
            TokenKind::Int(1),
            TokenKind::Int(2),
            TokenKind::RParen,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_unquote_token() {
    let tokens = lex("~x");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Unquote,
            TokenKind::Symbol("x".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_splice_unquote_token() {
    let tokens = lex("~@args");
    assert_eq!(
        tokens,
        vec![
            TokenKind::SpliceUnquote,
            TokenKind::Symbol("args".to_string()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_quote_in_expression() {
    let tokens = lex("(defmacro m [x] '(+ ~x 1))");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LParen,
            TokenKind::DefMacro,
            TokenKind::Symbol("m".to_string()),
            TokenKind::LBracket,
            TokenKind::Symbol("x".to_string()),
            TokenKind::RBracket,
            TokenKind::Quote,
            TokenKind::LParen,
            TokenKind::Symbol("+".to_string()),
            TokenKind::Unquote,
            TokenKind::Symbol("x".to_string()),
            TokenKind::Int(1),
            TokenKind::RParen,
            TokenKind::RParen,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_splice_unquote_in_list() {
    let tokens = lex("'(a ~@rest b)");
    assert_eq!(
        tokens,
        vec![
            TokenKind::Quote,
            TokenKind::LParen,
            TokenKind::Symbol("a".to_string()),
            TokenKind::SpliceUnquote,
            TokenKind::Symbol("rest".to_string()),
            TokenKind::Symbol("b".to_string()),
            TokenKind::RParen,
            TokenKind::Eof,
        ]
    );
}
