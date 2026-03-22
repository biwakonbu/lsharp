//! ドキュメント追跡エンジン
//!
//! AST ノードのハッシュを計算し、コメントの鮮度を管理する。
//! コードが変更されたらドキュメントの再レビューを要求する。

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

/// ドキュメント追跡状態
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DocStatus {
    /// 関数名 -> ドキュメント状態
    pub entries: HashMap<String, DocEntry>,
}

/// 個別のドキュメント状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocEntry {
    /// AST ハッシュ（コード変更検出用）
    pub ast_hash: u64,
    /// ドキュメントハッシュ（コメント変更検出用）
    pub doc_hash: u64,
    /// 最終確認日時 (ISO 8601)
    pub last_reviewed: Option<String>,
    /// 確認者
    pub reviewed_by: Option<String>,
    /// 鮮度状態
    pub freshness: Freshness,
}

/// ドキュメントの鮮度
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Freshness {
    /// 最新（コードもドキュメントも変更なし）
    Fresh,
    /// 古い（コードが変更された）
    Stale,
    /// 未確認（まだレビューされていない）
    Unreviewed,
}

/// AST からドキュメント追跡用ハッシュを計算
pub fn compute_ast_hash(source: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    // 空白とコメントを除去してからハッシュ（フォーマット変更に耐性）
    let normalized: String = source
        .lines()
        .filter(|line| !line.trim_start().starts_with(';'))
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    normalized.hash(&mut hasher);
    hasher.finish()
}

/// ドキュメント文字列のハッシュを計算
pub fn compute_doc_hash(doc: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    doc.hash(&mut hasher);
    hasher.finish()
}

/// DocStatus をキャッシュファイルからロード
pub fn load_doc_status(path: &std::path::Path) -> DocStatus {
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => DocStatus::default(),
    }
}

/// DocStatus をキャッシュファイルに保存
pub fn save_doc_status(status: &DocStatus, path: &std::path::Path) -> Result<(), std::io::Error> {
    let json = serde_json::to_string_pretty(status)?;
    std::fs::write(path, json)
}

/// コードの変更を検出して鮮度を更新
pub fn update_freshness(status: &mut DocStatus, name: &str, current_ast_hash: u64) {
    if let Some(entry) = status.entries.get_mut(name) {
        if entry.ast_hash != current_ast_hash {
            entry.freshness = Freshness::Stale;
            entry.ast_hash = current_ast_hash;
        }
    }
}

/// ドキュメントを確認済みとしてマーク
pub fn acknowledge(status: &mut DocStatus, name: &str, reviewer: &str) {
    if let Some(entry) = status.entries.get_mut(name) {
        entry.freshness = Freshness::Fresh;
        entry.last_reviewed = Some(chrono_now());
        entry.reviewed_by = Some(reviewer.to_string());
    }
}

/// 現在時刻を ISO 8601 形式で返す（簡易実装）
fn chrono_now() -> String {
    // 外部クレートなしの簡易実装
    use std::time::SystemTime;
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}s", duration.as_secs())
}

#[cfg(test)]
mod tests {
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
        assert_eq!(
            compute_ast_hash(compact),
            compute_ast_hash(spaced)
        );
    }

    #[test]
    fn test_compute_ast_hash_ignores_blank_lines() {
        let without_blanks = "(defn add [x y]
  (+ x y))";
        let with_blanks = "(defn add [x y]

  (+ x y))

";
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
        status.entries.insert("add".to_string(), DocEntry {
            ast_hash: 100,
            doc_hash: 200,
            last_reviewed: None,
            reviewed_by: None,
            freshness: Freshness::Fresh,
        });

        // コードが変更されたら Stale になる
        update_freshness(&mut status, "add", 999);
        assert_eq!(status.entries["add"].freshness, Freshness::Stale);
    }

    #[test]
    fn test_freshness_unchanged() {
        let mut status = DocStatus::default();
        status.entries.insert("add".to_string(), DocEntry {
            ast_hash: 100,
            doc_hash: 200,
            last_reviewed: None,
            reviewed_by: None,
            freshness: Freshness::Fresh,
        });

        // 同じハッシュなら Fresh のまま
        update_freshness(&mut status, "add", 100);
        assert_eq!(status.entries["add"].freshness, Freshness::Fresh);
    }

    #[test]
    fn test_acknowledge() {
        let mut status = DocStatus::default();
        status.entries.insert("add".to_string(), DocEntry {
            ast_hash: 100,
            doc_hash: 200,
            last_reviewed: None,
            reviewed_by: None,
            freshness: Freshness::Stale,
        });

        acknowledge(&mut status, "add", "reviewer1");
        assert_eq!(status.entries["add"].freshness, Freshness::Fresh);
        assert!(status.entries["add"].reviewed_by.is_some());
    }

    #[test]
    fn test_doc_status_serialization() {
        let mut status = DocStatus::default();
        status.entries.insert("test".to_string(), DocEntry {
            ast_hash: 42,
            doc_hash: 84,
            last_reviewed: Some("2025-01-01".to_string()),
            reviewed_by: Some("dev".to_string()),
            freshness: Freshness::Fresh,
        });

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: DocStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.entries["test"].ast_hash, 42);
    }
}
