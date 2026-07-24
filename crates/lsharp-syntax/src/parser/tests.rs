use super::*;
use crate::lexer::Lexer;

fn parse(input: &str) -> Program {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    parser.parse_program().unwrap()
}

fn parse_expr_str(input: &str) -> Expr {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    parser.parse_expr().unwrap()
}

#[test]
fn test_simple_defn() {
    let prog = parse("(defn add [x y] (+ x y))");
    assert_eq!(prog.decls.len(), 1);
    assert_eq!(prog.to_string(), "(defn add [x y] (+ x y))");
}

#[test]
fn test_defn_with_type_annotation() {
    let prog = parse("(defn add [(: x Int) (: y Int)] : Int (+ x y))");
    assert_eq!(prog.decls.len(), 1);
    assert_eq!(prog.to_string(), "(defn add [x y] : Int (+ x y))");
}

#[test]
fn test_fib() {
    let prog = parse("(defn fib [n] (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2)))))");
    assert_eq!(prog.decls.len(), 1);
}

#[test]
fn test_let_expr() {
    let expr = parse_expr_str("(let [x 10 y 20] (+ x y))");
    assert_eq!(expr.to_string(), "(let [x 10 y 20] (+ x y))");
}

#[test]
fn test_lambda() {
    let expr = parse_expr_str("(fn [x y] (+ x y))");
    assert_eq!(expr.to_string(), "(fn [x y] (+ x y))");
}

#[test]
fn test_match() {
    let expr = parse_expr_str("(match x [(Some v) v] [None 0])");
    assert_eq!(expr.to_string(), "(match x [(Some v) v] [None 0])");
}

#[test]
fn test_type_def() {
    let prog = parse("(type (Option a) (Some a) None)");
    assert_eq!(prog.decls.len(), 1);
    assert_eq!(prog.to_string(), "(type (Option a) (Some a) None)");
}

#[test]
fn test_do_expr() {
    let expr = parse_expr_str("(do (print 1) (print 2))");
    assert_eq!(expr.to_string(), "(do (print 1) (print 2))");
}

#[test]
fn test_type_annotation_expr() {
    let expr = parse_expr_str("(: 42 Int)");
    assert_eq!(expr.to_string(), "(: 42 Int)");
}

#[test]
fn test_unit() {
    let expr = parse_expr_str("()");
    assert_eq!(expr.to_string(), "()");
}

#[test]
fn test_multiple_decls() {
    let prog = parse(
        "(defn add [x y] (+ x y))
             (defn main [] (print (add 1 2)))",
    );
    assert_eq!(prog.decls.len(), 2);
}

// --- レコード型テスト ---

#[test]
fn test_record_type_def() {
    let prog = parse("(type Point (record (: x Float) (: y Float)))");
    assert_eq!(prog.decls.len(), 1);
    assert_eq!(
        prog.to_string(),
        "(type Point (record (: x Float) (: y Float)))"
    );
}

#[test]
fn test_parametric_record_type_def() {
    let prog = parse("(type (Pair a b) (record (: fst a) (: snd b)))");
    assert_eq!(prog.decls.len(), 1);
    assert_eq!(
        prog.to_string(),
        "(type (Pair a b) (record (: fst a) (: snd b)))"
    );
}

#[test]
fn test_record_literal() {
    let expr = parse_expr_str("{Point x 1.0 y 2.0}");
    assert_eq!(expr.to_string(), "{Point x 1 y 2}");
}

#[test]
fn test_record_update() {
    let expr = parse_expr_str("{p | x 3.0}");
    assert_eq!(expr.to_string(), "{p | x 3}");
    let reparsed = parse_expr_str(&expr.to_string());
    assert_eq!(reparsed.to_string(), "{p | x 3}");
}

#[test]
fn test_field_access() {
    let expr = parse_expr_str("(. point x)");
    assert_eq!(expr.to_string(), "(. point x)");
    assert!(matches!(
        expr,
        Expr::FieldAccess(_, inner, field)
            if matches!(*inner, Expr::Var(_, ref name) if name == "point")
                && field == "x"
    ));
}

// --- 型エイリアステスト ---

#[test]
fn test_type_alias() {
    let prog = parse("(type-alias Str String)");
    assert_eq!(prog.decls.len(), 1);
    assert_eq!(prog.to_string(), "(type-alias Str String)");
}

#[test]
fn test_parametric_type_alias() {
    let prog = parse("(type-alias (Callback a b) (-> a b))");
    assert_eq!(prog.decls.len(), 1);
}

// --- 制約付き型テスト ---

#[test]
fn test_type_constrained_basic() {
    let prog = parse("(type-constrained Natural Int :constraints [(>= 0)])");
    assert_eq!(prog.decls.len(), 1);
    if let Decl::TypeConstrained {
        name, constraints, ..
    } = &prog.decls[0]
    {
        assert_eq!(name, "Natural");
        assert_eq!(constraints.len(), 1);
    } else {
        panic!("Expected TypeConstrained");
    }
}

#[test]
fn test_type_constrained_range() {
    let prog = parse("(type-constrained Percentage Int :constraints [(>= 0) (<= 100)])");
    assert_eq!(prog.decls.len(), 1);
    if let Decl::TypeConstrained {
        name, constraints, ..
    } = &prog.decls[0]
    {
        assert_eq!(name, "Percentage");
        assert_eq!(constraints.len(), 2);
    } else {
        panic!("Expected TypeConstrained");
    }
}

#[test]
fn test_type_constrained_matches() {
    let prog = parse(r#"(type-constrained Email String :constraints [(matches "^[^@]+@[^@]+$")])"#);
    assert_eq!(prog.decls.len(), 1);
    if let Decl::TypeConstrained { constraints, .. } = &prog.decls[0] {
        assert_eq!(constraints.len(), 1);
        assert!(matches!(&constraints[0], Constraint::Matches(_)));
    } else {
        panic!("Expected TypeConstrained");
    }
}

#[test]
fn test_type_constrained_satisfies() {
    let prog = parse("(type-constrained EvenInt Int :constraints [(satisfies is-even)])");
    assert_eq!(prog.decls.len(), 1);
    if let Decl::TypeConstrained { constraints, .. } = &prog.decls[0] {
        assert_eq!(constraints.len(), 1);
        assert!(matches!(&constraints[0], Constraint::Satisfies(_)));
    } else {
        panic!("Expected TypeConstrained");
    }
}

// --- where 制約テスト ---

#[test]
fn test_defn_with_where_clause() {
    let prog = parse("(defn show-it [x] :where [(Show a)] (show x))");
    assert_eq!(prog.decls.len(), 1);
    if let Decl::Defn { where_clauses, .. } = &prog.decls[0] {
        assert_eq!(where_clauses.len(), 1);
        assert_eq!(where_clauses[0].trait_name, "Show");
        assert_eq!(where_clauses[0].type_var, "a");
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_defn_with_multiple_where_clauses() {
    let prog = parse("(defn show-eq [x y] :where [(Show a) (Eq a)] (do (show x) (== x y)))");
    assert_eq!(prog.decls.len(), 1);
    if let Decl::Defn { where_clauses, .. } = &prog.decls[0] {
        assert_eq!(where_clauses.len(), 2);
    } else {
        panic!("Expected Defn");
    }
}

// --- メタデータテスト ---

#[test]
fn test_defn_with_metadata() {
    let prog = parse(r#"(defn add [x y] :doc "adds two numbers" (+ x y))"#);
    assert_eq!(prog.decls.len(), 1);
    if let Decl::Defn { metadata, .. } = &prog.decls[0] {
        assert!(metadata.is_some());
        let m = metadata.as_ref().unwrap();
        assert_eq!(m.doc.as_deref(), Some("adds two numbers"));
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_defn_with_params_metadata() {
    let prog = parse(
        r#"(defn add [x y] :doc "addition" :params [(x "left") (y "right")] :returns "sum" (+ x y))"#,
    );
    assert_eq!(prog.decls.len(), 1);
    if let Decl::Defn { metadata, .. } = &prog.decls[0] {
        let m = metadata.as_ref().unwrap();
        assert_eq!(m.params.len(), 2);
        assert_eq!(m.params[0].0, "x");
        assert_eq!(m.returns.as_deref(), Some("sum"));
    } else {
        panic!("Expected Defn");
    }
}

// --- モジュールテスト ---

#[test]
fn test_module_decl() {
    let prog = parse("(module MyModule)");
    assert_eq!(prog.decls.len(), 1);
    assert_eq!(prog.to_string(), "(module MyModule)");
}

#[test]
fn test_nested_module_decl() {
    let prog = parse("(module Utils (defn helper [x] (+ x 1)))");
    assert_eq!(prog.decls.len(), 1);
    if let Decl::ModuleDecl { name, body, .. } = &prog.decls[0] {
        assert_eq!(name, "Utils");
        assert_eq!(body.len(), 1);
        assert!(matches!(&body[0], Decl::Defn { name, .. } if name == "helper"));
    } else {
        panic!("Expected ModuleDecl");
    }
}

#[test]
fn test_nested_module_multiple_decls() {
    let prog = parse(
        "(module App.Utils
              (defn add [x y] (+ x y))
              (defn mul [x y] (* x y)))",
    );
    assert_eq!(prog.decls.len(), 1);
    if let Decl::ModuleDecl { name, body, .. } = &prog.decls[0] {
        assert_eq!(name, "App.Utils");
        assert_eq!(body.len(), 2);
    } else {
        panic!("Expected ModuleDecl");
    }
}

#[test]
fn test_nested_module_with_types() {
    let prog = parse(
        "(module Models
              (type Point (record (: x Float) (: y Float)))
              (defn origin [] {Point x 0.0 y 0.0}))",
    );
    assert_eq!(prog.decls.len(), 1);
    if let Decl::ModuleDecl { name, body, .. } = &prog.decls[0] {
        assert_eq!(name, "Models");
        assert_eq!(body.len(), 2);
        assert!(matches!(&body[0], Decl::RecordDef { .. }));
        assert!(matches!(&body[1], Decl::Defn { .. }));
    } else {
        panic!("Expected ModuleDecl");
    }
}

#[test]
fn test_nested_module_display() {
    let prog = parse("(module Utils (defn id [x] x))");
    assert_eq!(prog.to_string(), "(module Utils (defn id [x] x))");
}

#[test]
fn test_deeply_nested_modules() {
    let prog = parse(
        "(module App
              (module Sub
                (defn inner [] 42)))",
    );
    assert_eq!(prog.decls.len(), 1);
    if let Decl::ModuleDecl { name, body, .. } = &prog.decls[0] {
        assert_eq!(name, "App");
        assert_eq!(body.len(), 1);
        if let Decl::ModuleDecl {
            name: inner_name,
            body: inner_body,
            ..
        } = &body[0]
        {
            assert_eq!(inner_name, "Sub");
            assert_eq!(inner_body.len(), 1);
        } else {
            panic!("Expected inner ModuleDecl");
        }
    } else {
        panic!("Expected ModuleDecl");
    }
}

#[test]
fn test_import_decl() {
    let prog = parse("(import MyModule)");
    assert_eq!(prog.decls.len(), 1);
}

// --- トレイトテスト ---

#[test]
fn test_trait_def() {
    let prog = parse("(trait (Show a) (defn show [self] : String))");
    assert_eq!(prog.decls.len(), 1);
}

#[test]
fn test_impl_def() {
    let prog = parse("(impl (Show Int) (defn show [self] (str self)))");
    assert_eq!(prog.decls.len(), 1);
}

// --- エラーリカバリテスト ---

fn parse_result(input: &str) -> Result<Program, ParseError> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

#[test]
fn test_parse_multiple_errors() {
    // 2つの壊れた宣言がある入力で、両方のエラーが報告されることを確認
    let source = "(defn [) (defn [)";
    let result = parse_result(source);
    assert!(result.is_err());
    if let Err(ParseError::Multiple(errors)) = &result {
        assert!(
            errors.len() >= 2,
            "Expected at least 2 errors, got {}",
            errors.len()
        );
    } else {
        panic!("Expected Multiple error variant, got: {:?}", result);
    }
}

#[test]
fn test_parse_recovery_after_error() {
    // 最初の宣言にエラーがあっても、2番目の宣言がパースされることを確認
    // parse_program は回復して継続するが、エラーがあるので Err を返す
    let source = "(defn [) (defn ok [] 42)";
    let result = parse_result(source);
    assert!(result.is_err());
    // 単一エラーが返ること（2番目はパース成功するため）
    assert!(
        !matches!(result, Err(ParseError::Multiple(_))),
        "Expected single error (second decl should parse ok), got: {:?}",
        result
    );
}

#[test]
fn test_parse_recovery_returns_partial_result() {
    // エラーがあっても、正常な宣言はパースされることを確認
    // parse_program_recovering で部分的な結果を取得可能
    let source = "(defn [) (defn ok [] 42)";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let (prog, errors) = parser.parse_program_recovering();
    assert_eq!(errors.len(), 1, "Expected 1 error");
    assert_eq!(prog.decls.len(), 1, "Expected 1 successfully parsed decl");
}

#[test]
fn test_parse_single_error_still_works() {
    // 1つだけエラーがある場合、従来通り単一のエラーが返ることを確認
    let source = "(unknown-form x)";
    let result = parse_result(source);
    assert!(result.is_err());
}

#[test]
fn test_parse_no_error_unchanged() {
    // エラーがない場合は従来通り Ok が返ることを確認
    let source = "(defn foo [] 42) (defn bar [] 99)";
    let result = parse_result(source);
    assert!(result.is_ok());
    let prog = result.unwrap();
    assert_eq!(prog.decls.len(), 2);
}

// 深いネスト回帰テスト: parse_expr / parse_pattern / parse_type_expr が
// 再帰で stack overflow しないことを検証する
fn build_deep_if(depth: usize) -> String {
    let mut s = String::new();
    for _ in 0..depth {
        s.push_str("(if true ");
    }
    s.push('0');
    for _ in 0..depth {
        s.push_str(" 1)");
    }
    s
}

#[test]
fn test_deep_nested_if_50() {
    let expr = parse_expr_str(&build_deep_if(50));
    // 壊れていないことだけ確認
    let _ = expr;
}

#[test]
fn test_deep_nested_if_500() {
    let expr = parse_expr_str(&build_deep_if(500));
    let _ = expr;
}

#[test]
fn test_gadt_variant_with_return_type() {
    // GADT: (: (IntLit Int) (Expr Int)) でバリアントの戻り型を指定
    let source = "(type (Expr a)
  (: (IntLit Int) (Expr Int))
  (: (BoolLit Bool) (Expr Bool)))";
    let prog = parse(source);
    assert_eq!(prog.decls.len(), 1);
    if let Decl::TypeDef { variants, .. } = &prog.decls[0] {
        assert_eq!(variants.len(), 2);
        assert!(
            variants[0].return_type.is_some(),
            "IntLit should have return_type"
        );
        assert!(
            variants[1].return_type.is_some(),
            "BoolLit should have return_type"
        );
    } else {
        panic!("expected TypeDef");
    }
}

#[test]
fn test_deep_nested_if_2000() {
    // 2000 段は従来の再帰版だと overflow する深さ
    let expr = parse_expr_str(&build_deep_if(2000));
    let _ = expr;
}
