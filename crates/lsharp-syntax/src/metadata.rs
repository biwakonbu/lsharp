use crate::ast::Expr;
use crate::span::Span;

/// source 上の順序と範囲を保持した metadata form。
///
/// 既存の [`crate::ast::Metadata`] field は互換 projection として残し、
/// canonical contract IR への変換はこの lossless form を入力にする。
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataForm {
    span: Span,
    pub kind: MetadataFormKind,
}

impl MetadataForm {
    pub fn new(span: Span, kind: MetadataFormKind) -> Self {
        Self { span, kind }
    }

    /// directive の `:` から payload 末尾までの source span。
    pub fn span(&self) -> Span {
        self.span
    }
}

/// canonical `:case` 内の `(expect actual expected)` entry。
#[derive(Debug, Clone, PartialEq)]
pub struct CaseExpectation {
    source_span: Span,
    actual: Expr,
    expected: Expr,
}

impl CaseExpectation {
    pub(crate) fn new(source_span: Span, actual: Expr, expected: Expr) -> Self {
        Self {
            source_span,
            actual,
            expected,
        }
    }

    pub fn source_span(&self) -> Span {
        self.source_span
    }

    pub fn actual(&self) -> &Expr {
        &self.actual
    }

    pub fn expected(&self) -> &Expr {
        &self.expected
    }
}

/// source から lossless に保持する contract form。
#[derive(Debug, Clone, PartialEq)]
pub enum MetadataFormKind {
    /// legacy `:example [expr ...]`。一つの directive 内の grouping を維持する。
    LegacyExample { expressions: Vec<Expr> },
    /// legacy `:invariant predicate`。
    LegacyInvariant { predicate: Expr },
    /// canonical `:case [(expect actual expected) ...]`。
    Case { expectations: Vec<CaseExpectation> },
    /// canonical `:assert [predicate ...]`。一つの directive 内の grouping を維持する。
    Assertion { predicates: Vec<Expr> },
}
