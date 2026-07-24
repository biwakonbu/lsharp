//! derive 展開の回帰テスト

use super::*;
use crate::ast::{TypeExpr, Variant};

fn make_variant(name: &str, field_count: usize) -> Variant {
    Variant {
        span: Span::new(0, 0),
        name: name.to_string(),
        fields: (0..field_count)
            .map(|_| TypeExpr::Named(Span::new(0, 0), "Int".to_string()))
            .collect(),
        return_type: None,
    }
}

#[test]
fn test_derive_show_adt_no_args() {
    let variants = vec![make_variant("Nothing", 0)];
    let decl = derive_show_adt("Maybe", &variants);
    if let Decl::Defn { name, body, .. } = &decl {
        assert_eq!(name, "show-Maybe");
        // match 式であること
        assert!(matches!(body, Expr::Match(_, _, _)));
        if let Expr::Match(_, _, arms) = body {
            assert_eq!(arms.len(), 1);
            // Nothing -> "Nothing"
            assert!(matches!(
                &arms[0].body,
                Expr::Lit(_, Literal::String(s)) if s == "Nothing"
            ));
        }
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_derive_show_adt_with_args() {
    let variants = vec![make_variant("Just", 1), make_variant("Nothing", 0)];
    let decl = derive_show_adt("Maybe", &variants);
    if let Decl::Defn { name, body, .. } = &decl {
        assert_eq!(name, "show-Maybe");
        if let Expr::Match(_, _, arms) = body {
            assert_eq!(arms.len(), 2);
            // Just arm: パターンに1引数
            if let Pattern::Constructor(_, cname, pats) = &arms[0].pattern {
                assert_eq!(cname, "Just");
                assert_eq!(pats.len(), 1);
            }
            // Nothing arm: 文字列リテラル
            assert!(matches!(
                &arms[1].body,
                Expr::Lit(_, Literal::String(s)) if s == "Nothing"
            ));
        }
    }
}

#[test]
fn test_derive_eq_adt() {
    let variants = vec![make_variant("Just", 1), make_variant("Nothing", 0)];
    let decl = derive_eq_adt("Maybe", &variants);
    if let Decl::Defn {
        name, params, body, ..
    } = &decl
    {
        assert_eq!(name, "eq-Maybe?");
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "a");
        assert_eq!(params[1].name, "b");
        // 外側は match on a
        assert!(matches!(body, Expr::Match(_, _, _)));
        if let Expr::Match(_, scrutinee, arms) = body {
            assert!(matches!(scrutinee.as_ref(), Expr::Var(_, n) if n == "a"));
            assert_eq!(arms.len(), 2);
            // 各腕のボディは内側の match on b
            assert!(matches!(&arms[0].body, Expr::Match(_, _, _)));
        }
    }
}

#[test]
fn test_derive_eq_adt_no_args() {
    let variants = vec![make_variant("Nothing", 0), make_variant("Something", 0)];
    let decl = derive_eq_adt("Option", &variants);
    if let Decl::Defn { body, .. } = &decl
        && let Expr::Match(_, _, arms) = body
    {
        assert_eq!(arms.len(), 2);
        // Nothing 腕の内側 match
        if let Expr::Match(_, _, inner_arms) = &arms[0].body {
            assert_eq!(inner_arms.len(), 2); // Nothing + wildcard
            // Nothing -> true
            assert!(matches!(
                &inner_arms[0].body,
                Expr::Lit(_, Literal::Bool(true))
            ));
            // _ -> false
            assert!(matches!(
                &inner_arms[1].body,
                Expr::Lit(_, Literal::Bool(false))
            ));
        }
    }
}

#[test]
fn test_derive_show_record() {
    let fields = vec![
        (
            "x".to_string(),
            TypeExpr::Named(Span::new(0, 0), "Int".to_string()),
        ),
        (
            "y".to_string(),
            TypeExpr::Named(Span::new(0, 0), "Int".to_string()),
        ),
    ];
    let decl = derive_show_record("Point", &fields);
    if let Decl::Defn { name, params, .. } = &decl {
        assert_eq!(name, "show-Point");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "r");
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_derive_eq_multi_field() {
    let variants = vec![make_variant("Pair", 2)];
    let decl = derive_eq_adt("Pair", &variants);
    if let Decl::Defn { body, .. } = &decl
        && let Expr::Match(_, _, arms) = body
    {
        assert_eq!(arms.len(), 1);
        // Pair(a0, a1) のパターン
        if let Pattern::Constructor(_, _, pats) = &arms[0].pattern {
            assert_eq!(pats.len(), 2);
        }
        // 内側 match の成功ボディは If (a0==b0) (a1==b1) false
        if let Expr::Match(_, _, inner_arms) = &arms[0].body {
            assert!(matches!(&inner_arms[0].body, Expr::If(_, _, _, _)));
        }
    }
}

#[test]
fn test_apply_derives() {
    use crate::ast::Program;
    let s = Span::new(0, 0);
    let program = Program {
        decls: vec![Decl::TypeDef {
            span: s,
            name: "Color".to_string(),
            type_params: vec![],
            variants: vec![
                make_variant("Red", 0),
                make_variant("Green", 0),
                make_variant("Blue", 0),
            ],
            metadata: None,
        }],
    };

    let derived = apply_derives(&program);
    assert_eq!(derived.len(), 2); // show + eq
    if let Decl::Defn { name, .. } = &derived[0] {
        assert_eq!(name, "show-Color");
    }
    if let Decl::Defn { name, .. } = &derived[1] {
        assert_eq!(name, "eq-Color?");
    }
}
