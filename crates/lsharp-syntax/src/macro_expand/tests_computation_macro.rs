use super::*;
use crate::ast::{Decl, Expr};
use crate::lexer::Lexer;
use crate::parser::Parser;

fn parse(input: &str) -> Program {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    parser.parse_program().unwrap()
}

// --- P10-5: Computation Expression マクロ化テスト ---

#[test]
fn test_computation_return_desugared() {
    // (computation maybe (return 42)) => (maybe-return 42)
    let prog = parse(
        "(computation-builder maybe maybe-bind maybe-return)\n\
             (defn test [] (computation maybe (return 42)))",
    );
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    // computation-builder + defn = 2 decls
    assert_eq!(expanded.decls.len(), 2);
    if let Decl::Defn { body, .. } = &expanded.decls[1] {
        // (maybe-return 42) に脱糖されるべき
        if let Expr::App(_, func, args) = body {
            assert!(
                matches!(func.as_ref(), Expr::Var(_, name) if name == "maybe-return"),
                "return は maybe-return に脱糖されるべき: {:?}",
                func
            );
            assert_eq!(args.len(), 1);
        } else {
            panic!("Expected App (maybe-return 42), got {:?}", body);
        }
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_computation_let_bang_desugared() {
    // (computation maybe (let! x (get-value)) (return x))
    //   => (maybe-bind (get-value) (fn [x] (maybe-return x)))
    let prog = parse(
        "(computation-builder maybe maybe-bind maybe-return)\n\
             (defn test [] (computation maybe (let! x (get-value)) (return x)))",
    );
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 2);
    if let Decl::Defn { body, .. } = &expanded.decls[1] {
        // (maybe-bind (get-value) (fn [x] (maybe-return x)))
        if let Expr::App(_, func, args) = body {
            assert!(
                matches!(func.as_ref(), Expr::Var(_, name) if name == "maybe-bind"),
                "let! は maybe-bind に脱糖されるべき: {:?}",
                func
            );
            assert_eq!(args.len(), 2, "bind は2引数であるべき");
            // 第2引数が Lambda であること
            assert!(
                matches!(&args[1], Expr::Lambda(_, params, _) if params.len() == 1),
                "bind の第2引数は Lambda であるべき: {:?}",
                args[1]
            );
        } else {
            panic!("Expected App (maybe-bind ...), got {:?}", body);
        }
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_computation_do_bang_desugared() {
    // (computation maybe (do! (side-effect)) (return 42))
    //   => (maybe-bind (side-effect) (fn [_] (maybe-return 42)))
    let prog = parse(
        "(computation-builder maybe maybe-bind maybe-return)\n\
             (defn test [] (computation maybe (do! (side-effect)) (return 42)))",
    );
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 2);
    if let Decl::Defn { body, .. } = &expanded.decls[1] {
        if let Expr::App(_, func, args) = body {
            assert!(
                matches!(func.as_ref(), Expr::Var(_, name) if name == "maybe-bind"),
                "do! は maybe-bind に脱糖されるべき: {:?}",
                func
            );
            assert_eq!(args.len(), 2);
            // 第2引数の Lambda パラメータ名が gensym (無視用) であること
            if let Expr::Lambda(_, params, _) = &args[1] {
                assert!(
                    params[0].name.starts_with("__gensym_"),
                    "do! の Lambda パラメータは gensym であるべき: {}",
                    params[0].name
                );
            }
        } else {
            panic!("Expected App, got {:?}", body);
        }
    }
}

#[test]
fn test_computation_chain_desugared() {
    // (computation maybe (let! x (get-value)) (let! y (process x)) (return y))
    //   => (maybe-bind (get-value) (fn [x] (maybe-bind (process x) (fn [y] (maybe-return y)))))
    let prog = parse(
        "(computation-builder maybe maybe-bind maybe-return)\n\
             (defn test [] (computation maybe (let! x (get-value)) (let! y (process x)) (return y)))",
    );
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 2);
    if let Decl::Defn { body, .. } = &expanded.decls[1] {
        // 外側: (maybe-bind (get-value) (fn [x] ...))
        if let Expr::App(_, func, args) = body {
            assert!(matches!(func.as_ref(), Expr::Var(_, name) if name == "maybe-bind"));
            // 内側 Lambda 内: (maybe-bind (process x) (fn [y] (maybe-return y)))
            if let Expr::Lambda(_, _, inner_body) = &args[1] {
                if let Expr::App(_, inner_func, _) = inner_body.as_ref() {
                    assert!(
                        matches!(inner_func.as_ref(), Expr::Var(_, name) if name == "maybe-bind"),
                        "チェーンの内側も maybe-bind であるべき: {:?}",
                        inner_func
                    );
                } else {
                    panic!("Expected inner App, got {:?}", inner_body);
                }
            }
        } else {
            panic!("Expected App, got {:?}", body);
        }
    }
}

#[test]
fn test_computation_trace_recorded() {
    let prog = parse(
        "(computation-builder maybe maybe-bind maybe-return)\n\
             (defn test [] (computation maybe (return 42)))",
    );
    let mut expander = MacroExpander::with_builtins();
    let _expanded = expander.expand_program(prog).unwrap();
    let trace = expander.expansion_trace();
    assert!(
        !trace.is_empty(),
        "computation 展開のトレースが記録されるべき"
    );
    assert_eq!(trace[0].macro_name, "computation:maybe");
}

#[test]
fn test_computation_without_builder_preserved() {
    // ビルダー未登録の場合は Computation ノードをそのまま残す
    let prog = parse("(defn test [] (computation unknown (return 42)))");
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        assert!(
            matches!(body, Expr::Computation(_, name, _) if name == "unknown"),
            "未登録ビルダーの場合は Computation を保持: {:?}",
            body
        );
    }
}
