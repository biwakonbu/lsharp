//! module・Ref・lambda の lowering 回帰

use super::*;

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
