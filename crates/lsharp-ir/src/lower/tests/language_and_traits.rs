//! 言語構造・計算式・trait の回帰

use super::*;

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
        matches!(&err, LowerError::UndefinedFunction { .. }),
        "expected UndefinedFunction error, got: {err}"
    );
    assert_eq!(err.code(), "LS3002");
    assert_eq!(err.span(), Some(s));
}

#[test]
fn test_emit_binop_unknown_operator_returns_error() {
    // 未知の二項演算子でエラーが返ることを確認 (R-M2)
    let mut lowerer = Lower::new();
    let mut ctx = FuncCtx::with_type_scope("test".to_string(), "test".to_string());
    let result = lowerer.emit_binop(&mut ctx, "unknown_op", Span::dummy());
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
        let mut ctx = FuncCtx::with_type_scope("test".to_string(), "test".to_string());
        let result = lowerer.emit_binop(&mut ctx, op, Span::dummy());
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
    // 最初の命令は I32Const(1) (true で初期化)。
    // Wasm の i32.and と整合するよう、最後に i64 へ拡張する。
    assert!(matches!(
        valid_func.body.first(),
        Some(Instruction::I32Const(1))
    ));
    assert!(matches!(
        valid_func.body.last(),
        Some(Instruction::I64ExtendI32S)
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
