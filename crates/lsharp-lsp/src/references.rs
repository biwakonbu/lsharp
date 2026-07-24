use tower_lsp::lsp_types::*;

use crate::util::{
    collect_definitions, collect_usages, offset_to_position, position_to_offset, symbol_at_position,
};

/// ソースコード内の指定位置にあるシンボルの全参照箇所を検索する
///
/// - source: ソースコード全文
/// - position: LSP Position (行・列)
/// - include_declaration: 定義箇所を含めるかどうか
///
/// 戻り値: 参照箇所の LSP Range のリスト
pub fn find_references(source: &str, position: Position, include_declaration: bool) -> Vec<Range> {
    let offset = match position_to_offset(source, position) {
        Some(o) => o,
        None => return Vec::new(),
    };

    let symbol_name = match symbol_at_position(source, offset) {
        Some(n) => n,
        None => return Vec::new(),
    };

    let program = match lsharp_syntax::parse(source).ok() {
        Some(p) => p,
        None => return Vec::new(),
    };

    let mut ranges = Vec::new();

    // 定義箇所を含める場合
    if include_declaration {
        let definitions = collect_definitions(&program, source);
        for def in &definitions {
            if def.name == symbol_name {
                let start = offset_to_position(source, def.start);
                let end = offset_to_position(source, def.end);
                ranges.push(Range::new(start, end));
            }
        }
    }

    // 使用箇所を収集
    let usages = collect_usages(&program);
    for usage in &usages {
        if usage.name == symbol_name {
            let start = offset_to_position(source, usage.start);
            let end = offset_to_position(source, usage.end);
            ranges.push(Range::new(start, end));
        }
    }

    ranges
}

#[cfg(test)]
#[path = "references_tests.rs"]
mod tests;
