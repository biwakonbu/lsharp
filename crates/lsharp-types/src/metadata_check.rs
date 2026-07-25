//! メタデータ機械的検証エンジン
//!
//! 構造化メタデータ（:doc, :params, :returns 等）の整合性を検証する。
//! エラー（不整合）と警告（推奨）を区別して報告する。

use crate::canonical_contract_check::{
    check_assertion_non_vacuity, check_assertion_types, check_case_non_vacuity, check_case_types,
    check_property_non_vacuity, check_property_types,
};
use crate::metadata_contract::inventory_contract_suites;
use lsharp_syntax::ast::{Decl, Program};
use lsharp_syntax::span::Span;

mod diagnostics;
mod legacy;
mod references;
mod test_generation;

pub use test_generation::{
    GeneratedTest, PropertyBinderType, PropertySmokeTestSpec, TestKind, generate_tests,
    property_smoke_test_spec,
};

use diagnostics::check_defn_metadata;
use legacy::check_legacy_invariant_types;

/// メタデータ検証結果の重大度
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    /// 不整合（修正必須）
    Error,
    /// 推奨事項（修正任意）
    Warning,
}

/// メタデータ検証の診断メッセージ
#[derive(Debug, Clone)]
pub struct MetadataDiagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    pub function_name: String,
}

impl std::fmt::Display for MetadataDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        write!(
            f,
            "[{level}] {}: {} ({})",
            self.function_name, self.message, self.span
        )
    }
}

/// プログラム全体のメタデータを検証
pub fn check_metadata(program: &Program) -> Vec<MetadataDiagnostic> {
    let mut diagnostics = Vec::new();

    // 全関数名を収集（:see-also の参照先チェック用）
    let all_names: Vec<String> = program
        .decls
        .iter()
        .filter_map(|d| match d {
            Decl::Defn { name, .. } => Some(name.clone()),
            Decl::TypeDef { name, .. } => Some(name.clone()),
            Decl::RecordDef { name, .. } => Some(name.clone()),
            Decl::TypeAlias { name, .. } => Some(name.clone()),
            Decl::TraitDef { name, .. } => Some(name.clone()),
            Decl::Private { inner, .. } => match inner.as_ref() {
                Decl::Defn { name, .. }
                | Decl::TypeDef { name, .. }
                | Decl::RecordDef { name, .. }
                | Decl::TypeAlias { name, .. }
                | Decl::TraitDef { name, .. } => Some(name.clone()),
                _ => None,
            },
            _ => None,
        })
        .collect();

    for decl in &program.decls {
        // Private 内の defn も展開して検証
        let actual_decl = match decl {
            Decl::Private { inner, .. } => inner.as_ref(),
            other => other,
        };

        if let Decl::Defn {
            name,
            params,
            metadata,
            span,
            ..
        } = actual_decl
            && let Some(meta) = metadata
        {
            check_defn_metadata(&mut diagnostics, name, params, meta, *span, &all_names);
        }
    }

    diagnostics.extend(check_legacy_invariant_types(program, &all_names));
    diagnostics.extend(check_assertion_non_vacuity(program));
    diagnostics.extend(check_case_non_vacuity(program));
    diagnostics.extend(check_property_non_vacuity(program));
    if let Ok(suites) = inventory_contract_suites(program) {
        diagnostics.extend(check_assertion_types(program, &suites));
        diagnostics.extend(check_case_types(program, &suites));
        diagnostics.extend(check_property_types(program, &suites));
    }

    diagnostics
}

#[cfg(test)]
mod diagnostics_tests;
#[cfg(test)]
mod legacy_tests;
#[cfg(test)]
mod test_generation_tests;
#[cfg(test)]
mod tests;
