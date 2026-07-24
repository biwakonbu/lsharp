use super::*;

#[test]
fn test_compute_ast_hash_ignores_comments() {
    let with_comment = "(defn add [x y]\n  ; add two numbers\n  (+ x y))";
    let without_comment = "(defn add [x y]\n  (+ x y))";
    assert_eq!(
        compute_ast_hash(with_comment),
        compute_ast_hash(without_comment)
    );
}

#[test]
fn test_compute_ast_hash_ignores_leading_trailing_whitespace() {
    // 行頭末の空白は無視される
    let compact = "(defn add [x y] (+ x y))";
    let spaced = "  (defn add [x y] (+ x y))  ";
    assert_eq!(compute_ast_hash(compact), compute_ast_hash(spaced));
}

#[test]
fn test_compute_ast_hash_ignores_blank_lines() {
    let without_blanks = "(defn add [x y]\n  (+ x y))";
    let with_blanks = "(defn add [x y]\n\n  (+ x y))\n\n";
    assert_eq!(
        compute_ast_hash(without_blanks),
        compute_ast_hash(with_blanks)
    );
}

#[test]
fn test_compute_ast_hash_detects_changes() {
    let v1 = "(defn add [x y] (+ x y))";
    let v2 = "(defn add [x y] (+ x y 1))";
    assert_ne!(compute_ast_hash(v1), compute_ast_hash(v2));
}

#[test]
fn test_freshness_update() {
    let mut status = DocStatus::default();
    status.entries.insert(
        "add".to_string(),
        DocEntry {
            ast_hash: 100,
            doc_hash: 200,
            last_reviewed: None,
            reviewed_by: None,
            freshness: Freshness::Fresh,
        },
    );

    // コードが変更されたら Stale になる
    update_freshness(&mut status, "add", 999);
    assert_eq!(status.entries["add"].freshness, Freshness::Stale);
}

#[test]
fn test_freshness_unchanged() {
    let mut status = DocStatus::default();
    status.entries.insert(
        "add".to_string(),
        DocEntry {
            ast_hash: 100,
            doc_hash: 200,
            last_reviewed: None,
            reviewed_by: None,
            freshness: Freshness::Fresh,
        },
    );

    // 同じハッシュなら Fresh のまま
    update_freshness(&mut status, "add", 100);
    assert_eq!(status.entries["add"].freshness, Freshness::Fresh);
}

#[test]
fn test_acknowledge() {
    let mut status = DocStatus::default();
    status.entries.insert(
        "add".to_string(),
        DocEntry {
            ast_hash: 100,
            doc_hash: 200,
            last_reviewed: None,
            reviewed_by: None,
            freshness: Freshness::Stale,
        },
    );

    acknowledge(&mut status, "add", "reviewer1");
    assert_eq!(status.entries["add"].freshness, Freshness::Fresh);
    assert!(status.entries["add"].reviewed_by.is_some());
}

#[test]
fn test_doc_status_serialization() {
    let mut status = DocStatus::default();
    status.entries.insert(
        "test".to_string(),
        DocEntry {
            ast_hash: 42,
            doc_hash: 84,
            last_reviewed: Some("2025-01-01".to_string()),
            reviewed_by: Some("dev".to_string()),
            freshness: Freshness::Fresh,
        },
    );

    let json = serde_json::to_string(&status).unwrap();
    let deserialized: DocStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.entries["test"].ast_hash, 42);
}

#[test]
fn test_chrono_now_iso8601_format() {
    let now = chrono_now();
    // ISO 8601 形式 "YYYY-MM-DDTHH:MM:SSZ" に合致すること
    assert!(
        now.contains('T'),
        "ISO 8601 形式に 'T' が含まれるべき: {}",
        now
    );
    assert!(
        now.contains('-'),
        "ISO 8601 形式に '-' が含まれるべき: {}",
        now
    );
    assert!(
        now.ends_with('Z'),
        "UTC タイムゾーン 'Z' で終わるべき: {}",
        now
    );
    assert!(
        !now.ends_with('s'),
        "旧形式 '...s' であってはならない: {}",
        now
    );
    // 長さチェック: "YYYY-MM-DDTHH:MM:SSZ" = 20 文字
    assert_eq!(now.len(), 20, "ISO 8601 形式は 20 文字: {}", now);
}
