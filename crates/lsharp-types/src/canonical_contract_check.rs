//! canonical metadata contract の型検査。

use crate::infer::Infer;
use crate::metadata_check::{MetadataDiagnostic, Severity};
use crate::metadata_contract::{ContractSuite, ExecutableContract};
use crate::types::Type;
use lsharp_syntax::ast::{Decl, Expr, Literal, Program};
use lsharp_syntax::metadata::MetadataFormKind;
use lsharp_syntax::span::Span;

struct AssertionProbe {
    name: String,
    owner: String,
    span: Span,
}

/// 空の canonical `:assert` を、検査 0 件の成功として扱わない。
pub(crate) fn check_assertion_non_vacuity(program: &Program) -> Vec<MetadataDiagnostic> {
    let mut diagnostics = Vec::new();
    for decl in &program.decls {
        collect_assertion_non_vacuity(decl, None, &mut diagnostics);
    }
    diagnostics
}

fn collect_assertion_non_vacuity(
    decl: &Decl,
    module_prefix: Option<&str>,
    diagnostics: &mut Vec<MetadataDiagnostic>,
) {
    match decl {
        Decl::Private { inner, .. } => {
            collect_assertion_non_vacuity(inner, module_prefix, diagnostics);
        }
        Decl::ModuleDecl { name, body, .. } => {
            let prefix = match module_prefix {
                Some(outer) => format!("{outer}.{name}"),
                None => name.clone(),
            };
            for nested in body {
                collect_assertion_non_vacuity(nested, Some(&prefix), diagnostics);
            }
        }
        Decl::Defn {
            name,
            metadata: Some(metadata),
            ..
        } => {
            let owner = match module_prefix {
                Some(prefix) => format!("{prefix}.{name}"),
                None => name.clone(),
            };
            for form in &metadata.forms {
                match &form.kind {
                    MetadataFormKind::Assertion { predicates } if predicates.is_empty() => {
                        diagnostics.push(MetadataDiagnostic {
                            severity: Severity::Error,
                            message: ":assert は少なくとも 1 件の predicate を必要とします"
                                .to_string(),
                            span: form.span(),
                            function_name: owner.clone(),
                        });
                    }
                    MetadataFormKind::Assertion { predicates } => {
                        for predicate in predicates {
                            if matches!(predicate, Expr::Lit(_, Literal::Bool(true))) {
                                diagnostics.push(MetadataDiagnostic {
                                    severity: Severity::Error,
                                    message: ":assert の literal true predicate は検査を識別できず vacuous です"
                                        .to_string(),
                                    span: predicate.span(),
                                    function_name: owner.clone(),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// canonical assertion を既存の HM 推論器で検査する。
///
/// assertion は関数引数や `result` を暗黙に束縛しないため、引数なしの内部関数として
/// program の lexical scope に追加する。元の AST は変更しない。
pub(crate) fn check_assertion_types(
    program: &Program,
    suites: &[ContractSuite],
) -> Vec<MetadataDiagnostic> {
    let mut check_program = program.clone();
    let mut probes = Vec::new();

    for suite in suites {
        for contract in suite.executable() {
            let ExecutableContract::Assertion(assertion) = contract else {
                continue;
            };
            let name = format!("lsharp.internal.assert#{}", probes.len());
            let span = assertion.source_span();
            check_program.decls.push(Decl::Defn {
                span,
                name: name.clone(),
                params: Vec::new(),
                return_ty: None,
                body: assertion.predicate().clone(),
                where_clauses: Vec::new(),
                metadata: None,
            });
            probes.push(AssertionProbe {
                name,
                owner: suite.owner().as_str().to_string(),
                span,
            });
        }
    }

    if probes.is_empty() {
        return Vec::new();
    }

    let mut infer = Infer::new();
    match infer.infer_program(&check_program) {
        Ok(results) => non_bool_assertion_diagnostics(&results, &probes),
        Err(error) => inference_error_diagnostic(error, &probes)
            .into_iter()
            .collect(),
    }
}

fn non_bool_assertion_diagnostics(
    results: &[(String, crate::types::TypeScheme)],
    probes: &[AssertionProbe],
) -> Vec<MetadataDiagnostic> {
    let bool_type = Type::bool();
    probes
        .iter()
        .filter_map(|probe| {
            let (_, scheme) = results.iter().find(|(name, _)| name == &probe.name)?;
            let Type::Fun(params, return_type) = &scheme.ty else {
                return None;
            };
            if !params.is_empty() || return_type.as_ref() == &bool_type {
                return None;
            }
            Some(MetadataDiagnostic {
                severity: Severity::Error,
                message: format!(
                    ":assert predicate は Bool 必須ですが、{} が推論されました",
                    return_type
                ),
                span: probe.span,
                function_name: probe.owner.clone(),
            })
        })
        .collect()
}

fn inference_error_diagnostic(
    error: crate::infer::TypeError,
    probes: &[AssertionProbe],
) -> Option<MetadataDiagnostic> {
    let error_span = error.span()?;
    let probe = probes
        .iter()
        .find(|probe| contains(probe.span, error_span))?;
    Some(MetadataDiagnostic {
        severity: Severity::Error,
        message: format!(":assert predicate の型推論に失敗しました: {error}"),
        span: error_span,
        function_name: probe.owner.clone(),
    })
}

fn contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}
