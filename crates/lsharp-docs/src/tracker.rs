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
    if let Some(entry) = status.entries.get_mut(name)
        && entry.ast_hash != current_ast_hash
    {
        entry.freshness = Freshness::Stale;
        entry.ast_hash = current_ast_hash;
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

/// 現在時刻を ISO 8601 (RFC 3339) 形式で返す（簡易実装）
fn chrono_now() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // UNIX epoch からの日数と時刻を計算
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // 日付計算 (civil date from days since 1970-01-01)
    // アルゴリズム: Howard Hinnant の chrono-compatible 日付計算
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    let year = if m <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, m, d, hours, minutes, seconds
    )
}

#[cfg(test)]
#[path = "tracker_tests.rs"]
mod tests;
