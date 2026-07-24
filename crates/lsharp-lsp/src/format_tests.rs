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
    assert!(formatted.contains("(+ x y)"), "式の構造が保持されるべき");
}

#[test]
fn test_format_removes_trailing_whitespace() {
    let source = "(defn f []   \n  42)";
    let formatted = format_source(source);
    // 行末に余分な空白がないことを確認
    for line in formatted.lines() {
        assert_eq!(line, line.trim_end(), "行末に余分な空白があってはならない");
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
