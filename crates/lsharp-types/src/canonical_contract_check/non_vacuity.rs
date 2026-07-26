//! canonical metadata contract の非空性検査。

use crate::metadata_check::{MetadataDiagnostic, Severity};
use lsharp_syntax::ast::{Decl, Expr, Literal, Program};
use lsharp_syntax::metadata::MetadataFormKind;
use std::collections::HashSet;

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

/// 空の canonical `:property` や literal-true property を成功扱いしない。
pub(crate) fn check_property_non_vacuity(program: &Program) -> Vec<MetadataDiagnostic> {
    let mut diagnostics = Vec::new();
    for decl in &program.decls {
        collect_property_non_vacuity(decl, None, &mut diagnostics);
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

fn collect_property_non_vacuity(
    decl: &Decl,
    module_prefix: Option<&str>,
    diagnostics: &mut Vec<MetadataDiagnostic>,
) {
    match decl {
        Decl::Private { inner, .. } => {
            collect_property_non_vacuity(inner, module_prefix, diagnostics);
        }
        Decl::ModuleDecl { name, body, .. } => {
            let prefix = match module_prefix {
                Some(outer) => format!("{outer}.{name}"),
                None => name.clone(),
            };
            for nested in body {
                collect_property_non_vacuity(nested, Some(&prefix), diagnostics);
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
                let MetadataFormKind::Property { properties } = &form.kind else {
                    continue;
                };
                if properties.is_empty() {
                    diagnostics.push(MetadataDiagnostic {
                        severity: Severity::Error,
                        message: ":property は少なくとも 1 件の for-all を必要とします".to_string(),
                        span: form.span(),
                        function_name: owner.clone(),
                    });
                }
                for property in properties {
                    let mut binder_names = HashSet::new();
                    for binder in property.binders() {
                        if binder.name() == "result" {
                            diagnostics.push(MetadataDiagnostic {
                                severity: Severity::Error,
                                message: ":property binder の result は予約名のため使用できません"
                                    .to_string(),
                                span: binder.source_span(),
                                function_name: owner.clone(),
                            });
                        } else if !binder_names.insert(binder.name()) {
                            diagnostics.push(MetadataDiagnostic {
                                severity: Severity::Error,
                                message: format!(
                                    ":property binder 名 '{}' は重複しています",
                                    binder.name()
                                ),
                                span: binder.source_span(),
                                function_name: owner.clone(),
                            });
                        }
                    }
                    if property.binders().is_empty() {
                        diagnostics.push(MetadataDiagnostic {
                            severity: Severity::Error,
                            message: ":property は少なくとも 1 件の typed binder を必要とします"
                                .to_string(),
                            span: property.source_span(),
                            function_name: owner.clone(),
                        });
                    }
                    if property.cases() == Some(0) {
                        diagnostics.push(MetadataDiagnostic {
                            severity: Severity::Error,
                            message: ":property の case count は 1 以上を必要とします".to_string(),
                            span: property.source_span(),
                            function_name: owner.clone(),
                        });
                    }
                    for precondition in property.preconditions() {
                        if statically_false_precondition(precondition) {
                            diagnostics.push(MetadataDiagnostic {
                                severity: Severity::Error,
                                message: ":property の precondition は到達不能で vacuous です"
                                    .to_string(),
                                span: precondition.span(),
                                function_name: owner.clone(),
                            });
                        }
                    }
                    if matches!(property.postcondition(), Expr::Lit(_, Literal::Bool(true)))
                        || statically_true_integer_comparison(property.postcondition())
                    {
                        diagnostics.push(MetadataDiagnostic {
                            severity: Severity::Error,
                            message: ":property の postcondition は検査を識別できず vacuous です"
                                .to_string(),
                            span: property.postcondition().span(),
                            function_name: owner.clone(),
                        });
                    }
                }
            }
        }
        _ => {}
    }
}

fn statically_true_integer_comparison(expr: &Expr) -> bool {
    matches!(static_boolean_result(expr), Some(true))
}

fn statically_false_precondition(expr: &Expr) -> bool {
    matches!(static_boolean_result(expr), Some(false))
}

fn static_integer_comparison_result(expr: &Expr) -> Option<bool> {
    let Expr::Ann(_, inner, _) = expr else {
        return static_integer_comparison_result_app(expr);
    };
    static_integer_comparison_result(inner)
}

fn static_integer_comparison_result_app(expr: &Expr) -> Option<bool> {
    let Expr::App(_, callee, args) = expr else {
        return None;
    };
    let Expr::Var(_, operator) = callee.as_ref() else {
        return None;
    };
    let [
        Expr::Lit(_, Literal::Int(left)),
        Expr::Lit(_, Literal::Int(right)),
    ] = args.as_slice()
    else {
        return None;
    };

    Some(match operator.as_str() {
        "=" | "==" => left == right,
        "!=" => left != right,
        "<" => left < right,
        ">" => left > right,
        "<=" => left <= right,
        ">=" => left >= right,
        _ => return None,
    })
}

fn expression_shape_equal(left: &Expr, right: &Expr) -> bool {
    match (left, right) {
        (Expr::Ann(_, left, _), _) => expression_shape_equal(left, right),
        (_, Expr::Ann(_, right, _)) => expression_shape_equal(left, right),
        (Expr::Lit(_, left), Expr::Lit(_, right)) => left == right,
        (Expr::Var(_, left), Expr::Var(_, right)) => left == right,
        (Expr::App(_, left_callee, left_args), Expr::App(_, right_callee, right_args)) => {
            left_args.len() == right_args.len()
                && expression_shape_equal(left_callee, right_callee)
                && left_args
                    .iter()
                    .zip(right_args)
                    .all(|(left, right)| expression_shape_equal(left, right))
        }
        _ => false,
    }
}

fn is_boolean_negation_pair(left: &Expr, right: &Expr) -> bool {
    fn is_not_of(candidate: &Expr, operand: &Expr) -> bool {
        let Expr::App(_, callee, args) = candidate else {
            return false;
        };
        let Expr::Var(_, operator) = callee.as_ref() else {
            return false;
        };
        args.len() == 1 && operator == "not" && expression_shape_equal(&args[0], operand)
    }

    is_not_of(left, right) || is_not_of(right, left)
}

fn static_boolean_result(expr: &Expr) -> Option<bool> {
    if let Expr::Ann(_, inner, _) = expr {
        return static_boolean_result(inner);
    }
    if let Expr::Lit(_, Literal::Bool(value)) = expr {
        return Some(*value);
    }
    let Expr::App(_, callee, args) = expr else {
        return static_integer_comparison_result(expr);
    };
    let Expr::Var(_, operator) = callee.as_ref() else {
        return static_integer_comparison_result(expr);
    };
    if operator == "not" {
        let [operand] = args.as_slice() else {
            return static_integer_comparison_result(expr);
        };
        return static_boolean_result(operand).map(|value| !value);
    }
    let [left, right] = args.as_slice() else {
        return static_integer_comparison_result(expr);
    };
    let left = static_boolean_result(left);
    let right = static_boolean_result(right);
    match operator.as_str() {
        "and" => {
            if is_boolean_negation_pair(&args[0], &args[1]) {
                Some(false)
            } else {
                match (left, right) {
                    (Some(false), _) | (_, Some(false)) => Some(false),
                    (Some(true), other) | (other, Some(true)) => other,
                    (None, None) => None,
                }
            }
        }
        "or" => {
            if is_boolean_negation_pair(&args[0], &args[1]) {
                Some(true)
            } else {
                match (left, right) {
                    (Some(true), _) | (_, Some(true)) => Some(true),
                    (Some(false), other) | (other, Some(false)) => other,
                    (None, None) => None,
                }
            }
        }
        _ => static_integer_comparison_result_app(expr),
    }
}
