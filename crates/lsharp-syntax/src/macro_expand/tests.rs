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

#[test]
fn test_expand_simple_macro() {
    let prog = parse(
        "(defmacro when [test body] '(if ~test ~body ()))\n\
             (defn f [x] (when (> x 0) (+ x 1)))",
    );

    let mut expander = MacroExpander::new();
    let expanded = expander.expand_program(prog).unwrap();

    assert_eq!(expanded.decls.len(), 1);

    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        assert!(matches!(body, Expr::If(_, _, _, _)));
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_defmacro_removed_from_output() {
    let prog = parse("(defmacro noop [x] '~x)");
    let mut expander = MacroExpander::new();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 0);
}

#[test]
fn test_macro_arity_mismatch() {
    let prog = parse(
        "(defmacro add2 [a b] '(+ ~a ~b))\n\
             (defn f [] (add2 1))",
    );
    let mut expander = MacroExpander::new();
    let result = expander.expand_program(prog);
    assert!(result.is_err());
    // P10-3: WithTrace でラップされることがある
    match result {
        Err(MacroExpandError::WithTrace { inner, .. }) => {
            if let MacroExpandError::ArityMismatch {
                expected, actual, ..
            } = *inner
            {
                assert_eq!(expected, 2);
                assert_eq!(actual, 1);
            } else {
                panic!("Expected ArityMismatch inside WithTrace");
            }
        }
        Err(MacroExpandError::ArityMismatch {
            expected, actual, ..
        }) => {
            assert_eq!(expected, 2);
            assert_eq!(actual, 1);
        }
        _ => panic!("Expected ArityMismatch"),
    }
}

#[test]
fn test_identity_macro() {
    let prog = parse(
        "(defmacro id [x] '~x)\n\
             (defn f [] (id 42))",
    );
    let mut expander = MacroExpander::new();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        assert!(matches!(body, Expr::Lit(_, Literal::Int(42))));
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_nested_macro_call() {
    let prog = parse(
        "(defmacro unless [test body] '(if ~test () ~body))\n\
             (defn f [x] (unless (> x 0) (- x 1)))",
    );
    let mut expander = MacroExpander::new();
    let expanded = expander.expand_program(prog).unwrap();

    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        if let Expr::If(_, cond, then_br, else_br) = body {
            assert!(matches!(cond.as_ref(), Expr::App(_, _, _)));
            assert!(matches!(then_br.as_ref(), Expr::Lit(_, Literal::Unit)));
            assert!(matches!(else_br.as_ref(), Expr::App(_, _, _)));
        } else {
            panic!("Expected If, got {:?}", body);
        }
    }
}

#[test]
fn test_gensym() {
    let mut expander = MacroExpander::new();
    let s1 = expander.gensym("tmp");
    let s2 = expander.gensym("tmp");
    assert_ne!(s1, s2);
    assert!(s1.starts_with("__gensym_tmp_"));
}

#[test]
fn test_no_macro_passthrough() {
    let prog = parse("(defn add [x y] (+ x y))");
    let mut expander = MacroExpander::new();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
}

#[test]
fn test_multiple_macros() {
    let prog = parse(
        "(defmacro when [test body] '(if ~test ~body ()))\n\
             (defmacro unless [test body] '(if ~test () ~body))\n\
             (defn f [x] (when (> x 0) 1))\n\
             (defn g [x] (unless (> x 0) 2))",
    );
    let mut expander = MacroExpander::new();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 2);
}

// --- 組み込みマクロテスト ---

#[test]
fn test_builtin_when() {
    let prog = parse("(defn f [x] (when (> x 0) (+ x 1)))");
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        assert!(matches!(body, Expr::If(_, _, _, _)));
        if let Expr::If(_, _, _, else_br) = body {
            assert!(matches!(else_br.as_ref(), Expr::Lit(_, Literal::Unit)));
        }
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_builtin_unless() {
    let prog = parse("(defn f [x] (unless (> x 0) (- x 1)))");
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        assert!(matches!(body, Expr::If(_, _, _, _)));
        if let Expr::If(_, _, then_br, _) = body {
            assert!(matches!(then_br.as_ref(), Expr::Lit(_, Literal::Unit)));
        }
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_user_macro_overrides_builtin() {
    let prog = parse(
        "(defmacro when [test body] '(if ~test (+ ~body 100) ()))
             (defn f [x] (when (> x 0) x))",
    );
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0]
        && let Expr::If(_, _, then_br, _) = body
    {
        assert!(matches!(then_br.as_ref(), Expr::App(_, _, _)));
    }
}

// --- P10-3: 再帰マクロテスト ---

#[test]
fn test_recursive_macro_depth_limit() {
    let prog = parse(
        "(defmacro loop [x] '(loop ~x))
             (defn f [] (loop 1))",
    );
    let mut expander = MacroExpander::new();
    let result = expander.expand_program(prog);
    assert!(result.is_err());
    match &result {
        Err(MacroExpandError::RecursionLimit { limit, .. }) => {
            assert_eq!(*limit, 128);
        }
        Err(MacroExpandError::WithTrace { inner, .. }) => {
            if let MacroExpandError::RecursionLimit { limit, .. } = inner.as_ref() {
                assert_eq!(*limit, 128);
            } else {
                panic!("Expected RecursionLimit, got {:?}", inner);
            }
        }
        _ => panic!("Expected RecursionLimit, got {:?}", result),
    }
}

#[test]
fn test_mutual_recursive_macros() {
    let prog = parse(
        "(defmacro mac-a [x] '(mac-b ~x))
             (defmacro mac-b [x] '(mac-a ~x))
             (defn f [] (mac-a 1))",
    );
    let mut expander = MacroExpander::new();
    let result = expander.expand_program(prog);
    assert!(result.is_err());
}

#[test]
fn test_finite_recursive_macro() {
    let prog = parse(
        "(defmacro double [x] '(+ ~x ~x))
             (defmacro quad [x] '(double (double ~x)))
             (defn f [] (quad 5))",
    );
    let mut expander = MacroExpander::new();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        assert!(matches!(body, Expr::App(_, _, _)));
    }
}

// --- P10-3: ~@ splice 展開テスト ---

#[test]
fn test_splice_in_apply() {
    let prog = parse(
        "(defmacro wrap [f a b] '(~f ~a ~b))
             (defn test [] (wrap + 1 2))",
    );
    let mut expander = MacroExpander::new();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        if let Expr::App(_, func, args) = body {
            assert!(matches!(func.as_ref(), Expr::Var(_, _)));
            assert_eq!(args.len(), 2);
        } else {
            panic!("Expected App, got {:?}", body);
        }
    }
}

// --- P10-3: 型シグネチャテスト ---

#[test]
fn test_macro_type_sig_stored() {
    // (defmacro typed-when [test body] : (-> Bool Int Int) '(if ~test ~body ()))
    let prog = parse("(defmacro typed-when [test body] : (-> Bool Int Int) '(if ~test ~body ()))");
    let mut expander = MacroExpander::new();
    let _expanded = expander.expand_program(prog).unwrap();
    // 型シグネチャが保存されていることを確認
    let sig = expander.macro_type_sig("typed-when");
    assert!(sig.is_some(), "型シグネチャが保存されているべき");
}

#[test]
fn test_macro_without_type_sig() {
    let prog = parse("(defmacro noop [x] '~x)");
    let mut expander = MacroExpander::new();
    let _expanded = expander.expand_program(prog).unwrap();
    let sig = expander.macro_type_sig("noop");
    assert!(sig.is_none(), "型シグネチャなしの場合は None");
}

// --- P10-3: 展開トレースバックテスト ---

#[test]
fn test_expansion_trace_recorded() {
    let prog = parse(
        "(defmacro double [x] '(+ ~x ~x))
             (defn f [] (double 5))",
    );
    let mut expander = MacroExpander::new();
    let _expanded = expander.expand_program(prog).unwrap();
    let trace = expander.expansion_trace();
    assert_eq!(trace.len(), 1);
    assert_eq!(trace[0].macro_name, "double");
    assert_eq!(trace[0].depth, 0);
}

#[test]
fn test_nested_expansion_trace() {
    let prog = parse(
        "(defmacro double [x] '(+ ~x ~x))
             (defmacro quad [x] '(double (double ~x)))
             (defn f [] (quad 5))",
    );
    let mut expander = MacroExpander::new();
    let _expanded = expander.expand_program(prog).unwrap();
    let trace = expander.expansion_trace();
    // quad -> double -> double の3段階
    assert!(
        trace.len() >= 2,
        "トレースは少なくとも2エントリ: {:?}",
        trace
    );
    assert_eq!(trace[0].macro_name, "quad");
}

#[test]
fn test_builtin_assert() {
    let prog = parse("(defn f [x] (assert (> x 0)))");
    let mut expander = MacroExpander::with_builtins();
    let expanded = expander.expand_program(prog).unwrap();
    assert_eq!(expanded.decls.len(), 1);
    if let Decl::Defn { body, .. } = &expanded.decls[0] {
        // assert は if に展開される
        assert!(matches!(body, Expr::If(_, _, _, _)));
        if let Expr::If(_, _, then_br, else_br) = body {
            // then: ()
            assert!(matches!(then_br.as_ref(), Expr::Lit(_, Literal::Unit)));
            // else: (do (print "Assertion failed") 0)
            assert!(matches!(else_br.as_ref(), Expr::Do(_, _)));
        }
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_traceback_format() {
    let err = MacroExpandError::RecursionLimit {
        name: "loop".to_string(),
        limit: 128,
        span: Span::new(0, 10),
    };
    let trace = vec![
        MacroExpansionStep {
            macro_name: "loop".to_string(),
            call_span: Span::new(0, 10),
            depth: 0,
        },
        MacroExpansionStep {
            macro_name: "loop".to_string(),
            call_span: Span::new(0, 10),
            depth: 1,
        },
    ];
    let with_trace = err.with_trace(trace);
    let formatted = with_trace.format_traceback();
    assert!(
        formatted.contains("トレースバック"),
        "フォーマットにトレースバックが含まれるべき: {formatted}"
    );
    assert!(
        formatted.contains("loop"),
        "マクロ名が含まれるべき: {formatted}"
    );
}
