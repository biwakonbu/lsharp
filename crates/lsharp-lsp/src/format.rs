/// S 式ソースコードをインデント整形する基本フォーマッター
///
/// ルール:
/// - トップレベルの括弧はインデント 0
/// - ネストした括弧は親 + 2 のインデント
/// - 文字列リテラル内の空白は保持
/// - コメント行 (;; で始まる) は保持
/// - トップレベルフォーム間に空行 1 つ
/// - 連続する空行を 1 つに圧縮
/// - 行末の空白を除去
pub fn format_source(source: &str) -> String {
    let mut result = String::new();
    let mut indent = 0usize;
    let mut in_string = false;
    let mut line_start = true;
    let mut paren_stack: Vec<usize> = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();

    // 保留中の改行数 (圧縮のため遅延出力)
    let mut pending_newlines = 0u32;

    while i < len {
        let ch = chars[i];

        // 文字列リテラル内はそのまま出力
        if in_string {
            result.push(ch);
            if ch == '\\' && i + 1 < len {
                // エスケープシーケンスをスキップ
                i += 1;
                result.push(chars[i]);
            } else if ch == '"' {
                in_string = false;
            }
            line_start = ch == '\n';
            i += 1;
            continue;
        }

        match ch {
            '\n' => {
                // 行末の空白を除去
                trim_trailing_whitespace(&mut result);
                pending_newlines += 1;
                line_start = true;
                i += 1;
                continue;
            }
            ' ' | '\t' | '\r' => {
                if line_start {
                    // 行頭の空白はスキップ (インデントは自動計算)
                    i += 1;
                    continue;
                }
                // 連続する空白を 1 つに圧縮
                if !result.ends_with(' ') && !result.ends_with('\n') {
                    result.push(' ');
                }
                i += 1;
                continue;
            }
            _ => {}
        }

        // ここに到達 = 出力する文字がある
        // 保留中の改行を出力 (最大 2 = 空行 1 つ)
        if pending_newlines > 0 && !result.is_empty() {
            let newlines = pending_newlines.min(2);
            for _ in 0..newlines {
                result.push('\n');
            }
            pending_newlines = 0;
        }

        match ch {
            '"' => {
                if line_start {
                    push_indent(&mut result, indent);
                    line_start = false;
                }
                in_string = true;
                result.push(ch);
            }
            ';' => {
                // コメント: 行末まで出力
                if line_start {
                    push_indent(&mut result, indent);
                    line_start = false;
                }
                while i < len && chars[i] != '\n' {
                    result.push(chars[i]);
                    i += 1;
                }
                continue; // '\n' は次のイテレーションで処理
            }
            '(' => {
                if line_start {
                    push_indent(&mut result, indent);
                    line_start = false;
                } else if !result.ends_with(' ') && !result.ends_with('\n') {
                    result.push(' ');
                }
                result.push('(');
                paren_stack.push(indent);
                indent += 2;
            }
            ')' => {
                if line_start {
                    // 閉じ括弧が行頭にある場合
                    if let Some(&parent_indent) = paren_stack.last() {
                        push_indent(&mut result, parent_indent);
                    }
                    line_start = false;
                }
                result.push(')');
                if let Some(parent) = paren_stack.pop() {
                    indent = parent;
                }
            }
            _ => {
                if line_start {
                    push_indent(&mut result, indent);
                    line_start = false;
                }
                result.push(ch);
            }
        }

        i += 1;
    }

    // 末尾の空白を除去し、最後に改行を追加
    trim_trailing_whitespace(&mut result);
    if !result.is_empty() && !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// インデントを出力する
fn push_indent(result: &mut String, indent: usize) {
    for _ in 0..indent {
        result.push(' ');
    }
}

/// 末尾の空白 (スペース/タブ) を除去する
fn trim_trailing_whitespace(result: &mut String) {
    while result.ends_with(' ') || result.ends_with('\t') {
        result.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_defn() {
        // 正しいインデントはおおむね保持される
        let source = "(defn add [x y] (+ x y))";
        let formatted = format_source(source);
        assert!(
            formatted.contains("(defn"),
            "defn キーワードが保持されるべき"
        );
        assert!(
            formatted.contains("(+ x y)"),
            "式の構造が保持されるべき"
        );
    }

    #[test]
    fn test_format_removes_trailing_whitespace() {
        let source = "(defn f []   \n  42)";
        let formatted = format_source(source);
        // 行末に余分な空白がないことを確認
        for line in formatted.lines() {
            assert_eq!(
                line,
                line.trim_end(),
                "行末に余分な空白があってはならない"
            );
        }
    }

    #[test]
    fn test_format_compress_blank_lines() {
        let source = "(defn f [] 1)\n\n\n\n(defn g [] 2)";
        let formatted = format_source(source);
        // 3 つ以上の連続改行がないことを確認
        assert!(
            !formatted.contains("\n\n\n"),
            "連続する空行は 1 つに圧縮されるべき: {:?}",
            formatted
        );
    }

    #[test]
    fn test_format_preserve_comment() {
        let source = ";; これはコメント\n(defn f [] 42)";
        let formatted = format_source(source);
        assert!(
            formatted.contains(";; これはコメント"),
            "コメントが保持されるべき"
        );
    }

    #[test]
    fn test_format_string_literal_preserved() {
        let source = "(defn f [] \"hello  world\")";
        let formatted = format_source(source);
        assert!(
            formatted.contains("\"hello  world\""),
            "文字列リテラル内の空白は変更されるべきでない"
        );
    }

    #[test]
    fn test_format_ends_with_newline() {
        let source = "(defn f [] 42)";
        let formatted = format_source(source);
        assert!(
            formatted.ends_with('\n'),
            "フォーマット結果は改行で終わるべき"
        );
    }

    #[test]
    fn test_format_normalize_whitespace() {
        let source = "(defn   f   []   (+   x   y))";
        let formatted = format_source(source);
        // 余分な空白が圧縮されていることを確認
        assert!(
            !formatted.contains("  "),
            "余分な空白が圧縮されるべき: {:?}",
            formatted
        );
    }
}
