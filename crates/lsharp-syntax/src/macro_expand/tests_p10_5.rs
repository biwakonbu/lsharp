use super::*;
use crate::ast::{Decl, Expr, Literal};
use crate::lexer::Lexer;
use crate::parser::Parser;

fn parse(input: &str) -> Program {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    parser.parse_program().unwrap()
}

// --- P10-5: cond マクロテスト ---

#[test]
fn test_builtin_cond_two_branches() {
    // (cond (> x 0) 1 (< x 0) -1 0) -> (if (> x 0) 1 (if (< x 0) -1 0))
    let prog = parse("(defn f [x] (cond (> x 0) 1 (< x 0) (- 0 1) 0))");
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        // 外側の if
        if let Expr::If(_, _, _, else_br) = body {
            // 内側も if
            assert!(
                matches!(else_br.as_ref(), Expr::If(_, _, _, _)),
                "cond の else 分岐が if に展開されるべき: {:?}",
                else_br
            );
        } else {
            panic!("Expected If, got {:?}", body);
        }
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_builtin_cond_single_branch() {
    // (cond true 42 0) -> (if true 42 0)
    let prog = parse("(defn f [] (cond true 42 0))");
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        assert!(
            matches!(body, Expr::If(_, _, _, _)),
            "cond は if に展開されるべき: {:?}",
            body
        );
    }
}

#[test]
fn test_builtin_cond_default_only() {
    // (cond 42) -> 42
    let prog = parse("(defn f [] (cond 42))");
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        assert!(
            matches!(body, Expr::Lit(_, Literal::Int(42))),
            "cond のデフォルト値のみの場合はそのまま返すべき: {:?}",
            body
        );
    }
}

// --- P10-5: |> パイプラインマクロテスト ---

#[test]
fn test_builtin_pipe_forward_single() {
    // (|> 42 print) -> (print 42)
    let prog = parse("(defn f [] (|> 42 print))");
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        if let Expr::App(_, func, args) = body {
            assert!(
                matches!(func.as_ref(), Expr::Var(_, name) if name == "print"),
                "pipe の関数が print であるべき: {:?}",
                func
            );
            assert_eq!(args.len(), 1);
            assert!(
                matches!(&args[0], Expr::Lit(_, Literal::Int(42))),
                "pipe の引数が 42 であるべき: {:?}",
                args
            );
        } else {
            panic!("Expected App, got {:?}", body);
        }
    }
}

#[test]
fn test_builtin_pipe_forward_chain() {
    // (|> 1 (+ 2) (+ 3)) -> (+ 3 (+ 2 1))
    // つまり (+ (+ 1 2) 3) のようなネスト
    let prog = parse("(defn f [] (|> 1 (+ 2) (+ 3)))");
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        // 最外の App: (+ 3 ...)
        if let Expr::App(_, func, args) = body {
            assert!(
                matches!(func.as_ref(), Expr::Var(_, name) if name == "+"),
                "最外の関数が + であるべき: {:?}",
                func
            );
            // 引数は [2, (+ 2 1)] の2つ (部分適用 + パイプ引数)
            assert_eq!(args.len(), 2, "引数が2つあるべき: {:?}", args);
        } else {
            panic!("Expected App, got {:?}", body);
        }
    }
}

#[test]
fn test_builtin_pipe_forward_value_only() {
    // (|> 42) -> 42
    let prog = parse("(defn f [] (|> 42))");
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        assert!(
            matches!(body, Expr::Lit(_, Literal::Int(42))),
            "値のみの場合はそのまま返すべき: {:?}",
            body
        );
    }
}

#[test]
fn test_cond_trace_recorded() {
    let prog = parse("(defn f [x] (cond (> x 0) 1 0))");
    let mut expander = MacroExpander::with_builtins();
    let _expanded = expander.expand_program(prog).unwrap();
    let trace = expander.expansion_trace();
    assert!(!trace.is_empty(), "cond 展開のトレースが記録されるべき");
    assert_eq!(trace[0].macro_name, "cond");
}

#[test]
fn test_pipe_trace_recorded() {
    let prog = parse("(defn f [] (|> 42 print))");
    let mut expander = MacroExpander::with_builtins();
    let _expanded = expander.expand_program(prog).unwrap();
    let trace = expander.expansion_trace();
    assert!(!trace.is_empty(), "|> 展開のトレースが記録されるべき");
    assert_eq!(trace[0].macro_name, "|>");
}
