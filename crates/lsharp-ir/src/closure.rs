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
mod tests {
    use super::*;
    use lsharp_syntax::span::Span;

    /// テスト用のダミー Span
    fn s() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn test_no_free_variables() {
        // (fn [x] x) - パラメータのみ参照
        let fv = free_variables(&["x".to_string()], &Expr::Var(s(), "x".to_string()));
        assert!(fv.is_empty());
    }

    #[test]
    fn test_single_free_variable() {
        // (fn [x] y) - y は自由変数
        let fv = free_variables(&["x".to_string()], &Expr::Var(s(), "y".to_string()));
        assert_eq!(fv.len(), 1);
        assert!(fv.contains("y"));
    }

    #[test]
    fn test_free_in_apply() {
        // (fn [x] (f x y)) - f と y は自由変数
        let expr = Expr::App(
            s(),
            Box::new(Expr::Var(s(), "f".to_string())),
            vec![
                Expr::Var(s(), "x".to_string()),
                Expr::Var(s(), "y".to_string()),
            ],
        );
        let fv = free_variables(&["x".to_string()], &expr);
        assert_eq!(fv.len(), 2);
        assert!(fv.contains("f"));
        assert!(fv.contains("y"));
    }

    #[test]
    fn test_let_binds_variable() {
        // (fn [] (let [x 1] x)) - x は let でバインドされるので自由変数ではない
        let expr = Expr::Let(
            s(),
            vec![(
                Pattern::Var(s(), "x".to_string()),
                Expr::Lit(s(), Literal::Int(1)),
            )],
            Box::new(Expr::Var(s(), "x".to_string())),
        );
        let fv = free_variables(&[], &expr);
        assert!(fv.is_empty());
    }

    #[test]
    fn test_nested_lambda() {
        // (fn [x] (fn [y] (+ x y z))) - z と + は自由変数、x は外側でバインド
        let inner = Expr::Lambda(
            s(),
            vec![Param {
                span: s(),
                name: "y".to_string(),
                ty: None,
            }],
            Box::new(Expr::App(
                s(),
                Box::new(Expr::Var(s(), "+".to_string())),
                vec![
                    Expr::Var(s(), "x".to_string()),
                    Expr::Var(s(), "y".to_string()),
                    Expr::Var(s(), "z".to_string()),
                ],
            )),
        );
        let fv = free_variables(&["x".to_string()], &inner);
        assert_eq!(fv.len(), 2);
        assert!(fv.contains("+"));
        assert!(fv.contains("z"));
    }

    #[test]
    fn test_if_expression() {
        // (fn [x] (if x y z)) - y と z は自由変数
        let expr = Expr::If(
            s(),
            Box::new(Expr::Var(s(), "x".to_string())),
            Box::new(Expr::Var(s(), "y".to_string())),
            Box::new(Expr::Var(s(), "z".to_string())),
        );
        let fv = free_variables(&["x".to_string()], &expr);
        assert_eq!(fv.len(), 2);
        assert!(fv.contains("y"));
        assert!(fv.contains("z"));
    }

    #[test]
    fn test_match_pattern_binds() {
        // (fn [x] (match x [(Some v) v] [None y]))
        // v はパターンでバインド、y は自由変数
        let expr = Expr::Match(
            s(),
            Box::new(Expr::Var(s(), "x".to_string())),
            vec![
                MatchArm {
                    span: s(),
                    pattern: Pattern::Constructor(
                        s(),
                        "Some".to_string(),
                        vec![Pattern::Var(s(), "v".to_string())],
                    ),
                    body: Expr::Var(s(), "v".to_string()),
                    guard: None,
                },
                MatchArm {
                    span: s(),
                    pattern: Pattern::Constructor(s(), "None".to_string(), vec![]),
                    body: Expr::Var(s(), "y".to_string()),
                    guard: None,
                },
            ],
        );
        let fv = free_variables(&["x".to_string()], &expr);
        assert_eq!(fv.len(), 1);
        assert!(fv.contains("y"));
    }

    #[test]
    fn test_literal_no_free_vars() {
        // (fn [] 42) - リテラルは自由変数なし
        let fv = free_variables(&[], &Expr::Lit(s(), Literal::Int(42)));
        assert!(fv.is_empty());
    }

    #[test]
    fn test_do_expression() {
        // (fn [x] (do x y)) - y は自由変数
        let expr = Expr::Do(
            s(),
            vec![
                Expr::Var(s(), "x".to_string()),
                Expr::Var(s(), "y".to_string()),
            ],
        );
        let fv = free_variables(&["x".to_string()], &expr);
        assert_eq!(fv.len(), 1);
        assert!(fv.contains("y"));
    }

    #[test]
    fn test_let_value_sees_outer_scope() {
        // (fn [] (let [x y] x)) - y は let の値式で自由変数
        let expr = Expr::Let(
            s(),
            vec![(
                Pattern::Var(s(), "x".to_string()),
                Expr::Var(s(), "y".to_string()),
            )],
            Box::new(Expr::Var(s(), "x".to_string())),
        );
        let fv = free_variables(&[], &expr);
        assert_eq!(fv.len(), 1);
        assert!(fv.contains("y"));
    }
}
