use crate::ast::{Expr, TypeExpr};
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

/// canonical `:property` の binder。
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyBinder {
    source_span: Span,
    name: String,
    ty: TypeExpr,
}

impl PropertyBinder {
    pub(crate) fn new(source_span: Span, name: String, ty: TypeExpr) -> Self {
        Self {
            source_span,
            name,
            ty,
        }
    }

    pub fn source_span(&self) -> Span {
        self.source_span
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ty(&self) -> &TypeExpr {
        &self.ty
    }
}

/// `:property` 内の `(for-all ...)` declaration。
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyForm {
    source_span: Span,
    binders: Vec<PropertyBinder>,
    preconditions: Vec<Expr>,
    postcondition: Expr,
    cases: Option<usize>,
    seed: Option<u64>,
    shrink: Option<bool>,
}

impl PropertyForm {
    pub(crate) fn new(
        source_span: Span,
        binders: Vec<PropertyBinder>,
        preconditions: Vec<Expr>,
        postcondition: Expr,
        cases: Option<usize>,
        seed: Option<u64>,
        shrink: Option<bool>,
    ) -> Self {
        Self {
            source_span,
            binders,
            preconditions,
            postcondition,
            cases,
            seed,
            shrink,
        }
    }

    pub fn source_span(&self) -> Span {
        self.source_span
    }

    pub fn binders(&self) -> &[PropertyBinder] {
        &self.binders
    }

    pub fn preconditions(&self) -> &[Expr] {
        &self.preconditions
    }

    pub fn postcondition(&self) -> &Expr {
        &self.postcondition
    }

    pub fn cases(&self) -> Option<usize> {
        self.cases
    }

    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    pub fn shrink(&self) -> Option<bool> {
        self.shrink
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
    /// canonical `:property [(for-all ...)]`。declaration grouping を維持する。
    Property { properties: Vec<PropertyForm> },
}
