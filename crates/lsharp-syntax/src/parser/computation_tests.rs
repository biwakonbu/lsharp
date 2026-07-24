use super::Parser;
use crate::ast::{ComputationStep, Decl, Expr};
use crate::lexer::Lexer;
use crate::parse;

fn parse_expr_str(input: &str) -> Expr {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    parser.parse_expr().unwrap()
}

#[test]
fn test_computation_builder_decl() {
    let prog = parse("(computation-builder maybe maybe-bind maybe-return)").unwrap();
    assert_eq!(prog.decls.len(), 1);
    if let Decl::ComputationBuilder {
        name,
        bind_fn,
        return_fn,
        ..
    } = &prog.decls[0]
    {
        assert_eq!(name, "maybe");
        assert_eq!(bind_fn, "maybe-bind");
        assert_eq!(return_fn, "maybe-return");
    } else {
        panic!("Expected ComputationBuilder");
    }
}

#[test]
fn test_computation_expr_basic() {
    let prog = parse("(defn test [] (computation maybe (let! x (get-value)) (return x)))").unwrap();
    if let Decl::Defn { body, .. } = &prog.decls[0] {
        if let Expr::Computation(_, builder, steps) = &body {
            assert_eq!(builder, "maybe");
            assert_eq!(steps.len(), 2);
            assert!(matches!(&steps[0], ComputationStep::LetBang(..)));
            assert!(matches!(&steps[1], ComputationStep::Return(..)));
        } else {
            panic!("Expected Computation expr, got: {body}");
        }
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_computation_expr_do_bang() {
    let prog = parse("(defn test [] (computation async (do! (print 1)) (return 42)))").unwrap();
    if let Decl::Defn { body, .. } = &prog.decls[0] {
        if let Expr::Computation(_, builder, steps) = &body {
            assert_eq!(builder, "async");
            assert_eq!(steps.len(), 2);
            assert!(matches!(&steps[0], ComputationStep::DoBang(..)));
            assert!(matches!(&steps[1], ComputationStep::Return(..)));
        } else {
            panic!("Expected Computation expr");
        }
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_computation_expr_with_plain_expr() {
    let prog = parse("(defn test [] (computation maybe (let! x (get-value)) (+ x 1)))").unwrap();
    if let Decl::Defn { body, .. } = &prog.decls[0] {
        if let Expr::Computation(_, _, steps) = &body {
            assert_eq!(steps.len(), 2);
            assert!(matches!(&steps[0], ComputationStep::LetBang(..)));
            assert!(matches!(&steps[1], ComputationStep::Expr(_)));
        } else {
            panic!("Expected Computation expr");
        }
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_computation_display() {
    let prog = parse("(defn test [] (computation maybe (let! x (get-value)) (return x)))").unwrap();
    let display = format!("{}", prog.decls[0]);
    assert!(display.contains("maybe"));
}

#[test]
fn test_computation_display_roundtrips_to_parser_syntax() {
    let prog = parse("(defn test [] (computation maybe (let! x (get-value)) (return x)))").unwrap();

    assert_eq!(
        format!("{}", prog.decls[0]),
        "(defn test [] (computation maybe (let! x (get-value)) (return x)))"
    );
}

// --- P10-1: Quote/Unquote テスト ---

#[test]
fn test_quote_simple() {
    let expr = parse_expr_str("'(+ 1 2)");
    assert!(matches!(expr, Expr::Quote(_, _)));
    assert_eq!(expr.to_string(), "'(+ 1 2)");
}

#[test]
fn test_quote_symbol() {
    let expr = parse_expr_str("'x");
    assert!(matches!(expr, Expr::Quote(_, _)));
    assert_eq!(expr.to_string(), "'x");
}

#[test]
fn test_unquote_simple() {
    let expr = parse_expr_str("~x");
    assert!(matches!(expr, Expr::Unquote(_, _)));
    assert_eq!(expr.to_string(), "~x");
}

#[test]
fn test_splice_unquote() {
    let expr = parse_expr_str("~@args");
    assert!(matches!(expr, Expr::UnquoteSplice(_, _)));
    assert_eq!(expr.to_string(), "~@args");
}

#[test]
fn test_quote_with_unquote() {
    // '(+ ~x 1) -- quote 内に unquote を含む
    let expr = parse_expr_str("'(+ ~x 1)");
    assert!(matches!(expr, Expr::Quote(_, _)));
    if let Expr::Quote(_, inner) = &expr {
        if let Expr::App(_, _, args) = inner.as_ref() {
            assert_eq!(args.len(), 2);
            assert!(matches!(&args[0], Expr::Unquote(_, _)));
        } else {
            panic!("Expected App inside Quote");
        }
    }
}

#[test]
fn test_quote_with_splice_unquote() {
    // '(a ~@rest b)
    let expr = parse_expr_str("'(a ~@rest b)");
    assert!(matches!(expr, Expr::Quote(_, _)));
    if let Expr::Quote(_, inner) = &expr {
        if let Expr::App(_, _, args) = inner.as_ref() {
            assert!(args.iter().any(|a| matches!(a, Expr::UnquoteSplice(_, _))));
        } else {
            panic!("Expected App inside Quote");
        }
    }
}

// --- P10-2: DefMacro テスト ---

#[test]
fn test_defmacro_simple() {
    let prog = parse("(defmacro when [test body] '(if ~test ~body ()))").unwrap();
    assert_eq!(prog.decls.len(), 1);
    if let Decl::DefMacro { name, params, .. } = &prog.decls[0] {
        assert_eq!(name, "when");
        assert_eq!(params.len(), 2);
    } else {
        panic!("Expected DefMacro");
    }
}

#[test]
fn test_defmacro_display() {
    let prog = parse("(defmacro unless [test body] '(if ~test () ~body))").unwrap();
    assert_eq!(
        prog.to_string(),
        "(defmacro unless [test body] '(if ~test () ~body))"
    );
}
