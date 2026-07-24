//! closure heap object と ADT memory の lowering 回帰

use super::*;

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
