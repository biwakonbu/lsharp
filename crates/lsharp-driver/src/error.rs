//! L# コンパイラ統一エラー型
//!
//! 各クレートのエラー型を統一的に扱うための列挙型。
//! thiserror の `#[from]` を使い、自動的な From 変換を提供する。
#![allow(dead_code)]

use thiserror::Error;

/// コンパイラパイプライン全体の統一エラー型
#[derive(Debug, Error)]
pub enum LsharpError {
    /// 字句解析エラー
    #[error(transparent)]
    Lex(#[from] lsharp_syntax::lexer::LexError),

    /// 構文解析エラー
    #[error(transparent)]
    Parse(#[from] lsharp_syntax::parser::ParseError),

    /// 型推論エラー
    #[error(transparent)]
    Type(#[from] lsharp_types::infer::TypeError),

    /// 制約エラー
    #[error(transparent)]
    Constraint(#[from] lsharp_types::constraints::ConstraintError),

    /// IR 変換エラー
    #[error(transparent)]
    Lower(#[from] lsharp_ir::lower::LowerError),

    /// コード生成エラー
    #[error(transparent)]
    Codegen(#[from] lsharp_wasm::codegen::CodegenError),

    /// モジュールグラフエラー
    #[error(transparent)]
    ModuleGraph(#[from] lsharp_ir::module_graph::ModuleGraphError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsharp_syntax::span::Span;

    #[test]
    fn test_from_lex_error() {
        let err = lsharp_syntax::lexer::LexError::UnexpectedChar {
            ch: '@',
            span: Span::new(0, 1),
        };
        let unified: LsharpError = err.into();
        assert!(matches!(unified, LsharpError::Lex(_)));
        assert!(unified.to_string().contains('@'));
    }

    #[test]
    fn test_from_parse_error() {
        let err = lsharp_syntax::parser::ParseError::UnexpectedEof {
            expected: "expression".to_string(),
        };
        let unified: LsharpError = err.into();
        assert!(matches!(unified, LsharpError::Parse(_)));
        assert!(unified.to_string().contains("expression"));
    }

    #[test]
    fn test_from_type_error() {
        use lsharp_types::types::Type;
        let err = lsharp_types::infer::TypeError::Mismatch {
            expected: Type::Con("Int".to_string()),
            found: Type::Con("Bool".to_string()),
            span: Span::new(0, 1),
        };
        let unified: LsharpError = err.into();
        assert!(matches!(unified, LsharpError::Type(_)));
        assert!(unified.to_string().contains("Int"));
    }

    #[test]
    fn test_from_constraint_error() {
        let err = lsharp_types::constraints::ConstraintError::Violation {
            constraint: "positive".to_string(),
            value: "-1".to_string(),
        };
        let unified: LsharpError = err.into();
        assert!(matches!(unified, LsharpError::Constraint(_)));
        assert!(unified.to_string().contains("positive"));
    }

    #[test]
    fn test_from_lower_error() {
        let err = lsharp_ir::lower::LowerError::UndefinedFunction {
            name: "foo".to_string(),
        };
        let unified: LsharpError = err.into();
        assert!(matches!(unified, LsharpError::Lower(_)));
        assert!(unified.to_string().contains("foo"));
    }

    #[test]
    fn test_from_codegen_error() {
        let err = lsharp_wasm::codegen::CodegenError::Error {
            msg: "out of memory".to_string(),
        };
        let unified: LsharpError = err.into();
        assert!(matches!(unified, LsharpError::Codegen(_)));
        assert!(unified.to_string().contains("out of memory"));
    }

    #[test]
    fn test_from_module_graph_error() {
        let err = lsharp_ir::module_graph::ModuleGraphError::DuplicateModule {
            name: "Foo".to_string(),
        };
        let unified: LsharpError = err.into();
        assert!(matches!(unified, LsharpError::ModuleGraph(_)));
        assert!(unified.to_string().contains("Foo"));
    }
}
