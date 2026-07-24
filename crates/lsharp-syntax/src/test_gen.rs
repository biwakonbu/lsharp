//! property test 専用の bounded AST generator。
//!
//! 生成する AST は parser の syntax surface に限定し、runtime dependency や公開 API には露出しない。

use proptest::prelude::*;

use crate::ast::{Expr, Literal, Param, Pattern, TypeExpr};
use crate::span::Span;

fn span() -> Span {
    Span::new(0, 0)
}

fn name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("x".to_string()),
        Just("y".to_string()),
        Just("value".to_string()),
        Just("f".to_string()),
    ]
}

fn safe_string() -> impl Strategy<Value = String> {
    prop::collection::vec(0u8..=25, 0..6).prop_map(|chars| {
        chars
            .into_iter()
            .map(|value| (b'a' + value) as char)
            .collect()
    })
}

/// 深さ 3、各コレクション最大 2 要素の小さな式を生成する。
pub(crate) fn arb_expr() -> impl Strategy<Value = Expr> {
    let leaves = prop_oneof![
        (0i64..100).prop_map(|value| Expr::Lit(span(), Literal::Int(value))),
        any::<bool>().prop_map(|value| Expr::Lit(span(), Literal::Bool(value))),
        safe_string().prop_map(|value| Expr::Lit(span(), Literal::String(value))),
        Just(Expr::Lit(span(), Literal::Unit)),
        name().prop_map(|value| Expr::Var(span(), value)),
    ];

    leaves.prop_recursive(3, 32, 4, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone(), inner.clone()).prop_map(|(cond, then, else_)| {
                Expr::If(span(), Box::new(cond), Box::new(then), Box::new(else_))
            }),
            (prop::collection::vec(inner.clone(), 1..3), inner.clone()).prop_map(
                |(values, body)| {
                    let bindings = values
                        .into_iter()
                        .map(|value| (Pattern::Var(span(), "x".to_string()), value))
                        .collect();
                    Expr::Let(span(), bindings, Box::new(body))
                }
            ),
            inner.clone().prop_map(|body| {
                Expr::Lambda(
                    span(),
                    vec![Param {
                        span: span(),
                        name: "x".to_string(),
                        ty: None,
                    }],
                    Box::new(body),
                )
            }),
            (inner.clone(), prop::collection::vec(inner.clone(), 1..3))
                .prop_map(|(func, args)| Expr::App(span(), Box::new(func), args)),
            prop::collection::vec(inner.clone(), 1..3).prop_map(|exprs| Expr::Do(span(), exprs)),
            inner.clone().prop_map(|expr| {
                Expr::Ann(
                    span(),
                    Box::new(expr),
                    TypeExpr::Named(span(), "Int".to_string()),
                )
            }),
            (prop::collection::vec((Just("value".to_string()), inner.clone()), 1..3))
                .prop_map(|fields| Expr::RecordLit(span(), "Box".to_string(), fields)),
            inner
                .clone()
                .prop_map(|expr| Expr::Quote(span(), Box::new(expr))),
        ]
    })
}
