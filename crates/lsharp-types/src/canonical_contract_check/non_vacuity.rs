//! canonical metadata contract の非空性検査。

use crate::metadata_check::{MetadataDiagnostic, Severity};
use lsharp_syntax::ast::{Decl, Expr, Literal, Pattern, Program};
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

/// `static_boolean_result` が意味を持つ演算子名。
///
/// `let` / `match` がこれらを再束縛していると、静的評価は影に隠れた定義ではなく
/// builtin の意味で計算してしまう。判定を諦める (None) 側へ倒すための集合である。
const STATIC_OPERATOR_NAMES: [&str; 10] =
    ["not", "and", "or", "=", "==", "!=", "<", ">", "<=", ">="];

/// 静的評価中に見えている束縛。値が静的に決まらない束縛は `None` で積み、
/// **外側の同名束縛を覆う**。後ろから引くことで最新の束縛が勝つ。
type StaticEnv<'a> = Vec<(&'a str, Option<bool>)>;

fn lookup_static_binding(env: &StaticEnv<'_>, name: &str) -> Option<bool> {
    env.iter()
        .rev()
        .find(|(bound, _)| *bound == name)
        .and_then(|(_, value)| *value)
}

fn pattern_shadows_static_operator(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Wildcard(_) | Pattern::Lit(_, _) => false,
        Pattern::Var(_, name) => STATIC_OPERATOR_NAMES.contains(&name.as_str()),
        Pattern::Constructor(_, _, inner) => inner.iter().any(pattern_shadows_static_operator),
        Pattern::RecordPat(_, _, fields) => fields
            .iter()
            .any(|(_, inner)| pattern_shadows_static_operator(inner)),
    }
}

/// pattern が束縛する名前を「値不明」として env へ積む。
///
/// 積まないと、外側の静的な同名束縛が arm body へ漏れて誤判定になる。
fn shadow_pattern_bindings<'a>(pattern: &'a Pattern, env: &mut StaticEnv<'a>) {
    match pattern {
        Pattern::Wildcard(_) | Pattern::Lit(_, _) => {}
        Pattern::Var(_, name) => env.push((name.as_str(), None)),
        Pattern::Constructor(_, _, inner) => {
            for nested in inner {
                shadow_pattern_bindings(nested, env);
            }
        }
        Pattern::RecordPat(_, _, fields) => {
            for (_, nested) in fields {
                shadow_pattern_bindings(nested, env);
            }
        }
    }
}

/// 分岐形の静的評価。全ての候補が同じ値へ落ちるときだけ確定させる。
///
/// `if` は条件が静的に決まればその枝だけを見る。決まらない場合と `match` は
/// **全候補の一致**を要求する。片方しか見ないと、到達しうる別の枝を無視して
/// vacuous と誤判定する。
fn static_boolean_result_of_branches<'a>(
    branches: impl IntoIterator<Item = &'a Expr>,
    env: &StaticEnv<'a>,
) -> Option<bool> {
    let mut settled: Option<bool> = None;
    for branch in branches {
        let value = static_boolean_result_in(branch, env)?;
        match settled {
            None => settled = Some(value),
            Some(previous) if previous == value => {}
            Some(_) => return None,
        }
    }
    settled
}

fn static_boolean_result(expr: &Expr) -> Option<bool> {
    static_boolean_result_in(expr, &StaticEnv::new())
}

fn static_boolean_result_in<'a>(expr: &'a Expr, env: &StaticEnv<'a>) -> Option<bool> {
    if let Expr::Ann(_, inner, _) = expr {
        return static_boolean_result_in(inner, env);
    }
    if let Expr::Lit(_, Literal::Bool(value)) = expr {
        return Some(*value);
    }
    if let Expr::Var(_, name) = expr {
        return lookup_static_binding(env, name);
    }
    if let Expr::If(_, condition, then_branch, else_branch) = expr {
        return match static_boolean_result_in(condition, env) {
            Some(true) => static_boolean_result_in(then_branch, env),
            Some(false) => static_boolean_result_in(else_branch, env),
            None => {
                static_boolean_result_of_branches([then_branch.as_ref(), else_branch.as_ref()], env)
            }
        };
    }
    if let Expr::Let(_, bindings, body) = expr {
        if bindings
            .iter()
            .any(|(pattern, _)| pattern_shadows_static_operator(pattern))
        {
            return None;
        }
        let mut scoped = env.clone();
        for (pattern, value) in bindings {
            match pattern {
                Pattern::Var(_, name) => {
                    let bound = static_boolean_result_in(value, &scoped);
                    scoped.push((name.as_str(), bound));
                }
                other => shadow_pattern_bindings(other, &mut scoped),
            }
        }
        return static_boolean_result_in(body, &scoped);
    }
    if let Expr::Do(_, exprs) = expr {
        return static_boolean_result_in(exprs.last()?, env);
    }
    if let Expr::Match(_, _, arms) = expr {
        if arms.is_empty()
            || arms
                .iter()
                .any(|arm| pattern_shadows_static_operator(&arm.pattern))
        {
            return None;
        }
        let mut settled: Option<bool> = None;
        for arm in arms {
            let mut scoped = env.clone();
            shadow_pattern_bindings(&arm.pattern, &mut scoped);
            let value = static_boolean_result_in(&arm.body, &scoped)?;
            match settled {
                None => settled = Some(value),
                Some(previous) if previous == value => {}
                Some(_) => return None,
            }
        }
        return settled;
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
        return static_boolean_result_in(operand, env).map(|value| !value);
    }
    let [left, right] = args.as_slice() else {
        return static_integer_comparison_result(expr);
    };
    let left = static_boolean_result_in(left, env);
    let right = static_boolean_result_in(right, env);
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
