use super::*;

#[test]
fn test_find_references_function_name() {
    // 関数名 "add" の定義と使用箇所
    let source = "(defn add [x y] (+ x y))\n(defn main [] (add 1 2))";
    // "add" の先頭位置 (offset 6)
    let pos = Position::new(0, 6);
    let refs = find_references(source, pos, true);
    // 定義 1 箇所 + 使用 1 箇所 = 2 箇所
    assert!(
        refs.len() >= 2,
        "add の参照は 2 箇所以上あるべき (定義+使用), 実際: {}",
        refs.len()
    );
}

#[test]
fn test_find_references_parameter() {
    // パラメータ "x" の定義と使用箇所
    let source = "(defn f [x] (+ x x))";
    // "x" パラメータの位置 (offset 9)
    let pos = Position::new(0, 9);
    let refs = find_references(source, pos, true);
    // 定義 1 箇所 + 使用 2 箇所 = 3 箇所
    assert_eq!(refs.len(), 3, "x の参照は 3 箇所あるべき (定義+使用*2)");
}

#[test]
fn test_find_references_let_binding() {
    // let バインディング "a" の定義と使用箇所
    let source = "(defn f [] (let [a 1] (+ a a)))";
    // "a" の定義位置 (let [ の直後)
    let pos = Position::new(0, 17);
    let refs = find_references(source, pos, true);
    // 定義 1 箇所 + 使用 2 箇所 = 3 箇所
    assert_eq!(refs.len(), 3, "a の参照は 3 箇所あるべき (定義+使用*2)");
}

#[test]
fn test_find_references_exclude_declaration() {
    // include_declaration: false で定義箇所を除外
    let source = "(defn f [x] (+ x x))";
    let pos = Position::new(0, 9);
    let refs = find_references(source, pos, false);
    // 使用 2 箇所のみ
    assert_eq!(refs.len(), 2, "定義除外で x の参照は 2 箇所あるべき");
}

#[test]
fn test_find_references_undefined() {
    // ソース内に存在しないシンボル
    let source = "(defn f [] 42)";
    // 空白位置を指す
    let pos = Position::new(0, 5);
    let refs = find_references(source, pos, true);
    // "f" は定義されているが、使用箇所はない → 定義のみ
    // (空白なら空)
    // offset 5 = 'f' → find_references は定義を含むので 1
    assert!(refs.len() <= 1, "未使用シンボルの参照は定義のみ");
}

#[test]
fn test_find_references_on_whitespace() {
    // 空白上では参照なし
    let source = "(defn f [] 42)";
    let pos = Position::new(0, 4); // space before 'f'
    let refs = find_references(source, pos, true);
    assert_eq!(refs.len(), 0, "空白上では参照なし");
}

#[test]
fn test_find_references_traverses_annotation_and_field_access() {
    let source = "(defn f [] (let [value 1] (: (. value field) Int)))";
    let binding_offset = source.find("value").expect("value binding should exist");
    let pos = crate::util::offset_to_position(source, binding_offset);
    let refs = find_references(source, pos, true);

    assert_eq!(
        refs.len(),
        2,
        "annotation/field access 内の value も参照されるべき"
    );
}
