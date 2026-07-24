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
