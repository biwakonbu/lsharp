use super::*;

fn lex(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    lexer.tokenize().unwrap()
}

#[test]
fn test_comment_with_japanese() {
    // 日本語コメントが正しくスキップされる
    let tokens = lex("; これは日本語のコメントです\n42");
    // 42 + Eof
    assert_eq!(tokens.len(), 2);
    assert!(matches!(tokens[0].kind, TokenKind::Int(42)));
}

#[test]
fn test_comment_with_emoji() {
    let tokens = lex("; Hello World! 🎉🎊\n(+ 1 2)");
    // (, +, 1, 2, ), Eof
    assert_eq!(tokens.len(), 6);
}

#[test]
fn test_comment_with_mixed_multibyte() {
    let tokens = lex("; CJK: 中文 한국어 日本語\n100");
    // 100 + Eof
    assert_eq!(tokens.len(), 2);
    assert!(matches!(tokens[0].kind, TokenKind::Int(100)));
}

#[test]
fn test_multiple_japanese_comments() {
    let tokens = lex("; 関数定義\n(defn add ; 加算\n  [x y] ; 引数\n  (+ x y)) ; 本体");
    // (, defn, add, [, x, y, ], (, +, x, y, ), ), Eof
    assert!(tokens.len() >= 4);
    assert!(matches!(tokens[0].kind, TokenKind::LParen));
}
