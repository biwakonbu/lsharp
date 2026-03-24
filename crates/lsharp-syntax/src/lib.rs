pub mod ast;
pub mod derive;
pub mod hygiene;
pub mod lexer;
pub mod macro_expand;
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

/// ソースコードをパースし、マクロ展開済み AST を返す
/// 組み込みマクロ (when, unless) とユーザー定義マクロの両方を展開する
pub fn parse_and_expand(source: &str) -> Result<ast::Program, ParseAllError> {
    let program = parse(source)?;
    let mut expander = macro_expand::MacroExpander::with_builtins();
    expander
        .expand_program(program)
        .map_err(ParseAllError::MacroExpand)
}

/// パース全体のエラー
#[derive(Debug, thiserror::Error)]
pub enum ParseAllError {
    #[error("字句解析エラー: {0}")]
    Lex(#[from] lexer::LexError),
    #[error("構文解析エラー: {0}")]
    Parse(#[from] parser::ParseError),
    #[error("マクロ展開エラー: {0}")]
    MacroExpand(#[from] macro_expand::MacroExpandError),
}
