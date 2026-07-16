//! v0.2 canonical contract IR と legacy metadata inventory。
//!
//! legacy form は migration classifier が判断するまで canonical docs / executable
//! contract へ変換せず、`pending_migration` に lossless なまま保持する。

use crate::types::Type;
use lsharp_syntax::ast::{Decl, Expr, Metadata, Program};
use lsharp_syntax::metadata::MetadataFormKind;
use lsharp_syntax::span::Span;

/// M1 の owner identifier。現 slice は parser が保持する関数名を格納し、
/// module-qualified name の解決は後続 inventory slice で追加する。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SymbolId(String);

impl SymbolId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// M2 で発番方針を決める intent identifier。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntentId(String);

/// M2 で発番方針を決める claim identifier。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClaimId(String);

/// docs-only canonical example。
#[derive(Debug, Clone, PartialEq)]
pub struct Example {
    expression: Expr,
    source_span: Span,
}

impl Example {
    pub fn expression(&self) -> &Expr {
        &self.expression
    }

    pub fn source_span(&self) -> Span {
        self.source_span
    }
}

/// concrete case の expected outcome。
#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedOutcome {
    Value(Expr),
    DiagnosticCode(String),
}

/// explicit input/output または expected diagnostic を持つ canonical case。
#[derive(Debug, Clone, PartialEq)]
pub struct Case {
    actual: Expr,
    expected: ExpectedOutcome,
    source_span: Span,
}

impl Case {
    pub fn actual(&self) -> &Expr {
        &self.actual
    }

    pub fn expected(&self) -> &ExpectedOutcome {
        &self.expected
    }

    pub fn source_span(&self) -> Span {
        self.source_span
    }
}

/// Bool 必須の canonical assertion predicate。
#[derive(Debug, Clone, PartialEq)]
pub struct Assertion {
    predicate: Expr,
    source_span: Span,
}

impl Assertion {
    pub fn predicate(&self) -> &Expr {
        &self.predicate
    }

    pub fn source_span(&self) -> Span {
        self.source_span
    }
}

/// property binder が利用する generator plan。
#[derive(Debug, Clone, PartialEq)]
pub enum GeneratorPlan {
    TypeDirected,
    Explicit(Expr),
}

/// canonical property の typed binder。
#[derive(Debug, Clone, PartialEq)]
pub struct Binder {
    name: String,
    ty: Type,
    generator: GeneratorPlan,
}

impl Binder {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn ty(&self) -> &Type {
        &self.ty
    }

    pub fn generator(&self) -> &GeneratorPlan {
        &self.generator
    }
}

/// Bool 必須の canonical predicate。
#[derive(Debug, Clone, PartialEq)]
pub struct Predicate {
    expression: Expr,
    source_span: Span,
}

impl Predicate {
    pub fn expression(&self) -> &Expr {
        &self.expression
    }

    pub fn source_span(&self) -> Span {
        self.source_span
    }
}

/// 再現可能な property sampling plan。
///
/// constructor は M1-05 で generator / shrink / coverage contract を固定してから公開する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplingPlan {
    cases: usize,
    seed: u64,
    generator_version: String,
    shrink: bool,
    coverage_buckets: Vec<String>,
}

impl SamplingPlan {
    pub fn cases(&self) -> usize {
        self.cases
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn generator_version(&self) -> &str {
        &self.generator_version
    }

    pub fn shrink(&self) -> bool {
        self.shrink
    }

    pub fn coverage_buckets(&self) -> &[String] {
        &self.coverage_buckets
    }
}

/// sampled canonical property。sampling は常に必須で、legacy から補完しない。
#[derive(Debug, Clone, PartialEq)]
pub struct Property {
    binders: Vec<Binder>,
    preconditions: Vec<Predicate>,
    postcondition: Predicate,
    sampling: SamplingPlan,
    source_span: Span,
}

impl Property {
    pub fn binders(&self) -> &[Binder] {
        &self.binders
    }

    pub fn preconditions(&self) -> &[Predicate] {
        &self.preconditions
    }

    pub fn postcondition(&self) -> &Predicate {
        &self.postcondition
    }

    pub fn sampling(&self) -> &SamplingPlan {
        &self.sampling
    }

    pub fn source_span(&self) -> Span {
        self.source_span
    }
}

/// executable canonical contract。
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutableContract {
    Case(Case),
    Assertion(Assertion),
    Property(Property),
}

/// migration classifier が未判定の v0.1 form。
#[derive(Debug, Clone, PartialEq)]
pub enum LegacyContract {
    Example {
        expressions: Vec<Expr>,
        source_span: Span,
    },
    Invariant {
        predicate: Expr,
        source_span: Span,
    },
}

impl LegacyContract {
    pub fn source_span(&self) -> Span {
        match self {
            Self::Example { source_span, .. } | Self::Invariant { source_span, .. } => *source_span,
        }
    }
}

/// 一つの owner に属する canonical contract suite。
#[derive(Debug, Clone, PartialEq)]
pub struct ContractSuite {
    owner: SymbolId,
    docs: Vec<Example>,
    executable: Vec<ExecutableContract>,
    pending_migration: Vec<LegacyContract>,
    intent_links: Vec<IntentId>,
    claim_links: Vec<ClaimId>,
    source_span: Span,
}

impl ContractSuite {
    pub fn owner(&self) -> &SymbolId {
        &self.owner
    }

    pub fn docs(&self) -> &[Example] {
        &self.docs
    }

    pub fn executable(&self) -> &[ExecutableContract] {
        &self.executable
    }

    pub fn pending_migration(&self) -> &[LegacyContract] {
        &self.pending_migration
    }

    pub fn intent_links(&self) -> &[IntentId] {
        &self.intent_links
    }

    pub fn claim_links(&self) -> &[ClaimId] {
        &self.claim_links
    }

    pub fn source_span(&self) -> Span {
        self.source_span
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContractInventoryError {
    #[error("{owner}: aggregate metadata に contract があるが ordered forms がない")]
    MissingOrderedForms { owner: String },
    #[error("{owner}: ordered forms と aggregate metadata の projection が一致しない")]
    ProjectionMismatch { owner: String },
}

/// parsed program から lossless contract inventory を構築する。
///
/// compatibility aggregate を fallback 正本として利用せず、不一致は fail-closed にする。
pub fn inventory_contract_suites(
    program: &Program,
) -> Result<Vec<ContractSuite>, ContractInventoryError> {
    let mut suites = Vec::new();
    for decl in &program.decls {
        if let Some(suite) = inventory_decl(decl)? {
            suites.push(suite);
        }
    }
    Ok(suites)
}

fn inventory_decl(decl: &Decl) -> Result<Option<ContractSuite>, ContractInventoryError> {
    let decl = match decl {
        Decl::Private { inner, .. } => inner.as_ref(),
        other => other,
    };
    let Decl::Defn {
        name,
        metadata: Some(metadata),
        ..
    } = decl
    else {
        return Ok(None);
    };

    validate_compatibility_projection(name, metadata)?;
    let Some(first) = metadata.forms.first() else {
        return Ok(None);
    };
    let last = metadata
        .forms
        .last()
        .expect("first があれば last も存在する");
    let pending_migration = metadata
        .forms
        .iter()
        .map(|form| match &form.kind {
            MetadataFormKind::LegacyExample { expressions } => LegacyContract::Example {
                expressions: expressions.clone(),
                source_span: form.span(),
            },
            MetadataFormKind::LegacyInvariant { predicate } => LegacyContract::Invariant {
                predicate: predicate.clone(),
                source_span: form.span(),
            },
        })
        .collect();

    Ok(Some(ContractSuite {
        owner: SymbolId(name.clone()),
        docs: Vec::new(),
        executable: Vec::new(),
        pending_migration,
        intent_links: Vec::new(),
        claim_links: Vec::new(),
        source_span: first.span().merge(last.span()),
    }))
}

fn validate_compatibility_projection(
    owner: &str,
    metadata: &Metadata,
) -> Result<(), ContractInventoryError> {
    let aggregate_has_contract = !metadata.example.is_empty() || metadata.invariant.is_some();
    if metadata.forms.is_empty() {
        return if aggregate_has_contract {
            Err(ContractInventoryError::MissingOrderedForms {
                owner: owner.to_string(),
            })
        } else {
            Ok(())
        };
    }

    let mut examples = Vec::new();
    let mut invariant = None;
    for form in &metadata.forms {
        match &form.kind {
            MetadataFormKind::LegacyExample { expressions } => {
                examples.extend(expressions.iter().cloned());
            }
            MetadataFormKind::LegacyInvariant { predicate } => {
                invariant = Some(predicate.clone());
            }
        }
    }

    if examples == metadata.example && invariant == metadata.invariant {
        Ok(())
    } else {
        Err(ContractInventoryError::ProjectionMismatch {
            owner: owner.to_string(),
        })
    }
}
