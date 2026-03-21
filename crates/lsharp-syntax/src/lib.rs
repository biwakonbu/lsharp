pub mod ast;
pub mod lexer;
pub mod parser;
pub mod span;
pub mod token;

use lexer::Lexer;
use parser::Parser;

/// ソースコードをパースして AST を返す
pub fn parse(source: &str) -> Result<ast::Program, ParseAllError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(ParseAllError::Lex)?;
    let mut parser = Parser::new(tokens);
    parser.parse_program().map_err(ParseAllError::Parse)
}

/// パース全体のエラー
#[derive(Debug, thiserror::Error)]
pub enum ParseAllError {
    #[error("字句解析エラー: {0}")]
    Lex(#[from] lexer::LexError),
    #[error("構文解析エラー: {0}")]
    Parse(#[from] parser::ParseError),
}
