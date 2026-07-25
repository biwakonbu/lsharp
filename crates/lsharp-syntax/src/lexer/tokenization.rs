use crate::span::Span;
use crate::token::{Token, TokenKind};

use super::{LexError, Lexer, is_symbol_char};

/// 空白とコメントをスキップ
pub(super) fn skip_whitespace_and_comments(lexer: &mut Lexer<'_>) {
    while lexer.pos < lexer.bytes.len() {
        let ch = lexer.bytes[lexer.pos];
        if ch.is_ascii_whitespace() {
            lexer.pos += 1;
        } else if ch == b';' {
            // 行コメント: 行末までスキップ
            while lexer.pos < lexer.bytes.len() && lexer.bytes[lexer.pos] != b'\n' {
                lexer.pos += 1;
            }
        } else {
            break;
        }
    }
}

/// 文字列リテラルの字句解析（UTF-8 対応）
pub(super) fn lex_string(lexer: &mut Lexer<'_>) -> Result<Token, LexError> {
    let start = lexer.pos;
    lexer.pos += 1; // 開始の `"` をスキップ

    let mut value = String::new();
    loop {
        if lexer.pos >= lexer.bytes.len() {
            return Err(LexError::UnterminatedString {
                span: Span::new(start, lexer.pos),
            });
        }
        let byte = lexer.bytes[lexer.pos];
        match byte {
            b'"' => {
                lexer.pos += 1;
                return Ok(Token::new(
                    TokenKind::String(value),
                    Span::new(start, lexer.pos),
                ));
            }
            b'\\' => {
                lexer.pos += 1;
                if lexer.pos >= lexer.bytes.len() {
                    return Err(LexError::UnterminatedString {
                        span: Span::new(start, lexer.pos),
                    });
                }
                let escaped = lexer.bytes[lexer.pos];
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
                        let remaining = &lexer.source[lexer.pos..];
                        if let Some(ch) = remaining.chars().next() {
                            value.push(ch);
                            lexer.pos += ch.len_utf8();
                        } else {
                            lexer.pos += 1;
                        }
                        continue;
                    }
                }
                lexer.pos += 1;
            }
            _ => {
                // UTF-8 マルチバイト文字の処理
                let remaining = &lexer.source[lexer.pos..];
                if let Some(ch) = remaining.chars().next() {
                    value.push(ch);
                    lexer.pos += ch.len_utf8();
                } else {
                    // 不正な UTF-8（通常到達しない）
                    lexer.pos += 1;
                }
            }
        }
    }
}

/// 数値リテラルの字句解析
pub(super) fn lex_number(lexer: &mut Lexer<'_>) -> Result<Token, LexError> {
    let start = lexer.pos;
    let mut is_float = false;

    // 負号
    if lexer.pos < lexer.bytes.len() && lexer.bytes[lexer.pos] == b'-' {
        lexer.pos += 1;
    }

    // 整数部
    while lexer.pos < lexer.bytes.len() && lexer.bytes[lexer.pos].is_ascii_digit() {
        lexer.pos += 1;
    }

    // 小数部
    if lexer.pos < lexer.bytes.len()
        && lexer.bytes[lexer.pos] == b'.'
        && lexer.pos + 1 < lexer.bytes.len()
        && lexer.bytes[lexer.pos + 1].is_ascii_digit()
    {
        is_float = true;
        lexer.pos += 1; // '.'
        while lexer.pos < lexer.bytes.len() && lexer.bytes[lexer.pos].is_ascii_digit() {
            lexer.pos += 1;
        }
    }

    let text = &lexer.source[start..lexer.pos];
    let span = Span::new(start, lexer.pos);

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
pub(super) fn lex_symbol(lexer: &mut Lexer<'_>) -> Result<Token, LexError> {
    let start = lexer.pos;
    while lexer.pos < lexer.bytes.len() {
        if let Some(ch) = lexer.source[lexer.pos..].chars().next() {
            if is_symbol_char(ch) {
                lexer.pos += ch.len_utf8();
            } else {
                break;
            }
        } else {
            break;
        }
    }

    let text = &lexer.source[start..lexer.pos];
    let span = Span::new(start, lexer.pos);

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
