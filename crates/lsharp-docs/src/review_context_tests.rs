use super::*;

#[test]
fn test_offset_to_line() {
    let source = "line1
line2
line3";
    assert_eq!(offset_to_line(source, 0), 1);
    assert_eq!(offset_to_line(source, 5), 1); // end of line1
    assert_eq!(offset_to_line(source, 6), 2); // start of line2
    assert_eq!(offset_to_line(source, 12), 3); // start of line3
}

#[test]
fn test_offset_to_line_empty() {
    assert_eq!(offset_to_line("", 0), 1);
}

#[test]
fn test_extract_context() {
    let source = "(module M)
(defn add [x y] (+ x y))
(defn sub [x y] (- x y))";
    let entry = ReviewEntry {
        name: "add".to_string(),
        freshness: Freshness::Unreviewed,
        metadata_issues: vec![],
        has_doc: false,
        reviewed_by: None,
        last_reviewed: None,
        span_start: 11, // start of (defn add ...)
        span_end: 36,   // end of (defn add ...)
    };
    let ctx = extract_context(source, &entry, 1);
    assert!(ctx.contains("(module M)"));
    assert!(ctx.contains("(defn add"));
    assert!(ctx.contains(">")); // marker for matching lines
}

#[test]
fn test_review_entry_has_span() {
    let source = "(defn add [x y] (+ x y))";
    let status = DocStatus::default();
    let checkpoint = generate_review("test.ls", source, &status, &[]);
    // span should be populated from AST
    let entry = &checkpoint.entries[0];
    assert!(entry.span_end > entry.span_start || (entry.span_start == 0 && entry.span_end > 0));
}
