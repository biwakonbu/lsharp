use ropey::Rope;
use tower_lsp::lsp_types::{Position, TextDocumentContentChangeEvent};

pub(crate) fn apply_content_changes(
    source: &str,
    changes: &[TextDocumentContentChangeEvent],
) -> Result<String, String> {
    if changes.is_empty() {
        return Ok(source.to_string());
    }

    let mut rope = Rope::from_str(source);
    for change in changes {
        if let Some(range) = change.range {
            let Some(start) = position_to_char_index(&rope, range.start) else {
                return Ok(change.text.clone());
            };
            let Some(end) = position_to_char_index(&rope, range.end) else {
                return Ok(change.text.clone());
            };
            if start > end {
                return Ok(change.text.clone());
            }
            rope.remove(start..end);
            rope.insert(start, &change.text);
        } else {
            rope = Rope::from_str(&change.text);
        }
    }

    Ok(rope.to_string())
}

fn position_to_char_index(rope: &Rope, position: Position) -> Option<usize> {
    let line = position.line as usize;
    if line >= rope.len_lines() {
        return None;
    }

    let line_start = rope.line_to_char(line);
    let line_slice = rope.line(line);
    let mut char_index = 0usize;
    let mut utf16_column = 0u32;
    for ch in line_slice.chars() {
        if position.character == utf16_column {
            return Some(line_start + char_index);
        }
        if ch == '\n' {
            break;
        }

        let next_column = utf16_column + ch.len_utf16() as u32;
        if position.character < next_column {
            // サロゲートペアの途中など、文字境界でない位置は文字の先頭へ寄せる
            return Some(line_start + char_index);
        }
        utf16_column = next_column;
        char_index += 1;
    }

    if position.character == utf16_column {
        Some(line_start + char_index)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Range;

    fn ranged_change(
        start_line: u32,
        start_char: u32,
        end_line: u32,
        end_char: u32,
        text: &str,
    ) -> TextDocumentContentChangeEvent {
        TextDocumentContentChangeEvent {
            range: Some(Range::new(
                Position::new(start_line, start_char),
                Position::new(end_line, end_char),
            )),
            range_length: None,
            text: text.to_string(),
        }
    }

    #[test]
    fn test_apply_content_changes_replaces_single_range() {
        let updated = apply_content_changes(
            "(defn main [] (+ 1 2))\n",
            &[ranged_change(0, 19, 0, 20, "3")],
        )
        .expect("single range change should succeed");

        assert_eq!(updated, "(defn main [] (+ 1 3))\n");
    }

    #[test]
    fn test_apply_content_changes_uses_utf16_positions() {
        let updated = apply_content_changes("😀x\n", &[ranged_change(0, 2, 0, 3, "y")])
            .expect("UTF-16 range change should succeed");

        assert_eq!(updated, "😀y\n");
    }

    #[test]
    fn test_apply_content_changes_applies_multiple_incremental_edits() {
        let after_first = "(defn main [] (+ helper-three helper-two))\n";
        let helper_two_start = after_first.find("helper-two").expect("helper-two") as u32;
        let helper_two_end = helper_two_start + "helper-two".len() as u32;
        let updated = apply_content_changes(
            "(defn main [] (+ helper-one helper-two))\n",
            &[
                ranged_change(0, 17, 0, 27, "helper-three"),
                ranged_change(0, helper_two_start, 0, helper_two_end, "helper-four"),
            ],
        )
        .expect("multiple range changes should succeed");

        assert_eq!(updated, "(defn main [] (+ helper-three helper-four))\n");
    }

    #[test]
    fn test_apply_content_changes_falls_back_to_full_text_on_invalid_range() {
        let updated = apply_content_changes(
            "(defn main [] 1)\n",
            &[ranged_change(10, 0, 10, 0, "(defn main [] 2)\n")],
        )
        .expect("invalid range should fall back to full text");

        assert_eq!(updated, "(defn main [] 2)\n");
    }

    #[test]
    fn test_apply_content_changes_large_document_partial_edit_stays_fast() {
        let mut source = String::new();
        for idx in 0..1000 {
            source.push_str(&format!("(defn helper-{idx} [] {idx})\n"));
        }
        let replacement = "(defn helper-500 [] 999)\n";
        let start = std::time::Instant::now();
        let updated =
            apply_content_changes(&source, &[ranged_change(500, 0, 500, 24, replacement)])
                .expect("large document change should succeed");
        let elapsed = start.elapsed();

        assert!(
            updated.contains(replacement),
            "差分適用後は対象行が新しい内容へ置き換わるべき"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "1000 行 partial edit の差分適用は 50ms 未満で終わるべき: {:?}",
            elapsed
        );
    }
}
