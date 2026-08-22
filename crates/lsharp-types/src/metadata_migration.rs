//! v0.1 metadata contract から v0.2 canonical form への preview classifier。
//!
//! classifier は source を書き換えず、現在選択される legacy semantics と移行候補を
//! code / owner / span 付きで返す。型や lexical scope を確定できない form は
//! silent conversion せず manual review に残す。

use crate::infer::Infer;
use crate::metadata_contract::{ContractInventoryError, LegacyContract, inventory_contract_suites};
use crate::types::Type;
use lsharp_syntax::ast::{Decl, Expr, Program};
use lsharp_syntax::span::Span;

/// v0.2 preview で実際に選択される互換 semantics。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacySelectedSemantics {
    /// v0.1 `:example` の 0 / 非0 判定。
    ExampleTruthiness,
    /// v0.1 `:invariant` の固定 sample plan。
    InvariantDeterministicSmoke,
}

impl LegacySelectedSemantics {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExampleTruthiness => "legacy-example-truthiness",
            Self::InvariantDeterministicSmoke => "legacy-invariant-deterministic-smoke",
        }
    }
}

/// classifier が提案する canonical disposition。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyMigrationDisposition {
    /// non-Bool expression を docs-only `:example` として保持する候補。
    DocumentationExample,
    /// Bool expression を strict `:assert` へ移す候補。
    Assertion,
    /// legacy invariant を sampled property / postcondition へ移す候補。
    PropertyPostcondition,
    /// lexical scope または型を確定できず、自動変換しない。
    ManualReview,
}

impl LegacyMigrationDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DocumentationExample => "docs-only-example",
            Self::Assertion => "assertion",
            Self::PropertyPostcondition => "property-postcondition",
            Self::ManualReview => "manual-review",
        }
    }
}

/// 一つの legacy contract expression に対する migration report row。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyMigrationDiagnostic {
    code: &'static str,
    owner: String,
    selected_semantics: LegacySelectedSemantics,
    disposition: LegacyMigrationDisposition,
    span: Span,
    message: String,
}

impl LegacyMigrationDiagnostic {
    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn selected_semantics(&self) -> LegacySelectedSemantics {
        self.selected_semantics
    }

    pub fn disposition(&self) -> LegacyMigrationDisposition {
        self.disposition
    }

    pub fn span(&self) -> Span {
        self.span
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for LegacyMigrationDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "[{}] {}: selected={}, disposition={}; {} ({})",
            self.code,
            self.owner,
            self.selected_semantics.as_str(),
            self.disposition.as_str(),
            self.message,
            self.span
        )
    }
}

/// legacy contract を source order の migration report へ分類する。
pub fn classify_legacy_contracts(
    program: &Program,
) -> Result<Vec<LegacyMigrationDiagnostic>, ContractInventoryError> {
    let suites = inventory_contract_suites(program)?;
    let mut diagnostics = Vec::new();
    let mut probe_index = 0;

    for suite in suites {
        let owner = suite.owner().as_str();
        for contract in suite.pending_migration() {
            match contract {
                LegacyContract::Example { expressions, .. } => {
                    for expression in expressions {
                        diagnostics.push(classify_example(program, owner, expression, probe_index));
                        probe_index += 1;
                    }
                }
                LegacyContract::Invariant { predicate, .. } => {
                    diagnostics.push(LegacyMigrationDiagnostic {
                        code: "LS2002",
                        owner: owner.to_string(),
                        selected_semantics: LegacySelectedSemantics::InvariantDeterministicSmoke,
                        disposition: LegacyMigrationDisposition::PropertyPostcondition,
                        span: predicate.span(),
                        message: "legacy :invariant は :property / :postcondition への移行候補です"
                            .to_string(),
                    });
                }
            }
        }
    }

    Ok(diagnostics)
}

fn classify_example(
    program: &Program,
    owner: &str,
    expression: &Expr,
    probe_index: usize,
) -> LegacyMigrationDiagnostic {
    let span = expression.span();
    match infer_lexical_expression_type(program, expression, probe_index) {
        Ok(ty) if ty == Type::bool() => LegacyMigrationDiagnostic {
            code: "LS2001",
            owner: owner.to_string(),
            selected_semantics: LegacySelectedSemantics::ExampleTruthiness,
            disposition: LegacyMigrationDisposition::Assertion,
            span,
            message: "Bool legacy :example は strict :assert への移行候補です".to_string(),
        },
        Ok(ty) if contains_type_variable(&ty) => ambiguous_example(
            owner,
            span,
            &format!("型 {ty} を concrete に確定できません"),
        ),
        Ok(ty) => LegacyMigrationDiagnostic {
            code: "LS2001",
            owner: owner.to_string(),
            selected_semantics: LegacySelectedSemantics::ExampleTruthiness,
            disposition: LegacyMigrationDisposition::DocumentationExample,
            span,
            message: format!(
                "non-Bool ({ty}) legacy :example は docs-only :example として保持する候補です"
            ),
        },
        Err(reason) => ambiguous_example(owner, span, &reason),
    }
}

fn ambiguous_example(owner: &str, span: Span, reason: &str) -> LegacyMigrationDiagnostic {
    LegacyMigrationDiagnostic {
        code: "LS2003",
        owner: owner.to_string(),
        selected_semantics: LegacySelectedSemantics::ExampleTruthiness,
        disposition: LegacyMigrationDisposition::ManualReview,
        span,
        message: format!(
            "legacy :example は silent conversion できません。manual review が必要です: {reason}"
        ),
    }
}

fn infer_lexical_expression_type(
    program: &Program,
    expression: &Expr,
    probe_index: usize,
) -> Result<Type, String> {
    let mut probe_program = program.clone();
    let probe_name = format!("lsharp.internal.migration#{probe_index}");
    probe_program.decls.push(Decl::Defn {
        span: expression.span(),
        name: probe_name.clone(),
        params: Vec::new(),
        return_ty: None,
        body: expression.clone(),
        where_clauses: Vec::new(),
        metadata: None,
    });

    let mut infer = Infer::new();
    let results = infer
        .infer_program(&probe_program)
        .map_err(|error| error.to_string())?;
    let (_, scheme) = results
        .iter()
        .find(|(name, _)| name == &probe_name)
        .ok_or_else(|| "migration probe の推論結果がありません".to_string())?;
    let Type::Fun(params, return_type) = &scheme.ty else {
        return Err("migration probe が関数型ではありません".to_string());
    };
    if !params.is_empty() {
        return Err("migration probe に予期しない引数があります".to_string());
    }
    Ok(return_type.as_ref().clone())
}

fn contains_type_variable(ty: &Type) -> bool {
    match ty {
        Type::Var(_) => true,
        Type::Con(_) => false,
        Type::Fun(params, return_type) => {
            params.iter().any(contains_type_variable) || contains_type_variable(return_type)
        }
        Type::App(_, args) => args.iter().any(contains_type_variable),
        Type::Record(_, fields) => fields
            .iter()
            .any(|(_, field_type)| contains_type_variable(field_type)),
    }
}
