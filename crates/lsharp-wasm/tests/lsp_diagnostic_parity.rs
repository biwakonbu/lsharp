#[path = "e2e/support.rs"]
mod support;

use std::sync::{Mutex, OnceLock};
use support::*;

fn diag_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn run_lsp_harness(harness: &str) -> Vec<String> {
    let _guard = diag_test_lock().lock().expect("diag test lock");
    let source = selfhost_lsp_runtime_bundle();
    let output = compile_and_run(&format!("{}\n{}", source, harness));
    output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect()
}

// ============================================================
// CP-04: diagnostic sort (AC-208/AC-211)
// ============================================================

/// sort-diagnostics: 行番号昇順でソートされること
#[test]
fn test_e2e_lsp_diagnostic_sort_by_line_ascending() {
    // 診断 Vector: [severity, rule-id, line, col, msg-hash, source]
    // diag-a: line=5, col=2  /  diag-b: line=1, col=3
    let harness = r#"
(defn main []
  (let [diag-a (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 100) 5) 2) 0) 0)
        diag-b (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 101) 1) 3) 0) 0)
        diags (vector-push (vector-push (vector-new 2) diag-a) diag-b)
        sorted (sort-diagnostics diags)]
    (do
      (print (vector-get (vector-get sorted 0) 2))
      (print (vector-get (vector-get sorted 1) 2))
      0)))
"#;
    let lines = run_lsp_harness(harness);
    assert_eq!(lines[0], "1", "ソート後の先頭は line=1 であるべき");
    assert_eq!(lines[1], "5", "ソート後の2番目は line=5 であるべき");
}

/// sort-diagnostics: 同一行では col 昇順でソートされること
#[test]
fn test_e2e_lsp_diagnostic_sort_same_line_by_col() {
    // diag-a: line=3, col=10  /  diag-b: line=3, col=2
    let harness = r#"
(defn main []
  (let [diag-a (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 100) 3) 10) 0) 0)
        diag-b (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 101) 3) 2) 0) 0)
        diags (vector-push (vector-push (vector-new 2) diag-a) diag-b)
        sorted (sort-diagnostics diags)]
    (do
      (print (vector-get (vector-get sorted 0) 3))
      (print (vector-get (vector-get sorted 1) 3))
      0)))
"#;
    let lines = run_lsp_harness(harness);
    assert_eq!(lines[0], "2", "同一行ではソート後の先頭は col=2 であるべき");
    assert_eq!(
        lines[1], "10",
        "同一行ではソート後の2番目は col=10 であるべき"
    );
}

/// sort-diagnostics: 3 件の診断を正しくソートできること
#[test]
fn test_e2e_lsp_diagnostic_sort_three_items() {
    // diag-a: line=10  /  diag-b: line=1  /  diag-c: line=5
    let harness = r#"
(defn main []
  (let [diag-a (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 100) 10) 1) 0) 0)
        diag-b (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 101) 1) 1) 0) 0)
        diag-c (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 102) 5) 1) 0) 0)
        diags (vector-push (vector-push (vector-push (vector-new 3) diag-a) diag-b) diag-c)
        sorted (sort-diagnostics diags)]
    (do
      (print (vector-get (vector-get sorted 0) 2))
      (print (vector-get (vector-get sorted 1) 2))
      (print (vector-get (vector-get sorted 2) 2))
      0)))
"#;
    let lines = run_lsp_harness(harness);
    assert_eq!(lines[0], "1", "3件ソート後の1番目は line=1");
    assert_eq!(lines[1], "5", "3件ソート後の2番目は line=5");
    assert_eq!(lines[2], "10", "3件ソート後の3番目は line=10");
}

// ============================================================
// CP-04: diagnostic dedup (AC-209)
// ============================================================

/// merge-duplicate-diagnostics: 同一 span の重複は severity 高い方のみ残すこと
#[test]
fn test_e2e_lsp_diagnostic_dedup_keeps_higher_severity() {
    // dup-a: severity=2, line=5, col=7  /  dup-b: severity=1, line=5, col=7
    let harness = r#"
(defn main []
  (let [dup-a (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 2) 101) 5) 7) 0) 0)
        dup-b (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 102) 5) 7) 0) 0)
        diags (vector-push (vector-push (vector-new 2) dup-a) dup-b)
        merged (merge-duplicate-diagnostics diags)]
    (do
      (print (vector-length merged))
      (print (vector-get (vector-get merged 0) 0))
      0)))
"#;
    let lines = run_lsp_harness(harness);
    assert_eq!(lines[0], "1", "同一 span の重複は 1 件にまとめるべき");
    assert_eq!(
        lines[1], "1",
        "severity が高い方 (数値が小さい=1=Error) を残すべき"
    );
}

/// merge-duplicate-diagnostics: 異なる span は両方残ること
#[test]
fn test_e2e_lsp_diagnostic_dedup_keeps_different_spans() {
    // diag-a: line=1, col=1  /  diag-b: line=5, col=3
    let harness = r#"
(defn main []
  (let [diag-a (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 100) 1) 1) 0) 0)
        diag-b (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 101) 5) 3) 0) 0)
        diags (vector-push (vector-push (vector-new 2) diag-a) diag-b)
        merged (merge-duplicate-diagnostics diags)]
    (do
      (print (vector-length merged))
      0)))
"#;
    let lines = run_lsp_harness(harness);
    assert_eq!(lines[0], "2", "異なる span の診断は両方残るべき");
}

// ============================================================
// CP-04: diagnostic JSON rendering
// ============================================================

/// render-diagnostics-json: 空配列は "[]" を返すこと
#[test]
fn test_e2e_lsp_diagnostic_render_empty() {
    let harness = r#"
(defn main []
  (let [diags (vector-new 0)
        json (render-diagnostics-json diags)]
    (do
      (print-string json)
      0)))
"#;
    let lines = run_lsp_harness(harness);
    assert_eq!(lines[0], "[]", "空診断は [] を返すべき");
}

/// render-diagnostics-json: 1 件の診断を正しく JSON レンダリングすること
#[test]
fn test_e2e_lsp_diagnostic_render_single() {
    // severity=1, rule-id=100, line=3, col=5, msg-hash=0, source=0
    let harness = r#"
(defn main []
  (let [diag (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 100) 3) 5) 0) 0)
        diags (vector-push (vector-new 1) diag)
        json (render-diagnostics-json diags)]
    (do
      (print-string json)
      0)))
"#;
    let lines = run_lsp_harness(harness);
    let json = &lines[0];
    // JSON に severity, rule, line, col が含まれることを確認
    assert!(
        json.contains("\"severity\":1"),
        "severity が含まれるべき: {}",
        json
    );
    assert!(json.contains("\"line\":3"), "line が含まれるべき: {}", json);
    assert!(json.contains("\"col\":5"), "col が含まれるべき: {}", json);
}

// ============================================================
// CP-04: publishDiagnostics frame rendering
// ============================================================

/// lsp-render-publish-diagnostics-frame: URI と診断を含む frame を生成すること
#[test]
fn test_e2e_lsp_diagnostic_publish_frame_contains_uri_and_diagnostics() {
    let harness = r#"
(defn main []
  (let [diag (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 100) 2) 4) 0) 0)
        diags (vector-push (vector-new 1) diag)
        frame-text (lsp-render-publish-diagnostics-frame 42 diags)]
    (do
      (print-string frame-text)
      0)))
"#;
    let lines = run_lsp_harness(harness);
    let output = lines.join("\n");
    assert!(
        output.contains("publishDiagnostics"),
        "frame に publishDiagnostics method が含まれるべき: {}",
        output
    );
    assert!(
        output.contains("\"uri\":42"),
        "frame に uri:42 が含まれるべき: {}",
        output
    );
}
