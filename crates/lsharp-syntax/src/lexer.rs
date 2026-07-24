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

impl LexError {
    /// 利用者向けの安定した診断コードを返す。
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnexpectedChar { .. } => "LS0001",
            Self::UnterminatedString { .. } => "LS0002",
            Self::InvalidNumber { .. } => "LS0003",
        }
    }

    /// 診断に対応する source span を返す。
    pub fn span(&self) -> Option<Span> {
        Some(match self {
            Self::UnexpectedChar { span, .. }
            | Self::UnterminatedString { span }
            | Self::InvalidNumber { span, .. } => *span,
        })
    }
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
                            // 未知 escape は従来どおり文字列へ残すが、非 ASCII の場合は
                            // UTF-8 の先頭 byte だけを消費して char boundary を壊さない。
                            let remaining = &self.source[self.pos..];
                            if let Some(ch) = remaining.chars().next() {
                                value.push(ch);
                                self.pos += ch.len_utf8();
                            } else {
                                self.pos += 1;
                            }
                            continue;
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
mod tests;
#[cfg(test)]
mod utf8_comment_tests;
#[cfg(test)]
mod utf8_tests;
