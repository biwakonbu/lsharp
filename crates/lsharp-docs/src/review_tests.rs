use super::*;

#[test]
fn test_generate_review_empty_source() {
    let status = DocStatus::default();
    let checkpoint = generate_review("test.ls", "", &status, &[]);
    assert_eq!(checkpoint.summary.total, 0);
}

#[test]
fn test_generate_review_with_defn() {
    let source = r#"(defn add [x y] (+ x y))"#;
    let status = DocStatus::default();
    let checkpoint = generate_review("test.ls", source, &status, &[]);
    assert_eq!(checkpoint.summary.total, 1);
    assert_eq!(checkpoint.entries[0].name, "add");
    assert_eq!(checkpoint.entries[0].freshness, Freshness::Unreviewed);
    assert!(!checkpoint.entries[0].has_doc);
}

#[test]
fn test_generate_review_with_doc_metadata() {
    let source = r#"(defn add [x y]
  :doc "二つの値を加算する"
  (+ x y))"#;
    let status = DocStatus::default();
    let checkpoint = generate_review("test.ls", source, &status, &[]);
    assert!(checkpoint.entries[0].has_doc);
}

#[test]
fn test_generate_review_with_fresh_status() {
    let source = r#"(defn add [x y] (+ x y))"#;
    let mut status = DocStatus::default();
    status.entries.insert(
        "add".to_string(),
        crate::tracker::DocEntry {
            ast_hash: 100,
            doc_hash: 200,
            last_reviewed: Some("2025-01-01".to_string()),
            reviewed_by: Some("dev".to_string()),
            freshness: Freshness::Fresh,
        },
    );
    let checkpoint = generate_review("test.ls", source, &status, &[]);
    assert_eq!(checkpoint.entries[0].freshness, Freshness::Fresh);
    assert_eq!(checkpoint.entries[0].reviewed_by, Some("dev".to_string()));
}

#[test]
fn test_generate_review_with_metadata_issues() {
    let source = r#"(defn add [x y] (+ x y))"#;
    let status = DocStatus::default();
    let diags = vec!["add: :params に未知の引数 'z' があります".to_string()];
    let checkpoint = generate_review("test.ls", source, &status, &diags);
    assert_eq!(checkpoint.entries[0].metadata_issues.len(), 1);
    assert_eq!(checkpoint.summary.with_issues, 1);
}

#[test]
fn test_format_yaml() {
    let checkpoint = ReviewCheckpoint {
        file: "test.ls".to_string(),
        entries: vec![ReviewEntry {
            name: "add".to_string(),
            freshness: Freshness::Unreviewed,
            metadata_issues: vec![],
            has_doc: false,
            reviewed_by: None,
            last_reviewed: None,
            span_start: 0,
            span_end: 0,
        }],
        summary: ReviewSummary {
            total: 1,
            fresh: 0,
            stale: 0,
            unreviewed: 1,
            with_issues: 0,
        },
    };
    let yaml = format_yaml(&checkpoint);
    assert!(yaml.contains("---"));
    assert!(yaml.contains("file: \"test.ls\""));
    assert!(yaml.contains("name: \"add\""));
    assert!(yaml.contains("freshness: \"unreviewed\""));
    assert!(yaml.contains("total: 1"));
}

#[test]
fn test_format_yaml_escapes_double_quoted_scalars() {
    let checkpoint = ReviewCheckpoint {
        file: "docs/\"review\"\nnext\\file".to_string(),
        entries: vec![ReviewEntry {
            name: "fn \"quoted\"".to_string(),
            freshness: Freshness::Unreviewed,
            metadata_issues: vec!["line \"quoted\"\nnext".to_string()],
            has_doc: false,
            reviewed_by: Some("alice\\review".to_string()),
            last_reviewed: Some("2026-07-31T12:34:56Z".to_string()),
            span_start: 0,
            span_end: 1,
        }],
        summary: ReviewSummary {
            total: 1,
            fresh: 0,
            stale: 0,
            unreviewed: 1,
            with_issues: 1,
        },
    };

    let yaml = format_yaml(&checkpoint);

    assert!(
        yaml.contains(r##"file: "docs/\"review\"\nnext\\file""##),
        "file path は YAML double-quoted scalar として escape されるべき: {yaml}"
    );
    assert!(
        yaml.contains(r##"  - name: "fn \"quoted\"""##),
        "entry name は quote を壊さず escape されるべき: {yaml}"
    );
    assert!(
        yaml.contains(r##"    reviewed_by: "alice\\review""##),
        "reviewer は backslash を escape されるべき: {yaml}"
    );
    assert!(
        yaml.contains(r##"      - "line \"quoted\"\nnext""##),
        "metadata issue は改行を scalar 内へ保持すべき: {yaml}"
    );
}

#[test]
fn test_generate_review_with_type_defs() {
    let source = r#"(type Point (record (: x Int) (: y Int)))"#;
    let status = DocStatus::default();
    let checkpoint = generate_review("test.ls", source, &status, &[]);
    assert_eq!(checkpoint.summary.total, 1);
    assert_eq!(checkpoint.entries[0].name, "Point");
}
