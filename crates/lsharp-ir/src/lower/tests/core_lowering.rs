//! 基本 lowering の回帰

use super::*;

#[test]
fn test_lower_integer_literal() {
    assert_ir("(defn main [] 42)", "lower_integer_literal");
}

#[test]
fn test_lower_bool_literal() {
    assert_ir("(defn main [] true)", "lower_bool_literal");
}

#[test]
fn test_lower_arithmetic() {
    assert_ir("(defn main [] (+ (* 3 4) 5))", "lower_arithmetic");
}

#[test]
fn test_lower_comparison() {
    assert_ir("(defn main [] (< 1 2))", "lower_comparison");
}

#[test]
fn test_lower_if_expr() {
    assert_ir("(defn main [] (if (< 1 2) 42 0))", "lower_if_expr");
}

#[test]
fn test_lower_let_binding() {
    assert_ir(
        "(defn main [] (let [x 10 y 20] (+ x y)))",
        "lower_let_binding",
    );
}

#[test]
fn test_lower_nested_let() {
    assert_ir(
        "(defn main [] (let [a 5 b (+ a 3)] (* a b)))",
        "lower_nested_let",
    );
}

#[test]
fn test_lower_function_call() {
    assert_ir(
        "(defn double [x] (* x 2))
         (defn main [] (double 21))",
        "lower_function_call",
    );
}

#[test]
fn test_lower_write_file_bytes_uses_dedicated_instruction() {
    let module = lower("(defn main [] (write-file-bytes \"out.wasm\" (vector-new 4)))");
    let main = module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main function が存在するべき");

    assert!(
        main.body
            .iter()
            .any(|instruction| matches!(instruction, Instruction::WriteFileBytes)),
        "write-file-bytes は import index を増やさない専用 IR 命令へ lower するべき: {:?}",
        main.body
    );
}

#[test]
fn test_lower_function_prefers_ast_arity_when_inferred_name_collides() {
    let program = lsharp_syntax::parse("(defn make-if [cond then else] cond)").unwrap();
    let expr_type_results = HashMap::new();
    let type_results = vec![(
        "make-if".to_string(),
        TypeScheme::mono(Type::Fun(vec![], Box::new(Type::int()))),
    )];

    let mut lowerer = Lower::new();
    let module = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .unwrap();
    let func = module
        .functions
        .iter()
        .find(|func| func.name == "make-if")
        .unwrap();
    let total_locals = (func.params.len() + func.locals.len()) as u32;
    let used_locals: Vec<u32> = func
        .body
        .iter()
        .filter_map(|instr| match instr {
            Instruction::LocalGet(idx)
            | Instruction::LocalSet(idx)
            | Instruction::LocalTee(idx) => Some(*idx),
            _ => None,
        })
        .collect();

    assert_eq!(
        func.params.len(),
        3,
        "AST 側の引数数を優先しないと local index が壊れる: {func:?}"
    );
    assert!(
        used_locals.iter().all(|idx| *idx < total_locals),
        "local index が宣言数を超えている: total={total_locals}, used={used_locals:?}, func={func:?}"
    );
}

#[test]
fn test_lower_recursive_function() {
    assert_ir(
        "(defn fib [n]
           (if (<= n 1)
             n
             (+ (fib (- n 1)) (fib (- n 2)))))
         (defn main [] (fib 10))",
        "lower_recursive_function",
    );
}

#[test]
fn test_lower_print_call() {
    assert_ir("(defn main [] (print 42))", "lower_print_call");
}
