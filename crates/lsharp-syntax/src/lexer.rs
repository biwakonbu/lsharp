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

    fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments();

        if self.pos >= self.bytes.len() {
            return Ok(Token::new(TokenKind::Eof, Span::new(self.pos, self.pos)));
        }

        let start = self.pos;
        let ch = self.bytes[self.pos] as char;

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
                span: Span::new(start, start + 1),
            }),
        }
    }

    /// 空白とコメントをスキップ
    fn skip_whitespace_and_comments(&mut self) {
        while self.pos < self.bytes.len() {
            let ch = self.bytes[self.pos] as char;
            if ch.is_ascii_whitespace() {
                self.pos += 1;
            } else if ch == ';' {
                // 行コメント: 行末までスキップ
                while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    /// 文字列リテラルの字句解析
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
            let ch = self.bytes[self.pos] as char;
            match ch {
                '"' => {
                    self.pos += 1;
                    return Ok(Token::new(
                        TokenKind::String(value),
                        Span::new(start, self.pos),
                    ));
                }
                '\\' => {
                    self.pos += 1;
                    if self.pos >= self.bytes.len() {
                        return Err(LexError::UnterminatedString {
                            span: Span::new(start, self.pos),
                        });
                    }
                    let escaped = self.bytes[self.pos] as char;
                    match escaped {
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        'r' => value.push('\r'),
                        '\\' => value.push('\\'),
                        '"' => value.push('"'),
                        _ => {
                            value.push('\\');
                            value.push(escaped);
                        }
                    }
                    self.pos += 1;
                }
                _ => {
                    value.push(ch);
                    self.pos += 1;
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

    /// シンボル/キーワードの字句解析
    fn lex_symbol(&mut self) -> Result<Token, LexError> {
        let start = self.pos;
        while self.pos < self.bytes.len() && is_symbol_char(self.bytes[self.pos] as char) {
            self.pos += 1;
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
            "true" => TokenKind::Bool(true),
            "false" => TokenKind::Bool(false),
            _ => TokenKind::Symbol(text.to_string()),
        };

        Ok(Token::new(kind, span))
    }

    /// 次の文字を覗き見る
    fn peek_next(&self) -> Option<char> {
        self.bytes.get(self.pos + 1).map(|&b| b as char)
    }
}

/// シンボルの開始文字として有効か
fn is_symbol_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
        || matches!(
            ch,
            '_' | '+' | '-' | '*' | '/' | '=' | '<' | '>' | '!' | '?' | '&' | '|' | '%' | '^'
            | '~' | '@'
        )
}

/// シンボルの継続文字として有効か
fn is_symbol_char(ch: char) -> bool {
    is_symbol_start(ch) || ch.is_ascii_digit() || matches!(ch, '.' | '-')
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
        let tokens = lex("3.14");
        assert_eq!(tokens, vec![TokenKind::Float(3.14), TokenKind::Eof]);
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
            vec![TokenKind::Bool(true), TokenKind::Bool(false), TokenKind::Eof]
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
}
