//! canonical metadata contract の型検査。

use crate::infer::Infer;
use crate::metadata_check::{MetadataDiagnostic, Severity};
use crate::metadata_contract::{ContractSuite, ExecutableContract, ExpectedOutcome};
use crate::types::Type;
use lsharp_syntax::ast::{Decl, Expr, Literal, Program};
use lsharp_syntax::metadata::MetadataFormKind;
use lsharp_syntax::span::Span;

struct AssertionProbe {
    name: String,
    owner: String,
    span: Span,
}

struct CaseProbe {
    actual_name: String,
    expected_name: String,
    owner: String,
    source_span: Span,
    actual_span: Span,
    expected_span: Span,
}

/// 空の canonical `:assert` を、検査 0 件の成功として扱わない。
pub(crate) fn check_assertion_non_vacuity(program: &Program) -> Vec<MetadataDiagnostic> {
    let mut diagnostics = Vec::new();
    for decl in &program.decls {
        collect_assertion_non_vacuity(decl, None, &mut diagnostics);
    }
    diagnostics
}

/// 空の canonical `:case` を、テスト 0 件の成功として扱わない。
pub(crate) fn check_case_non_vacuity(program: &Program) -> Vec<MetadataDiagnostic> {
    let mut diagnostics = Vec::new();
    for decl in &program.decls {
        collect_case_non_vacuity(decl, None, &mut diagnostics);
    }
    diagnostics
}

fn collect_case_non_vacuity(
    decl: &Decl,
    module_prefix: Option<&str>,
    diagnostics: &mut Vec<MetadataDiagnostic>,
) {
    match decl {
        Decl::Private { inner, .. } => {
            collect_case_non_vacuity(inner, module_prefix, diagnostics);
        }
        Decl::ModuleDecl { name, body, .. } => {
            let prefix = match module_prefix {
                Some(outer) => format!("{outer}.{name}"),
                None => name.clone(),
            };
            for nested in body {
                collect_case_non_vacuity(nested, Some(&prefix), diagnostics);
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
                if let MetadataFormKind::Case { expectations } = &form.kind
                    && expectations.is_empty()
                {
                    diagnostics.push(MetadataDiagnostic {
                        severity: Severity::Error,
                        message: ":case は少なくとも 1 件の expectation を必要とします".to_string(),
                        span: form.span(),
                        function_name: owner.clone(),
                    });
                }
            }
        }
        _ => {}
    }
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
                            if matches!(predicate, Expr::Lit(_, Literal::Bool(true)))
                                || statically_true_integer_comparison(predicate)
                            {
                                diagnostics.push(MetadataDiagnostic {
                                    severity: Severity::Error,
                                    message: ":assert predicate は静的に true で検査を識別できず vacuous です"
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

fn statically_true_integer_comparison(expr: &Expr) -> bool {
    let Expr::Ann(_, inner, _) = expr else {
        return statically_true_integer_comparison_app(expr);
    };
    statically_true_integer_comparison(inner)
}

fn statically_true_integer_comparison_app(expr: &Expr) -> bool {
    let Expr::App(_, callee, args) = expr else {
        return false;
    };
    let Expr::Var(_, operator) = callee.as_ref() else {
        return false;
    };
    let [Expr::Lit(_, Literal::Int(left)), Expr::Lit(_, Literal::Int(right))] = args.as_slice()
    else {
        return false;
    };

    match operator.as_str() {
        "=" | "==" => left == right,
        "!=" => left != right,
        "<" => left < right,
        ">" => left > right,
        "<=" => left <= right,
        ">=" => left >= right,
        _ => false,
    }
}

/// canonical `:case` の actual / expected を同じ lexical scope で型検査する。
pub(crate) fn check_case_types(
    program: &Program,
    suites: &[ContractSuite],
) -> Vec<MetadataDiagnostic> {
    let mut check_program = program.clone();
    let mut probes = Vec::new();

    for suite in suites {
        for contract in suite.executable() {
            let ExecutableContract::Case(case) = contract else {
                continue;
            };
            let ExpectedOutcome::Value(expected) = case.expected() else {
                continue;
            };
            let index = probes.len();
            let actual_name = format!("lsharp.internal.case.actual#{index}");
            let expected_name = format!("lsharp.internal.case.expected#{index}");
            append_case_probe(
                &mut check_program,
                &actual_name,
                case.actual().clone(),
                case.actual().span(),
            );
            append_case_probe(
                &mut check_program,
                &expected_name,
                expected.clone(),
                expected.span(),
            );
            probes.push(CaseProbe {
                actual_name,
                expected_name,
                owner: suite.owner().as_str().to_string(),
                source_span: case.source_span(),
                actual_span: case.actual().span(),
                expected_span: expected.span(),
            });
        }
    }

    if probes.is_empty() {
        return Vec::new();
    }

    let mut infer = Infer::new();
    match infer.infer_program(&check_program) {
        Ok(results) => case_type_diagnostics(&results, &probes),
        Err(error) => case_inference_error_diagnostic(error, &probes)
            .into_iter()
            .collect(),
    }
}

fn append_case_probe(program: &mut Program, name: &str, body: Expr, span: Span) {
    program.decls.push(Decl::Defn {
        span,
        name: name.to_string(),
        params: Vec::new(),
        return_ty: None,
        body,
        where_clauses: Vec::new(),
        metadata: None,
    });
}

fn case_type_diagnostics(
    results: &[(String, crate::types::TypeScheme)],
    probes: &[CaseProbe],
) -> Vec<MetadataDiagnostic> {
    let mut diagnostics = Vec::new();
    for probe in probes {
        let (Some(actual_type), Some(expected_type)) = (
            probe_return_type(results, &probe.actual_name),
            probe_return_type(results, &probe.expected_name),
        ) else {
            diagnostics.push(MetadataDiagnostic {
                severity: Severity::Error,
                message: ":case の型検査結果が不足しています".to_string(),
                span: probe.source_span,
                function_name: probe.owner.clone(),
            });
            continue;
        };

        if actual_type != expected_type {
            diagnostics.push(MetadataDiagnostic {
                severity: Severity::Error,
                message: format!(
                    ":case actual / expected の型検査に失敗しました: actual={actual_type}, expected={expected_type}"
                ),
                span: probe.expected_span,
                function_name: probe.owner.clone(),
            });
            continue;
        }

        if actual_type != &Type::int() && actual_type != &Type::bool() {
            diagnostics.push(MetadataDiagnostic {
                severity: Severity::Error,
                message: format!(
                    ":case comparison は現在 Int / Bool 必須ですが、{actual_type} が推論されました"
                ),
                span: probe.actual_span,
                function_name: probe.owner.clone(),
            });
        }
    }
    diagnostics
}

fn probe_return_type<'a>(
    results: &'a [(String, crate::types::TypeScheme)],
    name: &str,
) -> Option<&'a Type> {
    let (_, scheme) = results
        .iter()
        .find(|(result_name, _)| result_name == name)?;
    let Type::Fun(params, return_type) = &scheme.ty else {
        return None;
    };
    params.is_empty().then_some(return_type.as_ref())
}

fn case_inference_error_diagnostic(
    error: crate::infer::TypeError,
    probes: &[CaseProbe],
) -> Option<MetadataDiagnostic> {
    let error_span = error.span()?;
    let probe = probes
        .iter()
        .find(|probe| contains(probe.source_span, error_span))?;
    Some(MetadataDiagnostic {
        severity: Severity::Error,
        message: format!(":case の型推論に失敗しました: {error}"),
        span: error_span,
        function_name: probe.owner.clone(),
    })
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
