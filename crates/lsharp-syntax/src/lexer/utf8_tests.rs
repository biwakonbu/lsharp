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
fn test_japanese_string_literal() {
    let tokens = lex("\"こんにちは\"");
    assert_eq!(
        tokens,
        vec![TokenKind::String("こんにちは".to_string()), TokenKind::Eof]
    );
}

#[test]
fn test_mixed_utf8_string() {
    let tokens = lex("\"hello 世界\"");
    assert_eq!(
        tokens,
        vec![TokenKind::String("hello 世界".to_string()), TokenKind::Eof]
    );
}

#[test]
fn test_emoji_string() {
    let tokens = lex("\"test 🦀\"");
    assert_eq!(
        tokens,
        vec![TokenKind::String("test 🦀".to_string()), TokenKind::Eof]
    );
}

// マルチバイトシンボル名のテスト

#[test]
fn test_japanese_symbol_name() {
    // 日本語のシンボル名
    let tokens = lex("(defn 足し算 [x y] (+ x y))");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LParen,
            TokenKind::Defn,
            TokenKind::Symbol("足し算".to_string()),
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
fn test_japanese_type_name() {
    // 日本語の型名
    let tokens = lex("(type 点 (record (: x Int)))");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LParen,
            TokenKind::Type,
            TokenKind::Symbol("点".to_string()),
            TokenKind::LParen,
            TokenKind::Record,
            TokenKind::LParen,
            TokenKind::Colon,
            TokenKind::Symbol("x".to_string()),
            TokenKind::Symbol("Int".to_string()),
            TokenKind::RParen,
            TokenKind::RParen,
            TokenKind::RParen,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn test_mixed_japanese_ascii_symbol() {
    // 日本語と ASCII が混在するシンボル
    let tokens = lex("add加算");
    assert_eq!(
        tokens,
        vec![TokenKind::Symbol("add加算".to_string()), TokenKind::Eof]
    );
}

#[test]
fn test_chinese_symbol() {
    let tokens = lex("加法");
    assert_eq!(
        tokens,
        vec![TokenKind::Symbol("加法".to_string()), TokenKind::Eof]
    );
}

#[test]
fn test_korean_symbol() {
    let tokens = lex("덧셈");
    assert_eq!(
        tokens,
        vec![TokenKind::Symbol("덧셈".to_string()), TokenKind::Eof]
    );
}

#[test]
fn test_japanese_variable_in_expr() {
    // 日本語変数名を含む式
    let tokens = lex("(+ 値 1)");
    assert_eq!(
        tokens,
        vec![
            TokenKind::LParen,
            TokenKind::Symbol("+".to_string()),
            TokenKind::Symbol("値".to_string()),
            TokenKind::Int(1),
            TokenKind::RParen,
            TokenKind::Eof,
        ]
    );
}
