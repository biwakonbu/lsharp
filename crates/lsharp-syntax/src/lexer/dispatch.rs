use crate::span::Span;
use crate::token::{Token, TokenKind};

use super::{LexError, Lexer, is_symbol_start, tokenization};

impl Lexer<'_> {
    pub(super) fn next_token(&mut self) -> Result<Token, LexError> {
        tokenization::skip_whitespace_and_comments(self);

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
            '.' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Dot, Span::new(start, self.pos)))
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
            '"' => tokenization::lex_string(self),
            _ if ch.is_ascii_digit() => tokenization::lex_number(self),
            '-' if self.peek_next().is_some_and(|c| c == '>') => {
                self.pos += 2;
                Ok(Token::new(TokenKind::Arrow, Span::new(start, self.pos)))
            }
            '-' if self.peek_next().is_some_and(|c| c.is_ascii_digit()) => {
                tokenization::lex_number(self)
            }
            ':' => {
                self.pos += 1;
                Ok(Token::new(TokenKind::Colon, Span::new(start, self.pos)))
            }
            _ if is_symbol_start(ch) => tokenization::lex_symbol(self),
            _ => Err(LexError::UnexpectedChar {
                ch,
                span: Span::new(start, start + ch.len_utf8()),
            }),
        }
    }
}
