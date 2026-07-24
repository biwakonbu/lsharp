use tower_lsp::lsp_types::*;

use crate::references::find_references;
use crate::util::{offset_to_position, position_to_offset, symbol_range_at_position};

/// カーソル位置のシンボル名と範囲を返す (prepare_rename 用)
///
/// 戻り値: (シンボル名, LSP Range) — シンボル上でなければ None
pub fn prepare_rename(source: &str, position: Position) -> Option<(String, Range)> {
    let offset = position_to_offset(source, position)?;
    let (name, start, end) = symbol_range_at_position(source, offset)?;
    let start_pos = offset_to_position(source, start);
    let end_pos = offset_to_position(source, end);
    Some((name, Range::new(start_pos, end_pos)))
}

/// 全出現箇所を新しい名前に置換する TextEdit のリストを計算する
///
/// find_references で全箇所を取得し、各箇所に対して TextEdit を生成する。
pub fn compute_rename_edits(source: &str, position: Position, new_name: &str) -> Vec<TextEdit> {
    let refs = find_references(source, position, true);
    refs.into_iter()
        .map(|range| TextEdit {
            range,
            new_text: new_name.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_rename_on_symbol() {
        let source = "(defn add [x y] (+ x y))";
        // "add" の先頭
        let pos = Position::new(0, 6);
        let result = prepare_rename(source, pos);
        assert!(result.is_some(), "シンボル上で prepare_rename が成功すべき");
        let (name, range) = result.unwrap();
        assert_eq!(name, "add");
        assert_eq!(range.start, Position::new(0, 6));
    }

    #[test]
    fn test_prepare_rename_on_whitespace() {
        let source = "(defn add [x y] (+ x y))";
        // 空白位置 (offset 5 = defn と add の間のスペース)
        let pos = Position::new(0, 5);
        let result = prepare_rename(source, pos);
        assert!(result.is_none(), "空白上では None を返すべき");
    }

    #[test]
    fn test_rename_function() {
        let source = "(defn add [x y] (+ x y))\n(defn main [] (add 1 2))";
        // "add" をリネーム
        let pos = Position::new(0, 6);
        let edits = compute_rename_edits(source, pos, "sum");
        // 定義 1 + 使用 1 = 2 箇所以上
        assert!(
            edits.len() >= 2,
            "関数リネームで 2 箇所以上の TextEdit が生成されるべき, 実際: {}",
            edits.len()
        );
        assert!(
            edits
                .iter()
                .any(|edit| { edit.range == Range::new(Position::new(0, 6), Position::new(0, 9)) }),
            "関数定義の TextEdit は関数名だけを置換する範囲であるべき: {edits:?}"
        );
        for edit in &edits {
            assert_eq!(edit.new_text, "sum");
        }
    }

    #[test]
    fn test_rename_parameter() {
        let source = "(defn f [x] (+ x 1))";
        // "x" をリネーム
        let pos = Position::new(0, 9);
        let edits = compute_rename_edits(source, pos, "y");
        // 定義 1 + 使用 1 = 2 箇所
        assert!(
            edits.len() >= 2,
            "パラメータリネームで 2 箇所以上の TextEdit が生成されるべき, 実際: {}",
            edits.len()
        );
        for edit in &edits {
            assert_eq!(edit.new_text, "y");
        }
    }

    #[test]
    fn test_rename_typed_parameter_replaces_name_only() {
        let source = "(defn f [(: value Int)] value)";
        let edits = compute_rename_edits(source, Position::new(0, 12), "item");

        assert!(
            edits.iter().any(|edit| {
                edit.range == Range::new(Position::new(0, 12), Position::new(0, 17))
            }),
            "typed parameter の TextEdit は name だけを置換する範囲であるべき: {edits:?}"
        );
        assert!(
            edits.iter().all(|edit| edit.range.end.character <= 29),
            "typed parameter の annotation や宣言末尾まで置換範囲に含めてはいけない: {edits:?}"
        );
    }

    #[test]
    fn test_rename_let_binding() {
        let source = "(defn f [] (let [a 1] (+ a a)))";
        // "a" をリネーム
        let pos = Position::new(0, 17);
        let edits = compute_rename_edits(source, pos, "b");
        // 定義 1 + 使用 2 = 3 箇所
        assert_eq!(
            edits.len(),
            3,
            "let 変数リネームで 3 箇所の TextEdit が生成されるべき"
        );
        for edit in &edits {
            assert_eq!(edit.new_text, "b");
        }
    }
}
