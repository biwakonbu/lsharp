//! lower モジュールのテスト

use super::*;
use crate::{Instruction, Module};
use lsharp_types::infer::Infer;

/// ソースコードから IR モジュールを生成するヘルパー
fn lower(source: &str) -> Module {
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    lowerer.lower_program(&program, &type_results).unwrap()
}

/// IR のテキストダンプをスナップショットテストで検証
fn assert_ir(source: &str, snapshot_name: &str) {
    let module = lower(source);
    insta::assert_snapshot!(snapshot_name, module.dump());
}

fn count_call_instr(body: &[Instruction], idx: u32) -> usize {
    body.iter()
        .filter(|instr| matches!(instr, Instruction::Call(call_idx) if *call_idx == idx))
        .count()
}

fn call_positions(body: &[Instruction], idx: u32) -> Vec<usize> {
    body.iter()
        .enumerate()
        .filter_map(|(i, instr)| {
            matches!(instr, Instruction::Call(call_idx) if *call_idx == idx).then_some(i)
        })
        .collect()
}

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

#[test]
fn test_lower_string_concat_auto_roots_arguments() {
    let module = lower(r#"(defn main [] (string-concat "a" "b"))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    let concat_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::Call(2)))
        .expect("string-concat は __string_concat を呼ぶべき");
    let pushes_before_concat = main_fn.body[..concat_pos]
        .iter()
        .filter(|instr| matches!(instr, Instruction::Call(14)))
        .count();
    let pops_after_concat = main_fn.body[concat_pos + 1..]
        .iter()
        .filter(|instr| matches!(instr, Instruction::Call(15)))
        .count();

    assert!(
        pushes_before_concat >= 2,
        "string-concat 前に 2 つの root_push が必要: {:?}",
        main_fn.body
    );
    assert!(
        pops_after_concat >= 2,
        "string-concat 後に 2 つの root_pop が必要: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_substring_auto_roots_source_string() {
    let module = lower(r#"(defn main [] (substring "abcd" 1 3))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert_eq!(
        count_call_instr(&main_fn.body, 14),
        1,
        "substring は source string を root_push で保護すべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        1,
        "substring は alloc 後に root_pop で解放すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_ref_new_auto_roots_wrapped_heap_value() {
    let module = lower(r#"(defn main [] (ref-new "ab"))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert!(
        count_call_instr(&main_fn.body, 14) >= 1,
        "ref-new は __alloc 前に wrapped value を root_push で保護すべき: {:?}",
        main_fn.body
    );
    assert!(
        count_call_instr(&main_fn.body, 15) >= 1,
        "ref-new は __alloc 後に root_pop で解放すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_vector_push_auto_roots_realloc_inputs() {
    let module = lower(r#"(defn main [] (vector-push (vector-new 0) "x"))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert!(
        count_call_instr(&main_fn.body, 14) >= 2,
        "vector-push の再割り当ては old vector / pushed value を root_push で保護すべき: {:?}",
        main_fn.body
    );
    assert!(
        count_call_instr(&main_fn.body, 15) >= 2,
        "vector-push の再割り当ては alloc 後に root_pop で解放すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_string_concat_roots_lhs_before_lowering_rhs() {
    let module = lower(r#"(defn main [] (string-concat "a" "b"))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let alloc_positions = call_positions(&main_fn.body, 1);
    let root_push_positions = call_positions(&main_fn.body, 14);

    assert!(
        alloc_positions.len() >= 2,
        "2 つの文字列リテラル割り当てが必要: {:?}",
        main_fn.body
    );
    assert!(
        !root_push_positions.is_empty(),
        "string-concat は root_push を使うべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[0] > alloc_positions[0] && root_push_positions[0] < alloc_positions[1],
        "lhs は rhs の割り当て前に root_push で保護すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_substring_roots_source_before_lowering_index_exprs() {
    let module = lower(r#"(defn main [] (substring "abcd" (string-length "xy") 2))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let alloc_positions = call_positions(&main_fn.body, 1);
    let root_push_positions = call_positions(&main_fn.body, 14);

    assert!(
        alloc_positions.len() >= 2,
        "source 文字列と index 側文字列の割り当てが必要: {:?}",
        main_fn.body
    );
    assert!(
        !root_push_positions.is_empty(),
        "substring は root_push を使うべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[0] > alloc_positions[0] && root_push_positions[0] < alloc_positions[1],
        "source string は start/end 式の割り当て前に root_push で保護すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_vector_push_roots_vector_before_lowering_value_expr() {
    let module = lower(r#"(defn main [] (vector-push (vector-new 0) "x"))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let alloc_positions = call_positions(&main_fn.body, 1);
    let root_push_positions = call_positions(&main_fn.body, 14);

    assert!(
        alloc_positions.len() >= 3,
        "vector-new / pushed string / realloc の 3 回割り当てが必要: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions.len() >= 2,
        "vector-push は vector と value を root_push で保護すべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[0] > alloc_positions[0] && root_push_positions[0] < alloc_positions[1],
        "receiver vector は value 式の割り当て前に root_push で保護すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_map_insert_roots_map_before_lowering_key_expr() {
    let module = lower(r#"(defn main [] (map-insert (map-new) (string-concat "a" "b") 1))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let alloc_positions = call_positions(&main_fn.body, 1);
    let root_push_positions = call_positions(&main_fn.body, 14);

    assert!(
        alloc_positions.len() >= 3,
        "map-new と key 側の文字列割り当てが必要: {:?}",
        main_fn.body
    );
    assert!(
        !root_push_positions.is_empty(),
        "map-insert は map receiver を root_push で保護すべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[0] > alloc_positions[0] && root_push_positions[0] < alloc_positions[1],
        "map receiver は key/value 式の割り当て前に root_push で保護すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_auto_roots_string_argument() {
    let module = lower(
        r#"
        (defn consume-string [s] (string-length s))
        (defn main [] (consume-string "hello"))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert!(
        count_call_instr(&main_fn.body, 14) >= 1,
        "string 引数の user call は root_push を使うべき: {:?}",
        main_fn.body
    );
    assert!(
        count_call_instr(&main_fn.body, 15) >= 1,
        "string 引数の user call は root_pop を使うべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_auto_roots_vector_argument() {
    let module = lower(
        r#"
        (defn vector-len [v] (vector-length v))
        (defn main [] (vector-len (vector-new 2)))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert!(
        count_call_instr(&main_fn.body, 14) >= 1,
        "vector 引数の user call は root_push を使うべき: {:?}",
        main_fn.body
    );
    assert!(
        count_call_instr(&main_fn.body, 15) >= 1,
        "vector 引数の user call は root_pop を使うべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_does_not_root_int_argument() {
    let module = lower(
        r#"
        (defn double [n] (+ n n))
        (defn main [] (double 21))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert_eq!(
        count_call_instr(&main_fn.body, 14),
        0,
        "Int 引数だけの user call は root_push を使わないべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        0,
        "Int 引数だけの user call は root_pop を使わないべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_selectively_roots_heap_arguments() {
    let module = lower(
        r#"
        (defn consume-both [s n] (+ (string-length s) n))
        (defn main [] (consume-both "hello" 21))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);

    assert_eq!(
        root_push_positions.len(),
        1,
        "heap 引数だけを 1 回 root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        1,
        "heap 引数だけを 1 回 root_pop するべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_self_recursive_user_call_preserves_self_tco() {
    let module = lower(
        r#"
        (defn append-loop [dst idx count]
          (if (>= idx count)
            dst
            (append-loop (vector-push dst idx) (+ idx 1) count)))
        (defn main [] 0)
        "#,
    );
    let append_loop = module
        .functions
        .iter()
        .find(|func| func.name == "append-loop")
        .unwrap();

    assert!(
        append_loop
            .body
            .iter()
            .any(|instr| matches!(instr, Instruction::Loop(_) | Instruction::LoopEmpty)),
        "自己末尾再帰は Loop に変換されるべき: {:?}",
        append_loop.body
    );
    assert_eq!(
        count_call_instr(&append_loop.body, 17),
        0,
        "自己末尾再帰は direct Call のまま残らないべき: {:?}",
        append_loop.body
    );
}

#[test]
fn test_lower_wildcard_let() {
    assert_ir("(defn main [] (let [_ 99] 1))", "lower_wildcard_let");
}

#[test]
fn test_lower_do_block() {
    assert_ir(
        "(defn main [] (do (print 1) (print 2) 42))",
        "lower_do_block",
    );
}

#[test]
fn test_lower_not_operator() {
    assert_ir("(defn main [] (not true))", "lower_not_operator");
}

#[test]
fn test_lower_undefined_variable_error() {
    use lsharp_syntax::ast::*;
    use lsharp_syntax::span::Span;

    let s = Span { start: 0, end: 0 };
    let program = Program {
        decls: vec![Decl::Defn {
            span: s,
            name: "main".to_string(),
            params: vec![],
            return_ty: None,
            body: Expr::Var(s, "undefined_var".to_string()),
            where_clauses: Vec::new(),
            metadata: None,
        }],
    };
    let mut lowerer = Lower::new();
    let result = lowerer.lower_program(&program, &[]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, LowerError::UndefinedFunction { .. }),
        "expected UndefinedFunction error, got: {err}"
    );
}

#[test]
fn test_emit_binop_unknown_operator_returns_error() {
    // 未知の二項演算子でエラーが返ることを確認 (R-M2)
    let mut lowerer = Lower::new();
    let mut ctx = FuncCtx::new("test".to_string());
    let result = lowerer.emit_binop(&mut ctx, "unknown_op");
    assert!(
        result.is_err(),
        "未知の演算子 'unknown_op' でエラーが返るべき"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, LowerError::Unsupported { .. }),
        "Unsupported エラーが返るべき: {err}"
    );
}

#[test]
fn test_emit_binop_known_operators_succeed() {
    // 既知の演算子は全て成功すること (R-M2 回帰テスト)
    let mut lowerer = Lower::new();
    let known_ops = [
        "+", "-", "*", "/", "%", "+.", "-.", "*.", "/.", "==", "=", "!=", "<", ">", "<=", ">=",
        "and", "or",
    ];
    for op in &known_ops {
        let mut ctx = FuncCtx::new("test".to_string());
        let result = lowerer.emit_binop(&mut ctx, op);
        assert!(result.is_ok(), "演算子 '{op}' は成功すべき");
    }
}

#[test]
fn test_lower_computation_return_calls_return_fn() {
    // computation の return が return_fn を呼び出すこと
    // (computation-builder name bind-fn return-fn)
    let source = r#"
        (computation-builder maybe mb mr)
        (defn mb [m x] m)
        (defn mr [x] x)
        (defn main [] (computation maybe (return 42)))
    "#;
    let module = lower(source);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    // mr (return_fn) への Call 命令が含まれるべき
    let has_call = main_fn
        .body
        .iter()
        .any(|instr| matches!(instr, Instruction::Call(_)));
    assert!(
        has_call,
        "return は return_fn を Call すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_computation_let_bang_binds_variable() {
    // computation の let! が変数をローカルに格納すること
    let source = r#"
        (computation-builder maybe identity identity)
        (defn identity [x] x)
        (defn main []
            (computation maybe
                (let! x 10)
                (return (+ x 1))))
    "#;
    let module = lower(source);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    // LocalSet + LocalGet の組み合わせがあるべき
    let has_local_set = main_fn
        .body
        .iter()
        .any(|instr| matches!(instr, Instruction::LocalSet(_)));
    assert!(
        has_local_set,
        "let! はローカル変数に格納すべき: {:?}",
        main_fn.body
    );
}

// --- private テスト ---

#[test]
fn test_lower_private_defn() {
    // private 内の関数も正しく IR 変換される
    let program =
        lsharp_syntax::parse("(private (defn helper [x] (+ x 1))) (defn main [] (helper 42))")
            .unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    // helper と main の2つの関数が生成される
    assert_eq!(module.functions.len(), 2);
    assert_eq!(module.functions[0].name, "helper");
    assert_eq!(module.functions[1].name, "main");
}

#[test]
fn test_lower_private_record() {
    // private 内のレコード型も正しく IR 変換される
    let program = lsharp_syntax::parse(
        "(private (type Point (record (: x Int) (: y Int)))) (defn main [] 42)",
    )
    .unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    // GC 型として Point が登録されている
    assert_eq!(module.gc_types.len(), 1);
    assert_eq!(module.gc_types[0].name, "Point");
}

// --- トレイト実装テスト ---

#[test]
fn test_lower_trait_impl_methods() {
    // トレイト定義 + 実装で、impl メソッドが IR 関数として生成される
    let source = r#"
        (trait (Show a)
          (defn show [x] 0))
        (impl (Show Int)
          (defn show [x] (+ x 1)))
        (defn main [] 42)
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    // main + Show_Int_show の 2 関数が生成される
    assert_eq!(module.functions.len(), 2);
    assert_eq!(module.functions[0].name, "main");
    assert_eq!(module.functions[1].name, "Show_Int_show");
}

#[test]
fn test_lower_multiple_trait_impls() {
    // 複数型への impl
    let source = r#"
        (trait (Eq a)
          (defn eq? [x y] (== x y)))
        (impl (Eq Int)
          (defn eq? [x y] (== x y)))
        (impl (Eq Bool)
          (defn eq? [x y] (== x y)))
        (defn main [] 0)
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    // main + Eq_Int_eq? + Eq_Bool_eq? = 3
    assert_eq!(module.functions.len(), 3);

    // マングル名を確認
    let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"Eq_Int_eq?"));
    assert!(names.contains(&"Eq_Bool_eq?"));
}

#[test]
fn test_trait_method_impl_resolution() {
    // trait_method_impls テーブルが正しく構築される
    let source = r#"
        (trait (Show a)
          (defn show [x] 0))
        (impl (Show Int)
          (defn show [x] (+ x 1)))
        (defn main [] 42)
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let _module = lowerer.lower_program(&program, &type_results).unwrap();

    // 解決テーブルに (Show, Int, show) -> Show_Int_show が登録されている
    let key = ("Show".to_string(), "Int".to_string(), "show".to_string());
    assert_eq!(
        lowerer.trait_method_impls.get(&key),
        Some(&"Show_Int_show".to_string())
    );
}

#[test]
fn test_static_dispatch_with_literal_arg() {
    // トレイトメソッド呼び出しがリテラル引数から自動解決される
    let source = r#"
        (trait (Show a)
          (defn show [x] 0))
        (impl (Show Int)
          (defn show [x] (+ x 1)))
        (defn main [] (show 42))
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    // main 関数に Call 命令が含まれる（Show_Int_show への呼び出し）
    let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
    let has_call = main_func
        .body
        .iter()
        .any(|i| matches!(i, Instruction::Call(_)));
    assert!(
        has_call,
        "main 関数にトレイトメソッド呼び出し（Call）が含まれるべき: {:?}",
        main_func.body
    );
}

#[test]
fn test_static_dispatch_unique_impl() {
    // 実装が1つだけの場合、型が不明でも一意解決される
    let source = r#"
        (trait (Show a)
          (defn show [x] 0))
        (impl (Show Int)
          (defn show [x] (+ x 1)))
        (defn use-show [x] (show x))
        (defn main [] 0)
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    // use-show が正常にコンパイルされる（一意解決）
    let use_show = module
        .functions
        .iter()
        .find(|f| f.name == "use-show")
        .unwrap();
    let has_call = use_show
        .body
        .iter()
        .any(|i| matches!(i, Instruction::Call(_)));
    assert!(
        has_call,
        "use-show にトレイトメソッド呼び出し（Call）が含まれるべき: {:?}",
        use_show.body
    );
}

#[test]
fn test_lower_trait_method_auto_roots_string_argument() {
    let module = lower(
        r#"
        (trait (Show a)
          (defn show [x] 0))
        (impl (Show String)
          (defn show [x] (string-length x)))
        (defn main [] (show "hello"))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert!(
        count_call_instr(&main_fn.body, 14) >= 1,
        "string 引数の trait dispatch は root_push を使うべき: {:?}",
        main_fn.body
    );
    assert!(
        count_call_instr(&main_fn.body, 15) >= 1,
        "string 引数の trait dispatch は root_pop を使うべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_trait_method_does_not_root_int_argument() {
    let module = lower(
        r#"
        (trait (Show a)
          (defn show [x] 0))
        (impl (Show Int)
          (defn show [x] (+ x 1)))
        (defn main [] (show 42))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert_eq!(
        count_call_instr(&main_fn.body, 14),
        0,
        "Int 引数の trait dispatch は root_push を使わないべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        0,
        "Int 引数の trait dispatch は root_pop を使わないべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_trait_method_selectively_roots_heap_arguments() {
    let module = lower(
        r#"
        (trait (Measure a)
          (defn measure [x n] 0))
        (impl (Measure String)
          (defn measure [x n] (+ (string-length x) n)))
        (defn main [] (measure "hello" 21))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);

    assert_eq!(
        root_push_positions.len(),
        1,
        "heap 引数だけを 1 回 root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        1,
        "heap 引数だけを 1 回 root_pop するべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_trait_method_names_table() {
    // trait_method_names テーブルが正しく構築される
    let source = r#"
        (trait (Show a)
          (defn show [x] 0))
        (trait (Eq a)
          (defn eq? [x y] (== x y)))
        (defn main [] 0)
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let _module = lowerer.lower_program(&program, &type_results).unwrap();

    assert_eq!(
        lowerer.trait_method_names.get("show"),
        Some(&vec!["Show".to_string()])
    );
    assert_eq!(
        lowerer.trait_method_names.get("eq?"),
        Some(&vec!["Eq".to_string()])
    );
}

// --- 制約チェックテスト ---

#[test]
fn test_lower_constrained_type_generates_new() {
    let source = r#"
        (type-constrained Natural Int :constraints [(>= 0)])
        (defn main [] 42)
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    // main + Natural.new + Natural.valid? = 3 関数
    let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"Natural.new"),
        "Natural.new が生成されていない: {:?}",
        names
    );
    assert!(
        names.contains(&"Natural.valid?"),
        "Natural.valid? が生成されていない: {:?}",
        names
    );
}

#[test]
fn test_constraint_check_gte_instructions() {
    let source = r#"
        (type-constrained Natural Int :constraints [(>= 0)])
        (defn main [] 42)
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    let new_func = module
        .functions
        .iter()
        .find(|f| f.name == "Natural.new")
        .unwrap();
    // 最後の命令は LocalGet(0) (値をそのまま返す)
    assert!(matches!(
        new_func.body.last(),
        Some(Instruction::LocalGet(0))
    ));
    // Unreachable が含まれている (制約違反時のトラップ)
    assert!(
        new_func
            .body
            .iter()
            .any(|i| matches!(i, Instruction::Unreachable)),
        "Natural.new に Unreachable が含まれていない"
    );
}

#[test]
fn test_constraint_valid_returns_bool() {
    let source = r#"
        (type-constrained Natural Int :constraints [(>= 0)])
        (defn main [] 42)
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    let valid_func = module
        .functions
        .iter()
        .find(|f| f.name == "Natural.valid?")
        .unwrap();
    // 最初の命令は I64Const(1) (true で初期化)
    assert!(matches!(
        valid_func.body.first(),
        Some(Instruction::I64Const(1))
    ));
    // Unreachable は含まれない (valid? はトラップしない)
    assert!(
        !valid_func
            .body
            .iter()
            .any(|i| matches!(i, Instruction::Unreachable)),
        "Natural.valid? に Unreachable が含まれてはいけない"
    );
}

#[test]
fn test_constraint_range_generates_both_checks() {
    let source = r#"
        (type-constrained Port Int :constraints [(range 1 65535)])
        (defn main [] 42)
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    let new_func = module
        .functions
        .iter()
        .find(|f| f.name == "Port.new")
        .unwrap();
    // range は 2 つの Unreachable を生成 (下限チェック + 上限チェック)
    let unreachable_count = new_func
        .body
        .iter()
        .filter(|i| matches!(i, Instruction::Unreachable))
        .count();
    assert_eq!(
        unreachable_count, 2,
        "Range 制約は 2 つのチェックを生成する"
    );
}

// --- レコードパターンテスト ---

#[test]
fn test_record_pattern_uses_struct_get() {
    let source = r#"
        (type Point (record (: x Int) (: y Int)))
        (defn get-x [p]
          (match p
            [{Point x px y py} px]))
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    let get_x = module.functions.iter().find(|f| f.name == "get-x").unwrap();
    // StructGet 命令が生成されていることを確認
    let struct_gets: Vec<_> = get_x
        .body
        .iter()
        .filter(|i| matches!(i, Instruction::StructGet(_, _)))
        .collect();
    assert!(
        !struct_gets.is_empty(),
        "レコードパターンは StructGet を使用すべき: {:?}",
        get_x.body
    );
}

#[test]
fn test_resolve_field_index() {
    let mut lowerer = Lower::new();
    lowerer
        .record_fields
        .insert("Point".to_string(), vec!["x".to_string(), "y".to_string()]);
    assert_eq!(lowerer.resolve_field_index("Point", "x"), Some(0));
    assert_eq!(lowerer.resolve_field_index("Point", "y"), Some(1));
    assert_eq!(lowerer.resolve_field_index("Point", "z"), None);
    assert_eq!(lowerer.resolve_field_index("Unknown", "x"), None);
}

#[test]
fn test_field_access_resolves_correct_type() {
    // R-M5: FieldAccess に型推論結果から正しい型名で解決することを検証
    // 直接 AST を構築（パーサーは FieldAccess を生成しないため）
    let mut lowerer = Lower::new();
    lowerer
        .record_fields
        .insert("Point".to_string(), vec!["x".to_string(), "y".to_string()]);
    lowerer
        .record_fields
        .insert("Size".to_string(), vec!["x".to_string(), "h".to_string()]);
    lowerer.record_type_indices.insert("Point".to_string(), 0);
    lowerer.record_type_indices.insert("Size".to_string(), 1);
    // Point の x フィールドはインデックス 0
    assert_eq!(lowerer.resolve_field_index("Point", "x"), Some(0));
    // Size の x フィールドもインデックス 0 だが、異なる GC 型
    assert_eq!(lowerer.resolve_field_index("Size", "x"), Some(0));
}

#[test]
fn test_field_access_error_on_unknown_field() {
    // R-M5: 解決失敗時にエラーを返すことを検証
    let mut lowerer = Lower::new();
    lowerer
        .record_fields
        .insert("Point".to_string(), vec!["x".to_string(), "y".to_string()]);
    lowerer.record_type_indices.insert("Point".to_string(), 0);
    // 存在しないフィールドは None
    assert_eq!(lowerer.resolve_field_index("Point", "z"), None);
}

#[test]
fn test_field_accessor_function_uses_struct_get() {
    // R-M5: フィールドアクセサ関数が正しい StructGet を生成することを検証
    let source = r#"
        (type Point (record (: x Int) (: y Int)))
        (type Size (record (: x Int) (: h Int)))
        (defn main [] 42)
    "#;
    let module = lower(source);
    // Point.x アクセサ関数が正しい GC 型インデックスとフィールドインデックスで StructGet を使用
    let point_x = module
        .functions
        .iter()
        .find(|f| f.name == "Point.x")
        .unwrap();
    let has_struct_get = point_x
        .body
        .iter()
        .any(|i| matches!(i, Instruction::StructGet(0, 0)));
    assert!(
        has_struct_get,
        "Point.x は StructGet(0, 0) を使用すべき: {:?}",
        point_x.body
    );
    // Size.x は異なる GC 型インデックスで StructGet を使用
    let size_x = module
        .functions
        .iter()
        .find(|f| f.name == "Size.x")
        .unwrap();
    let has_struct_get = size_x
        .body
        .iter()
        .any(|i| matches!(i, Instruction::StructGet(1, 0)));
    assert!(
        has_struct_get,
        "Size.x は StructGet(1, 0) を使用すべき: {:?}",
        size_x.body
    );
}

#[test]
fn test_constructor_pattern_emits_tag_comparison() {
    // R-m9: 引数なしコンストラクタパターンでタグ比較命令が発行されることを検証
    let source = r#"
        (type Color
          Red
          Green
          Blue)
        (defn color-to-int [c]
          (match c
            [Red 0]
            [Green 1]
            [Blue 2]))
    "#;
    let module = lower(source);
    let func = module
        .functions
        .iter()
        .find(|f| f.name == "color-to-int")
        .unwrap();
    // タグ比較のために I64Eq が使用されるべき
    let has_eq = func.body.iter().any(|i| matches!(i, Instruction::I64Eq));
    assert!(
        has_eq,
        "コンストラクタパターンはタグ比較 (I64Eq) を使用すべき: {:?}",
        func.body
    );
}

#[test]
fn test_record_update_resolves_correct_type() {
    // R-m3: RecordUpdate がベース式から正しい型を推定することを検証
    let source = r#"
        (type Point (record (: x Int) (: y Int)))
        (defn make-point [] {Point x 1 y 2})
        (defn move-x []
          (let [p (make-point)]
            {p | x 10}))
    "#;
    let module = lower(source);
    let move_x = module
        .functions
        .iter()
        .find(|f| f.name == "move-x")
        .unwrap();
    // StructNew が発行されることを確認（更新されたレコードを構築）
    let has_struct_new = move_x
        .body
        .iter()
        .any(|i| matches!(i, Instruction::StructNew(_)));
    assert!(
        has_struct_new,
        "RecordUpdate は StructNew を生成すべき: {:?}",
        move_x.body
    );
}

// --- タグ付きワード (TASK-004) テスト ---

#[test]
fn test_emit_tag_pointer_generates_correct_instructions() {
    // タグ付きポインタ変換: i32 アドレス → タグ付き i64
    // スタック上の i32 を i64 に拡張して最上位ビットを立てる
    let mut body = Vec::new();
    emit_tag_pointer(&mut body, 0);
    assert_eq!(body.len(), 3, "emit_tag_pointer は 3 命令を生成すべき");
    assert!(matches!(body[0], Instruction::I64ExtendI32U));
    assert!(matches!(body[1], Instruction::I64Const(v) if v == (1i64 << 63)));
    assert!(matches!(body[2], Instruction::I64Add));
}

#[test]
fn test_emit_untag_pointer_generates_correct_instructions() {
    // アンタグ変換: タグ付き i64 → i32 アドレス
    let mut body = Vec::new();
    emit_untag_pointer(&mut body);
    assert_eq!(body.len(), 1, "emit_untag_pointer は 1 命令を生成すべき");
    assert!(matches!(body[0], Instruction::I32WrapI64));
}

#[test]
fn test_emit_write_heap_header_generates_correct_instructions() {
    // ヒープヘッダ書き込み: [tag: i32, size: i32]
    // スタック: [addr] -> [] (addr は消費される、呼び出し側で保存が必要)
    let mut body = Vec::new();
    emit_write_heap_header(&mut body, 1, 16);
    // addr, tag を store + addr, size を store = 4 命令
    assert_eq!(
        body.len(),
        4,
        "emit_write_heap_header は 4 命令を生成すべき: {:?}",
        body
    );
    // tag 書き込み: I32Const(tag), I32Store { offset: 0 }
    assert!(matches!(body[0], Instruction::I32Const(1)));
    assert!(matches!(body[1], Instruction::I32Store { offset: 0 }));
    // size 書き込み: I32Const(size), I32Store { offset: 4 }
    assert!(matches!(body[2], Instruction::I32Const(16)));
    assert!(matches!(body[3], Instruction::I32Store { offset: 4 }));
}

#[test]
fn test_heap_object_tag_constants() {
    // ヒープオブジェクトタグ定数が正しく定義されていること
    assert_eq!(HEAP_TAG_STRING, 1);
    assert_eq!(HEAP_TAG_RECORD, 2);
    assert_eq!(HEAP_TAG_ADT, 3);
    assert_eq!(HEAP_TAG_CLOSURE, 4);
    assert_eq!(HEAP_TAG_VECTOR, 5);
    assert_eq!(HEAP_TAG_HASHMAP, 6);
    assert_eq!(HEAP_TAG_REF, 7);
}

#[test]
fn test_tagged_pointer_roundtrip() {
    // タグ付きポインタの往復変換が正しい命令列を生成すること
    let mut body = Vec::new();
    // タグ付け
    emit_tag_pointer(&mut body, 0);
    // アンタグ
    emit_untag_pointer(&mut body);
    // 合計 4 命令 (3 + 1)
    assert_eq!(body.len(), 4);
}

// --- ADT リニアメモリ版テスト ---

#[test]
fn test_adt_constructor_linear_memory_uses_alloc() {
    // ADT コンストラクタがリニアメモリ版で __alloc を呼び出すことを検証
    let source = r#"
        (type (Maybe a) (Just a) Nothing)
        (defn main [] (Just 42))
    "#;
    let module = lower(source);
    let just_fn = module.functions.iter().find(|f| f.name == "Just").unwrap();
    // __alloc への Call 命令が含まれるべき (func_idx = 1)
    let has_alloc_call = just_fn
        .body
        .iter()
        .any(|i| matches!(i, Instruction::Call(1)));
    assert!(
        has_alloc_call,
        "Just コンストラクタは __alloc を呼び出すべき: {:?}",
        just_fn.body
    );
}

#[test]
fn test_adt_constructor_linear_memory_writes_heap_tag() {
    // ADT コンストラクタがヒープタグ (tag=3) を書き込むことを検証
    let source = r#"
        (type (Maybe a) (Just a) Nothing)
        (defn main [] 0)
    "#;
    let module = lower(source);
    let just_fn = module.functions.iter().find(|f| f.name == "Just").unwrap();
    // I32Const(3) (HEAP_TAG_ADT) が含まれるべき
    let has_heap_tag = just_fn
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I32Const(3)));
    assert!(
        has_heap_tag,
        "Just コンストラクタはヒープタグ 3 を書き込むべき: {:?}",
        just_fn.body
    );
}

#[test]
fn test_adt_constructor_linear_memory_writes_variant_tag() {
    // ADT コンストラクタがバリアントタグを書き込むことを検証
    let source = r#"
        (type (Maybe a) (Just a) Nothing)
        (defn main [] 0)
    "#;
    let module = lower(source);
    let just_fn = module.functions.iter().find(|f| f.name == "Just").unwrap();
    // I32Store でバリアントタグを書き込む命令が含まれるべき
    let has_variant_store = just_fn
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I32Store { offset: 4 }));
    assert!(
        has_variant_store,
        "Just コンストラクタはバリアントタグを offset 4 に書き込むべき: {:?}",
        just_fn.body
    );
}

#[test]
fn test_adt_constructor_linear_memory_stores_field() {
    // ADT コンストラクタがフィールド値をメモリに書き込むことを検証
    let source = r#"
        (type (Maybe a) (Just a) Nothing)
        (defn main [] 0)
    "#;
    let module = lower(source);
    let just_fn = module.functions.iter().find(|f| f.name == "Just").unwrap();
    // フィールドは offset 8 に I64Store で書き込まれるべき
    let has_field_store = just_fn
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I64Store { offset: 8 }));
    assert!(
        has_field_store,
        "Just コンストラクタはフィールドを offset 8 に書き込むべき: {:?}",
        just_fn.body
    );
}

#[test]
fn test_adt_constructor_linear_memory_returns_tagged_pointer() {
    // ADT コンストラクタがタグ付きポインタを返すことを検証
    let source = r#"
        (type (Maybe a) (Just a) Nothing)
        (defn main [] 0)
    "#;
    let module = lower(source);
    let just_fn = module.functions.iter().find(|f| f.name == "Just").unwrap();
    // タグ付きポインタ: I64ExtendI32U + I64Const(1<<63) + I64Add
    let has_tag_pointer = just_fn
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I64ExtendI32U));
    assert!(
        has_tag_pointer,
        "Just コンストラクタはタグ付きポインタを返すべき: {:?}",
        just_fn.body
    );
}

#[test]
fn test_adt_constructor_no_args_linear_memory() {
    // 引数なしコンストラクタもヒープ確保してタグ付きポインタを返すことを検証
    let source = r#"
        (type (Maybe a) (Just a) Nothing)
        (defn main [] 0)
    "#;
    let module = lower(source);
    let nothing_fn = module
        .functions
        .iter()
        .find(|f| f.name == "Nothing")
        .unwrap();
    // __alloc への Call が含まれるべき
    let has_alloc_call = nothing_fn
        .body
        .iter()
        .any(|i| matches!(i, Instruction::Call(1)));
    assert!(
        has_alloc_call,
        "Nothing コンストラクタも __alloc を呼び出すべき: {:?}",
        nothing_fn.body
    );
}

// --- ADT パターンマッチ リニアメモリ版テスト ---

#[test]
fn test_adt_pattern_match_no_args_reads_variant_tag_from_memory() {
    // 引数なしコンストラクタのパターンマッチが、メモリから variant_tag を読み出して比較することを検証
    let source = r#"
        (type Color Red Green Blue)
        (defn color-to-int [c]
          (match c
            [Red 0]
            [Green 1]
            [Blue 2]))
        (defn main [] 0)
    "#;
    let module = lower(source);
    let func = module
        .functions
        .iter()
        .find(|f| f.name == "color-to-int")
        .unwrap();
    // タグ付きポインタからアドレスを取り出す I32WrapI64 が含まれるべき
    let has_untag = func
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I32WrapI64));
    assert!(
        has_untag,
        "コンストラクタパターンはポインタをアンタグすべき: {:?}",
        func.body
    );
    // variant_tag を読み出す I32Load { offset: 4 } が含まれるべき
    let has_tag_load = func
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I32Load { offset: 4 }));
    assert!(
        has_tag_load,
        "コンストラクタパターンは variant_tag を I32Load(offset:4) で読むべき: {:?}",
        func.body
    );
}

#[test]
fn test_adt_pattern_match_with_args_extracts_fields() {
    // 引数付きコンストラクタのパターンマッチがフィールドを I64Load で取り出すことを検証
    let source = r#"
        (type (Maybe a) (Just a) Nothing)
        (defn from-maybe [m d]
          (match m
            [(Just x) x]
            [Nothing d]))
        (defn main [] 0)
    "#;
    let module = lower(source);
    let func = module
        .functions
        .iter()
        .find(|f| f.name == "from-maybe")
        .unwrap();
    // フィールド取り出しの I64Load { offset: 8 } が含まれるべき
    let has_field_load = func
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I64Load { offset: 8 }));
    assert!(
        has_field_load,
        "引数付きコンストラクタパターンはフィールドを I64Load(offset:8) で取り出すべき: {:?}",
        func.body
    );
}

#[test]
fn test_adt_pattern_match_with_args_compares_variant_tag() {
    // 引数付きコンストラクタのパターンマッチが variant_tag で分岐することを検証
    let source = r#"
        (type (Maybe a) (Just a) Nothing)
        (defn from-maybe [m d]
          (match m
            [(Just x) x]
            [Nothing d]))
        (defn main [] 0)
    "#;
    let module = lower(source);
    let func = module
        .functions
        .iter()
        .find(|f| f.name == "from-maybe")
        .unwrap();
    // variant_tag を読み出す I32Load が含まれるべき
    let has_tag_load = func
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I32Load { offset: 4 }));
    assert!(
        has_tag_load,
        "引数付きコンストラクタパターンは variant_tag で分岐すべき: {:?}",
        func.body
    );
    // I64ExtendI32U + I64Eq でタグ比較すべき
    let has_eq = func.body.iter().any(|i| matches!(i, Instruction::I64Eq));
    assert!(
        has_eq,
        "パターンマッチはタグ値を比較すべき: {:?}",
        func.body
    );
}

// --- モジュール分割検証テスト ---

#[test]
fn test_lower_module_structure() {
    // Lower::new() と lower_program() が正しく動作することを検証
    // 簡単なプログラム（整数リテラル）の IR 変換
    let source = "(defn main [] 42)";
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    // main 関数が1つ生成される
    assert_eq!(module.functions.len(), 1);
    assert_eq!(module.functions[0].name, "main");
    assert!(
        module.functions[0].is_export,
        "main 関数は export されるべき"
    );
    // 本体は I64Const(42) の1命令
    assert_eq!(module.functions[0].body.len(), 1);
    assert!(matches!(
        module.functions[0].body[0],
        Instruction::I64Const(42)
    ));
}

// --- Ref Cell テスト ---

#[test]
fn test_ref_new_generates_alloc_and_store() {
    // ref-new はヒープ確保 + ヘッダ書き込み + 値書き込みを行うべき
    let source = r#"
        (defn main [] (ref-new 42))
    "#;
    let module = lower(source);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    // __alloc 呼び出しが含まれるべき
    let has_alloc_call = main_fn.body.iter().any(|i| {
        matches!(i, Instruction::Call(idx) if *idx == 1) // __alloc のインデックス
    });
    assert!(
        has_alloc_call,
        "ref-new は __alloc を呼び出すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_ref_get_generates_load() {
    // ref-get はポインタアンタグ + I64Load を行うべき
    let source = r#"
        (defn main [] (let [r (ref-new 42)] (ref-get r)))
    "#;
    let module = lower(source);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    // I64Load が含まれるべき (ヒープからの値読み出し)
    let has_load = main_fn
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I64Load { .. }));
    assert!(
        has_load,
        "ref-get は I64Load を含むべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_ref_set_generates_store() {
    // ref-set はポインタアンタグ + I64Store を行うべき
    let source = r#"
        (defn main [] (let [r (ref-new 42)] (do (ref-set r 100) 0)))
    "#;
    let module = lower(source);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    // I64Store が含まれるべき (ヒープへの値書き込み)
    let has_store = main_fn
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I64Store { .. }));
    assert!(
        has_store,
        "ref-set は I64Store を含むべき: {:?}",
        main_fn.body
    );
}

// --- Lambda Lifting テスト ---

#[test]
fn test_lambda_no_free_vars_lifted() {
    // 自由変数なし Lambda がリフトされた関数として生成される
    let source = r#"
        (defn make-inc [] (fn [x] (+ x 1)))
        (defn main [] 0)
    "#;
    let module = lower(source);
    // リフトされた Lambda 関数が生成されているべき
    let lifted = module
        .functions
        .iter()
        .find(|f| f.name.starts_with("__lambda"));
    assert!(
        lifted.is_some(),
        "Lambda はリフトされた関数として生成されるべき: {:?}",
        module.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    // 統一呼び出し規約: 元パラメータ + closure_ptr で 2 つ
    let lifted = lifted.unwrap();
    assert_eq!(
        lifted.params.len(),
        2,
        "統一呼び出し規約: 元パラメータ 1 + closure_ptr 1 = 2"
    );
}

#[test]
fn test_lambda_with_free_vars_captures() {
    // 自由変数あり Lambda: 統一呼び出し規約 (元パラメータ + closure_ptr)
    // 自由変数は closure_ptr から読み出す
    let source = r#"
        (defn make-adder [n] (fn [x] (+ x n)))
        (defn main [] 0)
    "#;
    let module = lower(source);
    let lifted = module
        .functions
        .iter()
        .find(|f| f.name.starts_with("__lambda"));
    assert!(
        lifted.is_some(),
        "Lambda はリフトされた関数として生成されるべき: {:?}",
        module.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
    // 統一呼び出し規約: 元パラメータ x + closure_ptr = 2
    let lifted = lifted.unwrap();
    assert_eq!(
        lifted.params.len(),
        2,
        "統一呼び出し規約: 元パラメータ 1 + closure_ptr 1 = 2"
    );
    // 本体に I64Load が含まれる (closure_ptr からキャプチャ値 n を読み出す)
    let has_load = lifted
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I64Load { .. }));
    assert!(
        has_load,
        "自由変数 n は closure_ptr から I64Load で読み出すべき: {:?}",
        lifted.body
    );
}

#[test]
fn test_lambda_lifting_preserves_existing_tests() {
    // 既存の関数定義が Lambda Lifting の影響を受けないことを確認
    let source = "(defn double [x] (* x 2)) (defn main [] (double 21))";
    let module = lower(source);
    // double と main 以外にリフトされた関数はないべき
    let non_lifted: Vec<_> = module
        .functions
        .iter()
        .filter(|f| !f.name.starts_with("__lambda"))
        .collect();
    assert_eq!(non_lifted.len(), 2);
    assert_eq!(non_lifted[0].name, "double");
    assert_eq!(non_lifted[1].name, "main");
}

#[test]
fn test_lambda_body_has_correct_instructions() {
    // リフトされた Lambda の本体に正しい命令列が含まれる
    let source = r#"
        (defn make-inc [] (fn [x] (+ x 1)))
        (defn main [] 0)
    "#;
    let module = lower(source);
    let lifted = module
        .functions
        .iter()
        .find(|f| f.name.starts_with("__lambda"))
        .unwrap();
    // (+ x 1) の命令: LocalGet(0), I64Const(1), I64Add
    let has_add = lifted.body.iter().any(|i| matches!(i, Instruction::I64Add));
    assert!(
        has_add,
        "Lambda 本体に I64Add が含まれるべき: {:?}",
        lifted.body
    );
}

#[test]
fn test_lambda_unsupported_test_removed() {
    // 以前の「Lambda は未サポート」テストは Lambda Lifting 実装によりパスしないことを確認
    // Lambda が正常にコンパイルされることを検証
    let program = lsharp_syntax::parse("(defn main [] (fn [x] x))").unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let result = lowerer.lower_program(&program, &type_results);
    assert!(
        result.is_ok(),
        "Lambda は Lambda Lifting で正常にコンパイルされるべき"
    );
}

#[test]
fn test_closure_call_indirect_ir() {
    // クロージャ呼び出しが CallIndirect を生成することを確認
    let source = r#"
        (defn make-inc [] (fn [x] (+ x 1)))
        (defn apply [f x] (f x))
        (defn main [] (print (apply (make-inc) 41)))
    "#;
    let module = lower(source);
    let apply_fn = module.functions.iter().find(|f| f.name == "apply").unwrap();
    eprintln!("apply IR: {:?}", apply_fn.body);
    // apply 関数に CallIndirect が含まれるべき
    let has_call_indirect = apply_fn
        .body
        .iter()
        .any(|i| matches!(i, Instruction::CallIndirect(_)));
    assert!(
        has_call_indirect,
        "apply 関数に CallIndirect が含まれるべき: {:?}",
        apply_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_closure_receiver() {
    let module = lower(
        r#"
        (defn make-inc [] (fn [x] (+ x 1)))
        (defn apply [f x] (f x))
        (defn main [] (print (apply (make-inc) 41)))
        "#,
    );
    let apply_fn = module.functions.iter().find(|f| f.name == "apply").unwrap();
    let root_push_positions = call_positions(&apply_fn.body, 14);
    let call_indirect_pos = apply_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        1,
        "closure call は receiver だけ 1 回 root_push するべき: {:?}",
        apply_fn.body
    );
    assert_eq!(
        count_call_instr(&apply_fn.body, 15),
        1,
        "closure call は receiver に対応する root_pop を 1 回使うべき: {:?}",
        apply_fn.body
    );
    assert!(
        root_push_positions[0] < call_indirect_pos,
        "closure receiver は call_indirect 前に root_push で保護されるべき: {:?}",
        apply_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_receiver_before_string_arg_eval() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn apply [f s] (f s))
        (defn main [] (print (apply (make-show) "hello")))
        "#,
    );
    let apply_fn = module.functions.iter().find(|f| f.name == "apply").unwrap();
    let root_push_positions = call_positions(&apply_fn.body, 14);
    let string_arg_get_pos = apply_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::LocalGet(idx) if *idx == 1))
        .unwrap();

    assert!(
        !root_push_positions.is_empty(),
        "closure call は receiver を root_push で保護すべき: {:?}",
        apply_fn.body
    );
    assert!(
        root_push_positions[0] < string_arg_get_pos,
        "closure receiver は string 引数の評価より前に root_push で保護されるべき: {:?}",
        apply_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_annotated_string_param_argument() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn apply [f (: s String)] (f s))
        (defn main [] (print (apply (make-show) "hello")))
        "#,
    );
    let apply_fn = module.functions.iter().find(|f| f.name == "apply").unwrap();
    let root_push_positions = call_positions(&apply_fn.body, 14);
    let call_indirect_pos = apply_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "closure call は receiver と注釈付き string 引数を root_push するべき: {:?}",
        apply_fn.body
    );
    assert_eq!(
        count_call_instr(&apply_fn.body, 15),
        2,
        "closure call は receiver と注釈付き string 引数に対応する root_pop を使うべき: {:?}",
        apply_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "注釈付き string 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        apply_fn.body
    );
}

#[test]
fn test_lower_closure_call_roots_let_bound_string_argument() {
    let module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn main []
          (let [f (make-show)
                s "hello"]
            (f s)))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let call_indirect_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "let 束縛 string 引数を使う closure call は receiver と string 引数を root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "let 束縛 string 引数を使う closure call は 2 回 root_pop するべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < call_indirect_pos,
        "let 束縛 string 引数は call_indirect 前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

// --- クロージャ変換テスト ---

#[test]
fn test_closure_object_allocates_heap() {
    // 自由変数あり Lambda はクロージャオブジェクトをヒープに確保すべき
    let source = r#"
        (defn make-adder [n] (fn [x] (+ x n)))
        (defn main [] 0)
    "#;
    let module = lower(source);
    let make_adder = module
        .functions
        .iter()
        .find(|f| f.name == "make-adder")
        .unwrap();
    // __alloc への Call が含まれるべき (クロージャオブジェクト確保)
    let has_alloc = make_adder
        .body
        .iter()
        .any(|i| matches!(i, Instruction::Call(1)));
    assert!(
        has_alloc,
        "クロージャは __alloc でヒープ確保すべき: {:?}",
        make_adder.body
    );
}

#[test]
fn test_closure_object_writes_tag() {
    // クロージャオブジェクトにヒープタグ (tag=4) を書き込むべき
    let source = r#"
        (defn make-adder [n] (fn [x] (+ x n)))
        (defn main [] 0)
    "#;
    let module = lower(source);
    let make_adder = module
        .functions
        .iter()
        .find(|f| f.name == "make-adder")
        .unwrap();
    // I32Const(4) (HEAP_TAG_CLOSURE) が含まれるべき
    let has_tag = make_adder
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I32Const(4)));
    assert!(
        has_tag,
        "クロージャはヒープタグ 4 を書き込むべき: {:?}",
        make_adder.body
    );
}

#[test]
fn test_closure_object_writes_func_idx() {
    // クロージャオブジェクトに func_idx を書き込むべき
    let source = r#"
        (defn make-adder [n] (fn [x] (+ x n)))
        (defn main [] 0)
    "#;
    let module = lower(source);
    let make_adder = module
        .functions
        .iter()
        .find(|f| f.name == "make-adder")
        .unwrap();
    // I32Store { offset: 4 } が含まれるべき (func_idx の書き込み)
    let has_func_idx_store = make_adder
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I32Store { offset: 4 }));
    assert!(
        has_func_idx_store,
        "クロージャは func_idx を offset 4 に書き込むべき: {:?}",
        make_adder.body
    );
}

#[test]
fn test_closure_object_captures_free_vars() {
    // クロージャオブジェクトにキャプチャ値を書き込むべき
    let source = r#"
        (defn make-adder [n] (fn [x] (+ x n)))
        (defn main [] 0)
    "#;
    let module = lower(source);
    let make_adder = module
        .functions
        .iter()
        .find(|f| f.name == "make-adder")
        .unwrap();
    // キャプチャ値の書き込み: I64Store { offset: 8 } が含まれるべき
    let has_capture_store = make_adder
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I64Store { offset: 8 }));
    assert!(
        has_capture_store,
        "クロージャはキャプチャ値を offset 8 に書き込むべき: {:?}",
        make_adder.body
    );
}

#[test]
fn test_closure_returns_tagged_pointer() {
    // クロージャオブジェクトはタグ付きポインタを返すべき
    let source = r#"
        (defn make-adder [n] (fn [x] (+ x n)))
        (defn main [] 0)
    "#;
    let module = lower(source);
    let make_adder = module
        .functions
        .iter()
        .find(|f| f.name == "make-adder")
        .unwrap();
    // タグ付きポインタ: I64Const(1<<63) + I64Add で最上位ビットをセット
    let has_tag_ptr = make_adder
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I64Const(n) if *n == (1i64 << 63)));
    assert!(
        has_tag_ptr,
        "クロージャはタグ付きポインタを返すべき: {:?}",
        make_adder.body
    );
}

#[test]
fn test_closure_no_free_vars_still_allocates() {
    // 自由変数なし Lambda もクロージャオブジェクトを確保する（統一呼び出し規約）
    let source = r#"
        (defn make-inc [] (fn [x] (+ x 1)))
        (defn main [] 0)
    "#;
    let module = lower(source);
    let make_inc = module
        .functions
        .iter()
        .find(|f| f.name == "make-inc")
        .unwrap();
    // __alloc への Call が含まれるべき
    let has_alloc = make_inc
        .body
        .iter()
        .any(|i| matches!(i, Instruction::Call(1)));
    assert!(
        has_alloc,
        "自由変数なし Lambda もクロージャオブジェクトを確保すべき: {:?}",
        make_inc.body
    );
}

// --- ADT パターンマッチ リニアメモリ版テスト ---

#[test]
fn test_adt_pattern_match_linear_memory_untags_pointer() {
    // 引数なしコンストラクタのパターンマッチでタグ付きポインタの解除が行われることを検証
    let source = r#"
        (type Color Red Green Blue)
        (defn color-to-int [c]
          (match c
            [Red 0]
            [Green 1]
            [Blue 2]))
    "#;
    let module = lower(source);
    let func = module
        .functions
        .iter()
        .find(|f| f.name == "color-to-int")
        .unwrap();
    // タグ付きポインタの解除: I32WrapI64 が含まれるべき
    let has_untag = func
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I32WrapI64));
    assert!(
        has_untag,
        "ADT パターンマッチはタグ付きポインタを解除すべき: {:?}",
        func.body
    );
}

#[test]
fn test_adt_pattern_match_linear_memory_loads_variant_tag() {
    // パターンマッチでバリアントタグをメモリから読み出すことを検証
    let source = r#"
        (type Color Red Green Blue)
        (defn color-to-int [c]
          (match c
            [Red 0]
            [Green 1]
            [Blue 2]))
    "#;
    let module = lower(source);
    let func = module
        .functions
        .iter()
        .find(|f| f.name == "color-to-int")
        .unwrap();
    // variant_tag の読み出し: I32Load { offset: 4 } が含まれるべき
    let has_tag_load = func
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I32Load { offset: 4 }));
    assert!(
        has_tag_load,
        "ADT パターンマッチは variant_tag を offset 4 から読み出すべき: {:?}",
        func.body
    );
}

#[test]
fn test_adt_pattern_match_with_fields_loads_from_memory() {
    // 引数付きコンストラクタのパターンマッチでフィールドをメモリから読み出すことを検証
    let source = r#"
        (type (Maybe a) (Just a) Nothing)
        (defn from-maybe [m default]
          (match m
            [(Just x) x]
            [Nothing default]))
    "#;
    let module = lower(source);
    let func = module
        .functions
        .iter()
        .find(|f| f.name == "from-maybe")
        .unwrap();
    // フィールド読み出し: I64Load { offset: 8 } が含まれるべき (field_0)
    let has_field_load = func
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I64Load { offset: 8 }));
    assert!(
        has_field_load,
        "引数付きコンストラクタのパターンマッチはフィールドを offset 8 から読み出すべき: {:?}",
        func.body
    );
}

#[test]
fn test_adt_pattern_match_with_fields_checks_variant_tag() {
    // 引数付きコンストラクタのパターンマッチでバリアントタグを比較することを検証
    let source = r#"
        (type (Maybe a) (Just a) Nothing)
        (defn from-maybe [m default]
          (match m
            [(Just x) x]
            [Nothing default]))
    "#;
    let module = lower(source);
    let func = module
        .functions
        .iter()
        .find(|f| f.name == "from-maybe")
        .unwrap();
    // タグ比較: I32Load { offset: 4 } でバリアントタグを読み出す
    let has_tag_comparison = func
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I32Load { offset: 4 }));
    assert!(
        has_tag_comparison,
        "引数付きコンストラクタのパターンマッチはバリアントタグを比較すべき: {:?}",
        func.body
    );
}

#[test]
fn test_adt_pattern_match_multi_field_loads() {
    // 複数フィールドのコンストラクタで各フィールドが正しいオフセットから読み出されることを検証
    let source = r#"
        (type (Pair a b) (MkPair a b))
        (defn fst [p]
          (match p
            [(MkPair a b) a]))
    "#;
    let module = lower(source);
    let func = module.functions.iter().find(|f| f.name == "fst").unwrap();
    // field_0 は offset 8 から読み出されるべき
    let has_field0 = func
        .body
        .iter()
        .any(|i| matches!(i, Instruction::I64Load { offset: 8 }));
    assert!(
        has_field0,
        "MkPair の field_0 は offset 8 から読み出すべき: {:?}",
        func.body
    );
}
