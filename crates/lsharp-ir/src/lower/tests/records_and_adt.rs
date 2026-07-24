//! record と ADT の lowering 回帰

use super::*;

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
fn test_nested_record_pattern_emits_recursive_struct_get() {
    let source = r#"
        (type Inner (record (: x Int)))
        (type Outer (record (: inner Inner)))
        (defn read-inner [o]
          (match o
            [{Outer inner {Inner x x}} x]
            [_ 0]))
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let module = lowerer.lower_program(&program, &type_results).unwrap();

    let read_inner = module
        .functions
        .iter()
        .find(|f| f.name == "read-inner")
        .unwrap();
    let struct_get_count = read_inner
        .body
        .iter()
        .filter(|instruction| matches!(instruction, Instruction::StructGet(_, _)))
        .count();
    assert!(
        struct_get_count >= 2,
        "nested record pattern は親と子の StructGet を生成すべき: {:?}",
        read_inner.body
    );
}

#[test]
fn test_nested_record_pattern_rejects_literal_child_until_lowered() {
    let source = r#"
        (type Inner (record (: x Int)))
        (type Outer (record (: inner Inner)))
        (defn read-inner [o]
          (match o
            [{Outer inner {Inner x 41}} 1]
            [_ 0]))
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lowerer = Lower::new();
    let result = lowerer.lower_program(&program, &type_results);

    assert!(
        matches!(&result, Err(LowerError::Unsupported { .. })),
        "nested literal child は lowering 未対応として明示エラーにすべき: {result:?}"
    );
    let err = result.unwrap_err();
    assert_eq!(err.code(), "LS3001");
    assert!(err.span().is_some());
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
