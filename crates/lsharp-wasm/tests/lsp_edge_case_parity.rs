#[path = "e2e/support.rs"]
mod support;

use std::sync::{Mutex, OnceLock};
use support::*;

fn edge_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn run_lsp_harness(harness: &str) -> Vec<String> {
    let _guard = edge_test_lock().lock().expect("edge test lock");
    let source = selfhost_lsp_runtime_bundle();
    let output = compile_and_run(&format!("{}\n{}", source, harness));
    output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect()
}

/// Content-Length フレームでラップ
fn frame(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

// ============================================================
// CP-04: 空ドキュメントへの hover
// ============================================================

/// 空ドキュメントを open した後に hover → crash しないこと (harness)
#[test]
fn test_e2e_lsp_edge_empty_document_hover_harness() {
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 100 "")
        params (vector-push (vector-push (vector-push (vector-new 3) 100) 1) 1)
        result (handle-hover params state)]
    (do
      (print-string (vector-get result 1))
      0)))
"#;
    let lines = run_lsp_harness(harness);
    // 空ドキュメントでは mock fallback が返る (crash しない)
    assert!(
        !lines.is_empty(),
        "空ドキュメントへの hover は crash せず何らかの応答を返すべき"
    );
}

/// 空ドキュメントを open した後に hover → crash しないこと (stdio)
#[test]
fn test_e2e_lsp_edge_empty_document_hover_stdio() {
    let _guard = edge_test_lock().lock().expect("edge test lock");
    let open_body =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":100,"source":""}}"#;
    let hover_body = r#"{"jsonrpc":"2.0","id":90,"method":"textDocument/hover","params":{"uri":100,"line":1,"col":1}}"#;
    let stdin = format!("{}{}", frame(open_body), frame(hover_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    // crash せずレスポンスが返ること
    assert!(
        output.contains("jsonrpc"),
        "空ドキュメント hover で jsonrpc レスポンスが返るべき: {}",
        output
    );
}

// ============================================================
// CP-04: 範囲外ポジションへの hover
// ============================================================

/// 範囲外ポジション (line=999) に hover → crash しないこと (harness)
#[test]
fn test_e2e_lsp_edge_out_of_bounds_hover_harness() {
    let source = "(defn add [x y] (+ x y))";
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 100 "{source}")
        params (vector-push (vector-push (vector-push (vector-new 3) 100) 999) 999)
        result (handle-hover params state)]
    (do
      (print-string (vector-get result 1))
      0)))
"#
    );
    let lines = run_lsp_harness(&harness);
    assert!(
        !lines.is_empty(),
        "範囲外ポジションへの hover は crash せず応答すべき"
    );
}

/// 範囲外ポジション (line=999) に hover → crash しないこと (stdio)
#[test]
fn test_e2e_lsp_edge_out_of_bounds_hover_stdio() {
    let _guard = edge_test_lock().lock().expect("edge test lock");
    let source = "(defn add [x y] (+ x y))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":100,"source":"{}"}}}}"#,
        source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":91,"method":"textDocument/hover","params":{"uri":100,"line":999,"col":999}}"#;
    let stdin = format!("{}{}", frame(&open_body), frame(hover_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert!(
        output.contains("jsonrpc"),
        "範囲外 hover で crash せずレスポンスが返るべき: {}",
        output
    );
}

// ============================================================
// CP-04: didChange 後の state 整合性
// ============================================================

/// didOpen → didChange (別内容) → hover が最新ソースを反映 (harness)
#[test]
fn test_e2e_lsp_edge_change_then_hover_reflects_latest_harness() {
    let initial = "(defn old-func [] 1)";
    let updated = "(defn new-func [] 2)";
    let col = updated.find("new-func").expect("new-func") + 1;
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 100 "{initial}")
        _ (server-state-open-document state 100 "{updated}")
        params (vector-push (vector-push (vector-push (vector-new 3) 100) 1) {col})
        result (handle-hover params state)]
    (do
      (print-string (vector-get result 1))
      0)))
"#
    );
    let lines = run_lsp_harness(&harness);
    assert!(
        lines.iter().any(|l| l.contains("new-func")),
        "didChange 後の hover は最新の new-func を反映すべき: {:?}",
        lines
    );
}

/// didOpen → didChange → hover が最新ソースを反映 (stdio)
#[test]
fn test_e2e_lsp_edge_change_then_hover_reflects_latest_stdio() {
    let _guard = edge_test_lock().lock().expect("edge test lock");
    let initial = "(defn old-func [] 1)";
    let updated = "(defn new-func [] 2)";
    let col = updated.find("new-func").expect("new-func") + 1;

    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":100,"source":"{}"}}}}"#,
        initial
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":100,"source":"{}"}}}}"#,
        updated
    );
    let hover_body = format!(
        r#"{{"jsonrpc":"2.0","id":92,"method":"textDocument/hover","params":{{"uri":100,"line":1,"col":{col}}}}}"#,
    );
    let stdin = format!(
        "{}{}{}",
        frame(&open_body),
        frame(&change_body),
        frame(&hover_body)
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert!(
        output.contains("new-func"),
        "didChange 後 hover は new-func を反映すべき: {}",
        output
    );
}

// ============================================================
// CP-04: 空ドキュメントへの completion
// ============================================================

/// 空ドキュメントへの completion → キーワード候補を返すこと (harness)
#[test]
fn test_e2e_lsp_edge_empty_document_completion_harness() {
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 100 "")
        params (vector-push (vector-push (vector-push (vector-new 3) 100) 1) 1)
        items (handle-completion params state)]
    (do
      (print (vector-length items))
      0)))
"#;
    let lines = run_lsp_harness(harness);
    let count: usize = lines[0].parse().expect("completion count");
    // 空ドキュメントでもキーワード候補 (defn, let, if, match, do, fn, module) が返る
    assert!(
        count >= 7,
        "空ドキュメントの completion は 7 キーワード候補を返すべき: {}",
        count
    );
}

// ============================================================
// CP-04: 空ドキュメントへの goto_definition
// ============================================================

/// 空ドキュメントへの goto_definition → crash しないこと (harness)
#[test]
fn test_e2e_lsp_edge_empty_document_definition_harness() {
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 100 "")
        params (vector-push (vector-push (vector-push (vector-new 3) 100) 1) 1)
        result (handle-goto-definition params state)]
    (do
      (print (vector-get result 0))
      0)))
"#;
    let lines = run_lsp_harness(harness);
    assert!(
        !lines.is_empty(),
        "空ドキュメントへの goto_definition は crash せず応答すべき"
    );
}

// ============================================================
// CP-04: 空ドキュメントへの formatting
// ============================================================

/// 空ドキュメントへの formatting → crash しないこと (harness)
#[test]
fn test_e2e_lsp_edge_empty_document_formatting_harness() {
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 100 "")
        params (vector-push (vector-new 1) 100)
        result (handle-formatting params state)]
    (do
      (print (vector-length result))
      0)))
"#;
    let lines = run_lsp_harness(harness);
    // 空ドキュメントでは mock fallback (1 要素) が返る
    assert!(
        !lines.is_empty(),
        "空ドキュメントへの formatting は crash せず応答すべき"
    );
}
