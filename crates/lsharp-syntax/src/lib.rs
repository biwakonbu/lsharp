pub mod ast;
pub mod derive;
pub mod hygiene;
pub mod lexer;
pub mod macro_expand;
pub mod metadata;
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

impl ParseAllError {
    /// パイプライン全体で利用する安定した診断コードを返す。
    pub fn code(&self) -> &'static str {
        match self {
            Self::Lex(error) => error.code(),
            Self::Parse(error) => error.code(),
            Self::MacroExpand(error) => error.code(),
        }
    }

    /// パイプライン全体で利用する source span を返す。
    pub fn span(&self) -> Option<span::Span> {
        match self {
            Self::Lex(error) => error.span(),
            Self::Parse(error) => error.span(),
            Self::MacroExpand(error) => error.span(),
        }
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::{ParseAllError, parse};
    use crate::lexer::Lexer;
    use crate::span::Span;

    #[test]
    fn lexer_error_exposes_stable_code_and_span() {
        let error = Lexer::new("@")
            .tokenize()
            .expect_err("未知文字は失敗するべき");

        assert_eq!(error.code(), "LS0001");
        assert_eq!(error.span(), Some(Span::new(0, 1)));
    }

    #[test]
    fn parser_error_exposes_stable_code_and_span() {
        let error = parse("(unknown-form)").expect_err("未知 form は失敗するべき");
        let ParseAllError::Parse(error) = error else {
            panic!("parser error を期待しました");
        };

        assert_eq!(error.code(), "LS0103");
        assert_eq!(error.span(), Some(Span::new(1, 13)));
    }

    #[test]
    fn parse_all_error_forwards_code_and_span() {
        let error = parse("@").expect_err("lexer error は失敗するべき");

        assert_eq!(error.code(), "LS0001");
        assert_eq!(error.span(), Some(Span::new(0, 1)));
    }

    #[test]
    fn macro_expand_error_exposes_stable_code_and_span() {
        let error =
            ParseAllError::MacroExpand(crate::macro_expand::MacroExpandError::SpliceOutsideList {
                span: Span::new(4, 6),
            });

        assert_eq!(error.code(), "LS0201");
        assert_eq!(error.span(), Some(Span::new(4, 6)));
    }
}

#[cfg(test)]
mod property_tests {
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn parser_never_panics_for_bounded_arbitrary_bytes(
            bytes in prop::collection::vec(any::<u8>(), 0..128)
        ) {
            let source = String::from_utf8_lossy(&bytes);
            let result = std::panic::catch_unwind(|| super::parse(&source));
            prop_assert!(result.is_ok(), "parser panicked for generated input");
        }
    }
}
