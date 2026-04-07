use crate::span::Span;
use crate::token::{Token, TokenKind};

/// 字句解析エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum LexError {
    #[error("予期しない文字 '{ch}' ({span})")]
    UnexpectedChar { ch: char, span: Span },

    #[error("閉じられていない文字列リテラル ({span})")]
    UnterminatedString { span: Span },

    #[error("不正な数値リテラル '{text}' ({span})")]
    InvalidNumber { text: String, span: Span },
}

/// 字句解析器
pub struct Lexer<'src> {
    source: &'src str,
    bytes: &'src [u8],
    pos: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
        }
    }

    /// 全トークンを生成（Eof 含む）
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    /// 現在位置の char を UTF-8 として取得
    fn current_char(&self) -> Option<char> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        self.source[self.pos..].chars().next()
    }

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments();

        if self.pos >= self.bytes.len() {
            return Ok(Token::new(TokenKind::Eof, Span::new(self.pos, self.pos)));
        }

        let start = self.pos;
        let ch = self.current_char().unwrap();

        match ch {
            '(' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::LParen, Span::new(start, self.pos)))
            }
            ')' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::RParen, Span::new(start, self.pos)))
            }
            '[' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::LBracket, Span::new(start, self.pos)))
            }
            ']' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::RBracket, Span::new(start, self.pos)))
            }
            '{' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::LBrace, Span::new(start, self.pos)))
            }
            '}' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::RBrace, Span::new(start, self.pos)))
            }
            '|' => {
                self.pos += 1;
                // P10-5: |> パイプライン演算子をシンボルとして扱う
                if self.pos < self.bytes.len() && self.bytes[self.pos] == b'>' {
                    self.pos += 1;
                    Ok(Token::new(
                        TokenKind::Symbol("|>".to_string()),
                        Span::new(start, self.pos),
                    ))
                } else {
                    Ok(Token::new(TokenKind::Pipe, Span::new(start, self.pos)))
                }
            }
            // P10-1: Quote トークン
            '\'' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Quote, Span::new(start, self.pos)))
            }
            // P10-1: Unquote / SpliceUnquote トークン
            '~' => {
                self.pos += 1;
                // ~@ の場合は SpliceUnquote
                if self.pos < self.bytes.len() && self.bytes[self.pos] == b'@' {
                    self.pos += 1;
                    Ok(Token::new(
                        TokenKind::SpliceUnquote,
                        Span::new(start, self.pos),
                    ))
                } else {
                    Ok(Token::new(TokenKind::Unquote, Span::new(start, self.pos)))
                }
            }
            '"' => self.lex_string(),
            _ if ch.is_ascii_digit() => self.lex_number(),
            '-' if self.peek_next().is_some_and(|c| c == '>') => {
                self.pos += 2;
                Ok(Token::new(TokenKind::Arrow, Span::new(start, self.pos)))
            }
            '-' if self.peek_next().is_some_and(|c| c.is_ascii_digit()) => self.lex_number(),
            ':' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Colon, Span::new(start, self.pos)))
            }
            _ if is_symbol_start(ch) => self.lex_symbol(),
            _ => Err(LexError::UnexpectedChar {
                ch,
                span: Span::new(start, start + ch.len_utf8()),
            }),
        }
    }

    /// 空白とコメントをスキップ
    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.bytes.len() {
            let ch = self.bytes[self.pos];
            if ch.is_ascii_whitespace() {
                self.pos += 1;
            } else if ch == b';' {
                // 行コメント: 行末までスキップ
                while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    /// 文字列リテラルの字句解析（UTF-8 対応）
    fn lex_string(&mut self) -> Result<Token, LexError> {
        let start = self.pos;
        self.pos += 1; // 開始の `"` をスキップ

        let mut value = String::new();
        loop {
            if self.pos >= self.bytes.len() {
                return Err(LexError::UnterminatedString {
                    span: Span::new(start, self.pos),
                });
            }
            let byte = self.bytes[self.pos];
            match byte {
                b'"' => {
                    self.pos += 1;
                    return Ok(Token::new(
                        TokenKind::String(value),
                        Span::new(start, self.pos),
                    ));
                }
                b'\\' => {
                    self.pos += 1;
                    if self.pos >= self.bytes.len() {
                        return Err(LexError::UnterminatedString {
                            span: Span::new(start, self.pos),
                        });
                    }
                    let escaped = self.bytes[self.pos];
                    match escaped {
                        b'n' => value.push('\n'),
                        b't' => value.push('\t'),
                        b'r' => value.push('\r'),
                        b'\\' => value.push('\\'),
                        b'"' => value.push('"'),
                        _ => {
                            value.push('\\');
                            value.push(escaped as char);
                        }
                    }
                    self.pos += 1;
                }
                _ => {
                    // UTF-8 マルチバイト文字の処理
                    let remaining = &self.source[self.pos..];
                    if let Some(ch) = remaining.chars().next() {
                        value.push(ch);
                        self.pos += ch.len_utf8();
                    } else {
                        // 不正な UTF-8（通常到達しない）
                        self.pos += 1;
                    }
                }
            }
        }
    }

    /// 数値リテラルの字句解析
    fn lex_number(&mut self) -> Result<Token, LexError> {
        let start = self.pos;
        let mut is_float = false;

        // 負号
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'-' {
            self.pos += 1;
        }

        // 整数部
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }

        // 小数部
        if self.pos < self.bytes.len()
            && self.bytes[self.pos] == b'.'
            && self.pos + 1 < self.bytes.len()
            && self.bytes[self.pos + 1].is_ascii_digit()
        {
            is_float = true;
            self.pos += 1; // '.'
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        let text = &self.source[start..self.pos];
        let span = Span::new(start, self.pos);

        if is_float {
            text.parse::<f64>()
                .map(|n| Token::new(TokenKind::Float(n), span))
                .map_err(|_| LexError::InvalidNumber {
                    text: text.to_string(),
                    span,
                })
        } else {
            text.parse::<i64>()
                .map(|n| Token::new(TokenKind::Int(n), span))
                .map_err(|_| LexError::InvalidNumber {
                    text: text.to_string(),
                    span,
                })
        }
    }

    /// シンボル/キーワードの字句解析（UTF-8 マルチバイト対応）
    fn lex_symbol(&mut self) -> Result<Token, LexError> {
        let start = self.pos;
        while self.pos < self.bytes.len() {
            if let Some(ch) = self.source[self.pos..].chars().next() {
                if is_symbol_char(ch) {
                    self.pos += ch.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let text = &self.source[start..self.pos];
        let span = Span::new(start, self.pos);

        let kind = match text {
            "defn" => TokenKind::Defn,
            "let" => TokenKind::Let,
            "if" => TokenKind::If,
            "match" => TokenKind::Match,
            "type" => TokenKind::Type,
            "fn" => TokenKind::Fn,
            "do" => TokenKind::Do,
            "module" => TokenKind::Module,
            "import" => TokenKind::Import,
            "record" => TokenKind::Record,
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "where" => TokenKind::Where,
            "type-alias" => TokenKind::TypeAlias,
            "type-constrained" => TokenKind::TypeConstrained,
            "constraints" => TokenKind::Constraints,
            "private" => TokenKind::Private,
            "computation" => TokenKind::Computation,
            "computation-builder" => TokenKind::ComputationBuilder,
            "defmacro" => TokenKind::DefMacro,
            "true" => TokenKind::Bool(true),
            "false" => TokenKind::Bool(false),
            _ => TokenKind::Symbol(text.to_string()),
        };

        Ok(Token::new(kind, span))
    }

    /// 次の文字を覗き見る（UTF-8 対応）
    fn peek_next(&self) -> Option<char> {
        if self.pos >= self.bytes.len() {
            return None;
        }
        // 現在の char の次の char を取得
        if let Some(ch) = self.source[self.pos..].chars().next() {
            let next_pos = self.pos + ch.len_utf8();
            self.source[next_pos..].chars().next()
        } else {
            None
        }
    }
}

/// シンボルの開始文字として有効か（マルチバイト対応）
/// 注意: '~' と '@' はマクロトークンとして使用するため除外
fn is_symbol_start(ch: char) -> bool {
    ch.is_alphabetic()
        || matches!(
            ch,
            '_' | '+' | '-' | '*' | '/' | '=' | '<' | '>' | '!' | '?' | '&' | '%' | '^'
        )
}

/// シンボルの継続文字として有効か（マルチバイト対応）
fn is_symbol_char(ch: char) -> bool {
    is_symbol_start(ch) || ch.is_ascii_digit() || matches!(ch, '.' | '-' | '~' | '@')
}

#[cfg(test)]
mod tests {
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
}

#[cfg(test)]
mod utf8_tests {
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
}

#[cfg(test)]
mod utf8_comment_tests {
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
}
