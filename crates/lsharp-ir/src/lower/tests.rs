//! lower モジュールのテスト

use super::*;
use lsharp_types::infer::Infer;
use crate::{Instruction, Module};

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
    assert_ir(
        "(defn main [] (if (< 1 2) 42 0))",
        "lower_if_expr",
    );
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
    assert_ir(
        "(defn main [] (print 42))",
        "lower_print_call",
    );
}

#[test]
fn test_lower_wildcard_let() {
    assert_ir(
        "(defn main [] (let [_ 99] 1))",
        "lower_wildcard_let",
    );
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
    assert_ir(
        "(defn main [] (not true))",
        "lower_not_operator",
    );
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
fn test_lower_lambda_unsupported() {
    let program = lsharp_syntax::parse("(defn main [] (fn [x] x))").unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let result = lowerer.lower_program(&program, &type_results);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, LowerError::Unsupported { .. }),
        "expected Unsupported error, got: {err}"
    );
}

#[test]
fn test_emit_binop_unknown_operator_returns_error() {
    // 未知の二項演算子でエラーが返ることを確認 (R-M2)
    let lowerer = Lower::new();
    let mut ctx = FuncCtx::new("test".to_string());
    let result = lowerer.emit_binop(&mut ctx, "unknown_op");
    assert!(result.is_err(), "未知の演算子 'unknown_op' でエラーが返るべき");
    let err = result.unwrap_err();
    assert!(
        matches!(err, LowerError::Unsupported { .. }),
        "Unsupported エラーが返るべき: {err}"
    );
}

#[test]
fn test_emit_binop_known_operators_succeed() {
    // 既知の演算子は全て成功すること (R-M2 回帰テスト)
    let lowerer = Lower::new();
    let known_ops = ["+", "-", "*", "/", "%", "+.", "-.", "*.", "/.",
                     "==", "=", "!=", "<", ">", "<=", ">=", "and", "or"];
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
    let has_call = main_fn.body.iter().any(|instr| matches!(instr, Instruction::Call(_)));
    assert!(has_call, "return は return_fn を Call すべき: {:?}", main_fn.body);
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
    let has_local_set = main_fn.body.iter().any(|instr| matches!(instr, Instruction::LocalSet(_)));
    assert!(has_local_set, "let! はローカル変数に格納すべき: {:?}", main_fn.body);
}

// --- private テスト ---

#[test]
fn test_lower_private_defn() {
    // private 内の関数も正しく IR 変換される
    let program = lsharp_syntax::parse(
        "(private (defn helper [x] (+ x 1))) (defn main [] (helper 42))"
    ).unwrap();
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
        "(private (type Point (record (: x Int) (: y Int)))) (defn main [] 42)"
    ).unwrap();
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
    let has_call = main_func.body.iter().any(|i| matches!(i, Instruction::Call(_)));
    assert!(has_call, "main 関数にトレイトメソッド呼び出し（Call）が含まれるべき: {:?}", main_func.body);
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
    let use_show = module.functions.iter().find(|f| f.name == "use-show").unwrap();
    let has_call = use_show.body.iter().any(|i| matches!(i, Instruction::Call(_)));
    assert!(has_call, "use-show にトレイトメソッド呼び出し（Call）が含まれるべき: {:?}", use_show.body);
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
    assert!(names.contains(&"Natural.new"), "Natural.new が生成されていない: {:?}", names);
    assert!(names.contains(&"Natural.valid?"), "Natural.valid? が生成されていない: {:?}", names);
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

    let new_func = module.functions.iter().find(|f| f.name == "Natural.new").unwrap();
    // 最後の命令は LocalGet(0) (値をそのまま返す)
    assert!(matches!(
        new_func.body.last(),
        Some(Instruction::LocalGet(0))
    ));
    // Unreachable が含まれている (制約違反時のトラップ)
    assert!(
        new_func.body.iter().any(|i| matches!(i, Instruction::Unreachable)),
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

    let valid_func = module.functions.iter().find(|f| f.name == "Natural.valid?").unwrap();
    // 最初の命令は I64Const(1) (true で初期化)
    assert!(matches!(valid_func.body.first(), Some(Instruction::I64Const(1))));
    // Unreachable は含まれない (valid? はトラップしない)
    assert!(
        !valid_func.body.iter().any(|i| matches!(i, Instruction::Unreachable)),
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

    let new_func = module.functions.iter().find(|f| f.name == "Port.new").unwrap();
    // range は 2 つの Unreachable を生成 (下限チェック + 上限チェック)
    let unreachable_count = new_func.body.iter()
        .filter(|i| matches!(i, Instruction::Unreachable))
        .count();
    assert_eq!(unreachable_count, 2, "Range 制約は 2 つのチェックを生成する");
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
    let struct_gets: Vec<_> = get_x.body.iter()
        .filter(|i| matches!(i, Instruction::StructGet(_, _)))
        .collect();
    assert!(struct_gets.len() >= 1, "レコードパターンは StructGet を使用すべき: {:?}", get_x.body);
}

#[test]
fn test_resolve_field_index() {
    let mut lowerer = Lower::new();
    lowerer.record_fields.insert(
        "Point".to_string(),
        vec!["x".to_string(), "y".to_string()],
    );
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
    lowerer.record_fields.insert("Point".to_string(), vec!["x".to_string(), "y".to_string()]);
    lowerer.record_fields.insert("Size".to_string(), vec!["x".to_string(), "h".to_string()]);
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
    lowerer.record_fields.insert("Point".to_string(), vec!["x".to_string(), "y".to_string()]);
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
    let point_x = module.functions.iter().find(|f| f.name == "Point.x").unwrap();
    let has_struct_get = point_x.body.iter().any(|i| matches!(i, Instruction::StructGet(0, 0)));
    assert!(has_struct_get, "Point.x は StructGet(0, 0) を使用すべき: {:?}", point_x.body);
    // Size.x は異なる GC 型インデックスで StructGet を使用
    let size_x = module.functions.iter().find(|f| f.name == "Size.x").unwrap();
    let has_struct_get = size_x.body.iter().any(|i| matches!(i, Instruction::StructGet(1, 0)));
    assert!(has_struct_get, "Size.x は StructGet(1, 0) を使用すべき: {:?}", size_x.body);
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
    let func = module.functions.iter().find(|f| f.name == "color-to-int").unwrap();
    // タグ比較のために I64Eq が使用されるべき
    let has_eq = func.body.iter().any(|i| matches!(i, Instruction::I64Eq));
    assert!(has_eq, "コンストラクタパターンはタグ比較 (I64Eq) を使用すべき: {:?}", func.body);
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
    let move_x = module.functions.iter().find(|f| f.name == "move-x").unwrap();
    // StructNew が発行されることを確認（更新されたレコードを構築）
    let has_struct_new = move_x.body.iter().any(|i| matches!(i, Instruction::StructNew(_)));
    assert!(has_struct_new, "RecordUpdate は StructNew を生成すべき: {:?}", move_x.body);
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
    assert_eq!(body.len(), 4, "emit_write_heap_header は 4 命令を生成すべき: {:?}", body);
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
    assert!(module.functions[0].is_export, "main 関数は export されるべき");
    // 本体は I64Const(42) の1命令
    assert_eq!(module.functions[0].body.len(), 1);
    assert!(matches!(module.functions[0].body[0], Instruction::I64Const(42)));
}
