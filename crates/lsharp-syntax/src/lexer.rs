use crate::span::Span;
use crate::token::{Token, TokenKind};

mod dispatch;
mod tokenization;

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
mod dispatch_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tokenization_tests;
#[cfg(test)]
mod utf8_comment_tests;
#[cfg(test)]
mod utf8_tests;
