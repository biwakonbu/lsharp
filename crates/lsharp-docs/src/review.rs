//! レビューチェックポイント出力
//!
//! `lsharp review` コマンドで使用される。
//! ソースファイルの関数・型ごとにレビューチェックポイントを YAML 形式で出力する。

use std::collections::HashMap;

use crate::tracker::{DocStatus, Freshness};

/// レビューチェックポイント
#[derive(Debug, Clone)]
pub struct ReviewCheckpoint {
    /// ファイルパス
    pub file: String,
    /// 各エントリのチェックポイント
    pub entries: Vec<ReviewEntry>,
    /// サマリー
    pub summary: ReviewSummary,
}

/// 個別のレビューエントリ
#[derive(Debug, Clone)]
pub struct ReviewEntry {
    /// 関数/型の名前
    pub name: String,
    /// ドキュメントの鮮度
    pub freshness: Freshness,
    /// メタデータ検証の結果
    pub metadata_issues: Vec<String>,
    /// ドキュメントの有無
    pub has_doc: bool,
    /// 最終レビュー者
    pub reviewed_by: Option<String>,
    /// 最終レビュー日時
    pub last_reviewed: Option<String>,
    /// ソース内の開始バイトオフセット
    pub span_start: usize,
    /// ソース内の終了バイトオフセット
    pub span_end: usize,
}

/// レビューサマリー
#[derive(Debug, Clone)]
pub struct ReviewSummary {
    /// 全エントリ数
    pub total: usize,
    /// Fresh なエントリ数
    pub fresh: usize,
    /// Stale なエントリ数
    pub stale: usize,
    /// 未レビューのエントリ数
    pub unreviewed: usize,
    /// メタデータ問題のあるエントリ数
    pub with_issues: usize,
}

/// ソースファイルからレビューチェックポイントを生成
pub fn generate_review(
    file_path: &str,
    source: &str,
    doc_status: &DocStatus,
    metadata_diagnostics: &[String],
) -> ReviewCheckpoint {
    use lsharp_syntax::ast::Decl;

    // パース
    let program = match lsharp_syntax::parse(source) {
        Ok(p) => p,
        Err(_e) => {
            return ReviewCheckpoint {
                file: file_path.to_string(),
                entries: vec![],
                summary: ReviewSummary {
                    total: 0,
                    fresh: 0,
                    stale: 0,
                    unreviewed: 0,
                    with_issues: 0,
                },
            };
        }
    };

    // 関数名 -> メタデータ問題のマッピング
    let mut issue_map: HashMap<String, Vec<String>> = HashMap::new();
    for diag in metadata_diagnostics {
        // 診断メッセージから関数名を抽出（"関数名: メッセージ" 形式を想定）
        if let Some(colon_pos) = diag.find(':') {
            let name = diag[..colon_pos].trim().to_string();
            issue_map
                .entry(name)
                .or_default()
                .push(diag[colon_pos + 1..].trim().to_string());
        }
    }

    let mut entries = Vec::new();

    for decl in &program.decls {
        let actual_decl = unwrap_private(decl);
        match actual_decl {
            Decl::Defn {
                name,
                metadata,
                span,
                ..
            } => {
                let has_doc = metadata.as_ref().map(|m| m.doc.is_some()).unwrap_or(false);

                let (freshness, reviewed_by, last_reviewed) =
                    if let Some(entry) = doc_status.entries.get(name) {
                        (
                            entry.freshness.clone(),
                            entry.reviewed_by.clone(),
                            entry.last_reviewed.clone(),
                        )
                    } else {
                        (Freshness::Unreviewed, None, None)
                    };

                let metadata_issues = issue_map.remove(name).unwrap_or_default();

                entries.push(ReviewEntry {
                    name: name.clone(),
                    freshness,
                    metadata_issues,
                    has_doc,
                    reviewed_by,
                    last_reviewed,
                    span_start: span.start,
                    span_end: span.end,
                });
            }
            Decl::RecordDef { name, span, .. }
            | Decl::TypeDef { name, span, .. }
            | Decl::TypeAlias { name, span, .. }
            | Decl::TraitDef { name, span, .. } => {
                let freshness = if let Some(entry) = doc_status.entries.get(name) {
                    entry.freshness.clone()
                } else {
                    Freshness::Unreviewed
                };

                entries.push(ReviewEntry {
                    name: name.clone(),
                    freshness,
                    metadata_issues: issue_map.remove(name).unwrap_or_default(),
                    has_doc: false,
                    reviewed_by: None,
                    last_reviewed: None,
                    span_start: span.start,
                    span_end: span.end,
                });
            }
            _ => {}
        }
    }

    // サマリー計算
    let total = entries.len();
    let fresh = entries
        .iter()
        .filter(|e| e.freshness == Freshness::Fresh)
        .count();
    let stale = entries
        .iter()
        .filter(|e| e.freshness == Freshness::Stale)
        .count();
    let unreviewed = entries
        .iter()
        .filter(|e| e.freshness == Freshness::Unreviewed)
        .count();
    let with_issues = entries
        .iter()
        .filter(|e| !e.metadata_issues.is_empty())
        .count();

    ReviewCheckpoint {
        file: file_path.to_string(),
        entries,
        summary: ReviewSummary {
            total,
            fresh,
            stale,
            unreviewed,
            with_issues,
        },
    }
}

/// Private 宣言をアンラップ
fn unwrap_private(decl: &lsharp_syntax::ast::Decl) -> &lsharp_syntax::ast::Decl {
    match decl {
        lsharp_syntax::ast::Decl::Private { inner, .. } => unwrap_private(inner),
        other => other,
    }
}

/// ソース内のバイトオフセットを行番号に変換
pub fn offset_to_line(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())]
        .chars()
        .filter(|c| *c == '\n')
        .count()
        + 1
}

/// エントリの前後のソースコードを差分表示用に抽出
pub fn extract_context(source: &str, entry: &ReviewEntry, context_lines: usize) -> String {
    let start_line = offset_to_line(source, entry.span_start);
    let end_line = offset_to_line(source, entry.span_end);

    let lines: Vec<&str> = source.lines().collect();
    let first = start_line.saturating_sub(context_lines + 1);
    let last = (end_line + context_lines).min(lines.len());

    let mut output = String::new();
    for (i, line) in lines.iter().enumerate().skip(first).take(last - first) {
        let marker = if i + 1 >= start_line && i < end_line {
            ">"
        } else {
            " "
        };
        output.push_str(&format!(
            "{marker} {:>4} | {}
",
            i + 1,
            line
        ));
    }
    output
}

/// レビューチェックポイントを YAML 形式で出力
pub fn format_yaml(checkpoint: &ReviewCheckpoint) -> String {
    let mut out = String::new();

    out.push_str("---\n");
    out.push_str(&format!("file: \"{}\"\n", checkpoint.file));
    out.push_str("entries:\n");

    for entry in &checkpoint.entries {
        out.push_str(&format!("  - name: \"{}\"\n", entry.name));
        out.push_str(&format!(
            "    freshness: \"{}\"\n",
            freshness_str(&entry.freshness)
        ));
        out.push_str(&format!("    has_doc: {}\n", entry.has_doc));

        if let Some(ref reviewer) = entry.reviewed_by {
            out.push_str(&format!("    reviewed_by: \"{}\"\n", reviewer));
        }
        if let Some(ref date) = entry.last_reviewed {
            out.push_str(&format!("    last_reviewed: \"{}\"\n", date));
        }

        if !entry.metadata_issues.is_empty() {
            out.push_str("    issues:\n");
            for issue in &entry.metadata_issues {
                out.push_str(&format!("      - \"{}\"\n", issue));
            }
        }
    }

    out.push_str("summary:\n");
    out.push_str(&format!("  total: {}\n", checkpoint.summary.total));
    out.push_str(&format!("  fresh: {}\n", checkpoint.summary.fresh));
    out.push_str(&format!("  stale: {}\n", checkpoint.summary.stale));
    out.push_str(&format!(
        "  unreviewed: {}\n",
        checkpoint.summary.unreviewed
    ));
    out.push_str(&format!(
        "  with_issues: {}\n",
        checkpoint.summary.with_issues
    ));

    out
}

/// Freshness を文字列に変換
fn freshness_str(f: &Freshness) -> &'static str {
    match f {
        Freshness::Fresh => "fresh",
        Freshness::Stale => "stale",
        Freshness::Unreviewed => "unreviewed",
    }
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "review_context_tests.rs"]
mod context_tests;
