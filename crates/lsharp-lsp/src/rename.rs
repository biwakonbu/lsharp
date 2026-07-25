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
#[path = "rename_tests.rs"]
mod tests;
