//! 自由変数解析
//!
//! Lambda 式の本体を走査し、バインドされていない変数参照を収集する。

use std::collections::HashSet;

use lsharp_syntax::ast::*;

/// Lambda 式から自由変数を収集する
pub fn free_variables(params: &[String], body: &Expr) -> HashSet<String> {
    let mut bound: HashSet<String> = params.iter().cloned().collect();
    let mut free = HashSet::new();
    collect_free_vars(body, &mut bound, &mut free);
    free
}

/// 式を再帰的に走査して自由変数を収集
fn collect_free_vars(expr: &Expr, bound: &mut HashSet<String>, free: &mut HashSet<String>) {
    match expr {
        Expr::Var(_, name) => {
            if !bound.contains(name) {
                free.insert(name.clone());
            }
        }
        Expr::Lit(_, _) => {}
        Expr::App(_, func, args) => {
            collect_free_vars(func, bound, free);
            for arg in args {
                collect_free_vars(arg, bound, free);
            }
        }
        Expr::Lambda(_, params, body) => {
            let mut inner_bound = bound.clone();
            for p in params {
                inner_bound.insert(p.name.clone());
            }
            collect_free_vars(body, &mut inner_bound, free);
        }
        Expr::Let(_, bindings, body) => {
            let mut inner_bound = bound.clone();
            for (pat, value) in bindings {
                collect_free_vars(value, &mut inner_bound, free);
                collect_pattern_bindings(pat, &mut inner_bound);
            }
            collect_free_vars(body, &mut inner_bound, free);
        }
        Expr::If(_, condition, then_branch, else_branch) => {
            collect_free_vars(condition, bound, free);
            collect_free_vars(then_branch, bound, free);
            collect_free_vars(else_branch, bound, free);
        }
        Expr::Do(_, expressions) => {
            for e in expressions {
                collect_free_vars(e, bound, free);
            }
        }
        Expr::Match(_, scrutinee, arms) => {
            collect_free_vars(scrutinee, bound, free);
            for arm in arms {
                let mut arm_bound = bound.clone();
                collect_pattern_bindings(&arm.pattern, &mut arm_bound);
                // ガード条件の自由変数も収集
                if let Some(guard) = &arm.guard {
                    collect_free_vars(guard, &mut arm_bound, free);
                }
                collect_free_vars(&arm.body, &mut arm_bound, free);
            }
        }
        Expr::Ann(_, inner, _) => {
            collect_free_vars(inner, bound, free);
        }
        Expr::RecordLit(_, _, fields) => {
            for (_, val) in fields {
                collect_free_vars(val, bound, free);
            }
        }
        Expr::FieldAccess(_, inner, _) => {
            collect_free_vars(inner, bound, free);
        }
        Expr::RecordUpdate(_, base, fields) => {
            collect_free_vars(base, bound, free);
            for (_, val) in fields {
                collect_free_vars(val, bound, free);
            }
        }
        Expr::Computation(_, _, steps) => {
            for step in steps {
                match step {
                    ComputationStep::LetBang(_, pat, expr) => {
                        collect_free_vars(expr, bound, free);
                        let mut inner_bound = bound.clone();
                        collect_pattern_bindings(pat, &mut inner_bound);
                        // 後続ステップは inner_bound を使うべきだが、
                        // 各ステップは独立して走査されるため bound を更新
                        for name in inner_bound.difference(bound).cloned().collect::<Vec<_>>() {
                            bound.insert(name);
                        }
                    }
                    ComputationStep::DoBang(_, expr) => {
                        collect_free_vars(expr, bound, free);
                    }
                    ComputationStep::Return(_, expr) => {
                        collect_free_vars(expr, bound, free);
                    }
                    ComputationStep::Expr(expr) => {
                        collect_free_vars(expr, bound, free);
                    }
                }
            }
        }
        // P10-1: Quote/Unquote/UnquoteSplice -- 内部式の自由変数を収集
        Expr::Quote(_, inner) | Expr::Unquote(_, inner) | Expr::UnquoteSplice(_, inner) => {
            collect_free_vars(inner, bound, free);
        }
    }
}

/// パターンからバインドされる変数を収集
fn collect_pattern_bindings(pattern: &Pattern, bound: &mut HashSet<String>) {
    match pattern {
        Pattern::Var(_, name) => {
            bound.insert(name.clone());
        }
        Pattern::Constructor(_, _, fields) => {
            for field in fields {
                collect_pattern_bindings(field, bound);
            }
        }
        Pattern::RecordPat(_, _, fields) => {
            for (_, pat) in fields {
                collect_pattern_bindings(pat, bound);
            }
        }
        Pattern::Wildcard(_) | Pattern::Lit(_, _) => {}
    }
}

#[cfg(test)]
#[path = "closure_tests.rs"]
mod tests;
