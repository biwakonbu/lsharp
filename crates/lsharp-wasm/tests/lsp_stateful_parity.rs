#[path = "e2e/support.rs"]
mod support;

use support::*;
use std::sync::{Mutex, OnceLock};

fn parity_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn run_lsp_harness(harness: &str) -> Vec<String> {
    let _guard = parity_test_lock().lock().expect("parity test lock");
    let source = selfhost_lsp_runtime_bundle();
    let output = compile_and_run(&format!("{}\n{}", source, harness));
    output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect()
}

/// CP-04: stateful session 上で hier fixture 同形の dotted import を
/// open 済み別 document から completion 候補に引けること
#[test]
fn test_e2e_lsp_stateful_completion_resolves_hier_fixture_shape_open_document() {
    let helper_source = "(module Syntax.SimpleHelper) (defn helper-value [] 42)";
    let main_source = "(module App.Main) (import Syntax.SimpleHelper) (defn main [] (helper-val))";
    let completion_col = main_source.find("helper-val").expect("helper-val") + "helper-val".len() + 1;
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 201 "{helper_source}")
        _ (server-state-open-document state 200 "{main_source}")
        params (vector-push (vector-push (vector-push (vector-new 3) 200) 1) {completion_col})
        items (handle-completion params state)]
    (do
      (print (vector-length items))
      (if (> (vector-length items) 0)
        (print-string (vector-get (vector-get items 0) 0))
        (print-string "none"))
      0)))
"#
    );

    let lines = run_lsp_harness(&harness);

    assert_eq!(lines[0], "1", "stateful completion は cross-document 候補を 1 件返すべき");
    assert_eq!(
        lines[1], "helper-value",
        "stateful completion は helper document の defn 名を候補に出すべき"
    );
}

// === 共通フィクスチャ ===

const HELPER_SOURCE: &str = "(module Syntax.SimpleHelper) (defn helper-value [] 42)";
const MAIN_SOURCE: &str =
    "(module App.Main) (import Syntax.SimpleHelper) (defn main [] (helper-value))";

/// didOpen JSON-RPC リクエストを生成
fn make_did_open(uri: u32, source: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":{},"source":"{}"}}}}"#,
        uri, source
    )
}

/// didOpen レスポンスを生成
fn make_did_open_response(uri: u32, source: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":{},"sourceBytes":{}}}}}"#,
        uri,
        source.len()
    )
}

/// Content-Length フレームでラップ
fn frame(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

/// helper + main を open した stdio リクエストの共通 prefix を生成
fn stdio_open_prefix() -> (String, String) {
    let open_helper = make_did_open(201, HELPER_SOURCE);
    let open_main = make_did_open(200, MAIN_SOURCE);
    let stdin_prefix = format!("{}{}", frame(&open_helper), frame(&open_main));

    let resp_helper = make_did_open_response(201, HELPER_SOURCE);
    let resp_main = make_did_open_response(200, MAIN_SOURCE);
    let expected_prefix = format!("{}{}", frame(&resp_helper), frame(&resp_main));

    (stdin_prefix, expected_prefix)
}

/// helper + main を open した harness の共通 prefix を生成
fn harness_open_prefix() -> String {
    format!(
        r#"
        (let [state (server-state-new)
              _ (server-state-open-document state 201 "{}")
              _ (server-state-open-document state 200 "{}")]
"#,
        HELPER_SOURCE, MAIN_SOURCE
    )
}

// ============================================================
// CP-04: hover cross-document parity
// ============================================================

/// CP-04: stateful harness 上で cross-document hover が
/// helper document の defn 名を contents に返すこと
#[test]
fn test_e2e_lsp_stateful_hover_resolves_cross_document() {
    // main source 内の "helper-value" の位置を計算
    let col = MAIN_SOURCE
        .find("(helper-value)")
        .expect("helper-value call")
        + 2; // '(' の次
    let harness = format!(
        r#"
(defn main []
  {}
          (let [params (vector-push (vector-push (vector-push (vector-new 3) 200) 1) {col})
                result (handle-hover params state)]
            (do
              (print-string (vector-get result 1))
              0))))
"#,
        harness_open_prefix(),
        col = col
    );

    let lines = run_lsp_harness(&harness);
    assert!(
        lines.iter().any(|l| l.contains("helper-value")),
        "hover contents に helper-value が含まれるべき: {:?}",
        lines
    );
}

/// CP-04: actual `lsp --stdio` でも cross-document hover が
/// helper document の defn 名を contents に返すこと
#[test]
fn test_e2e_lsp_actual_stdio_hover_resolves_cross_document() {
    let _guard = parity_test_lock().lock().expect("parity test lock");
    let col = MAIN_SOURCE
        .find("(helper-value)")
        .expect("helper-value call")
        + 2;
    let (stdin_prefix, expected_prefix) = stdio_open_prefix();

    let hover_body = format!(
        r#"{{"jsonrpc":"2.0","id":80,"method":"textDocument/hover","params":{{"uri":200,"line":1,"col":{col}}}}}"#
    );
    let stdin = format!("{}{}", stdin_prefix, frame(&hover_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    // レスポンスが helper-value の hover contents を含むことを確認
    assert!(
        output.starts_with(&expected_prefix),
        "open response prefix が一致すべき"
    );
    let hover_part = &output[expected_prefix.len()..];
    assert!(
        hover_part.contains("helper-value"),
        "hover response に helper-value が含まれるべき: {}",
        hover_part
    );
}

// ============================================================
// CP-04: goto_definition cross-document parity
// ============================================================

/// CP-04: stateful harness 上で cross-document goto_definition が
/// helper document の URI と定義位置を返すこと
#[test]
fn test_e2e_lsp_stateful_goto_definition_resolves_cross_document() {
    let col = MAIN_SOURCE
        .find("(helper-value)")
        .expect("helper-value call")
        + 2;
    let harness = format!(
        r#"
(defn main []
  {}
          (let [params (vector-push (vector-push (vector-push (vector-new 3) 200) 1) {col})
                result (handle-goto-definition params state)]
            (do
              (print (vector-get result 0))
              0))))
"#,
        harness_open_prefix(),
        col = col
    );

    let lines = run_lsp_harness(&harness);
    // goto_definition は helper document の URI (201) を返すべき
    assert_eq!(
        lines[0], "201",
        "goto_definition は helper document URI 201 を返すべき"
    );
}

/// CP-04: actual `lsp --stdio` でも cross-document goto_definition が
/// helper document の定義位置を返すこと
#[test]
fn test_e2e_lsp_actual_stdio_goto_definition_resolves_cross_document() {
    let _guard = parity_test_lock().lock().expect("parity test lock");
    let col = MAIN_SOURCE
        .find("(helper-value)")
        .expect("helper-value call")
        + 2;
    let (stdin_prefix, expected_prefix) = stdio_open_prefix();

    let def_body = format!(
        r#"{{"jsonrpc":"2.0","id":81,"method":"textDocument/definition","params":{{"uri":200,"line":1,"col":{col}}}}}"#
    );
    let stdin = format!("{}{}", stdin_prefix, frame(&def_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert!(
        output.starts_with(&expected_prefix),
        "open response prefix が一致すべき"
    );
    let def_part = &output[expected_prefix.len()..];
    // 定義位置は helper document (URI 201) にあるべき
    assert!(
        def_part.contains("201"),
        "definition response に helper URI 201 が含まれるべき: {}",
        def_part
    );
}

// ============================================================
// CP-04: references cross-document parity
// ============================================================

/// CP-04: stateful harness 上で references が
/// 現在ドキュメント内のシンボル出現箇所を返すこと
#[test]
fn test_e2e_lsp_stateful_references_finds_occurrences() {
    let col = MAIN_SOURCE
        .find("(helper-value)")
        .expect("helper-value call")
        + 2;
    let harness = format!(
        r#"
(defn main []
  {}
          (let [params (vector-push (vector-push (vector-push (vector-new 3) 200) 1) {col})
                result (handle-references params state)]
            (do
              (print (vector-length result))
              0))))
"#,
        harness_open_prefix(),
        col = col
    );

    let lines = run_lsp_harness(&harness);
    let count: usize = lines[0].parse().expect("references count");
    assert!(
        count >= 1,
        "references は少なくとも 1 箇所を返すべき: {}",
        count
    );
}

/// CP-04: actual `lsp --stdio` でも references が出現箇所を返すこと
#[test]
fn test_e2e_lsp_actual_stdio_references_finds_occurrences() {
    let _guard = parity_test_lock().lock().expect("parity test lock");
    let col = MAIN_SOURCE
        .find("(helper-value)")
        .expect("helper-value call")
        + 2;
    let (stdin_prefix, expected_prefix) = stdio_open_prefix();

    let ref_body = format!(
        r#"{{"jsonrpc":"2.0","id":82,"method":"textDocument/references","params":{{"uri":200,"line":1,"col":{col}}}}}"#
    );
    let stdin = format!("{}{}", stdin_prefix, frame(&ref_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert!(
        output.starts_with(&expected_prefix),
        "open response prefix が一致すべき"
    );
    let ref_part = &output[expected_prefix.len()..];
    assert!(
        ref_part.contains("result"),
        "references response に result が含まれるべき: {}",
        ref_part
    );
}

// ============================================================
// CP-04: rename parity
// ============================================================

/// CP-04: stateful harness 上で rename が workspace edit を返すこと
#[test]
fn test_e2e_lsp_stateful_rename_returns_workspace_edit() {
    let col = MAIN_SOURCE
        .find("(helper-value)")
        .expect("helper-value call")
        + 2;
    // rename params: [uri, line, col, "", new-name] (index 4 = new-name)
    let harness = format!(
        r#"
(defn main []
  {}
          (let [params (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 200) 1) {col}) "") "helper-val2")
                result (handle-rename params state)]
            (do
              (print (vector-length result))
              0))))
"#,
        harness_open_prefix(),
        col = col
    );

    let lines = run_lsp_harness(&harness);
    let count: usize = lines[0].parse().expect("rename changes count");
    assert!(
        count >= 1,
        "rename は workspace edit を返すべき: {}",
        count
    );
}

/// CP-04: actual `lsp --stdio` でも rename が workspace edit を返すこと
#[test]
fn test_e2e_lsp_actual_stdio_rename_returns_workspace_edit() {
    let _guard = parity_test_lock().lock().expect("parity test lock");
    let col = MAIN_SOURCE
        .find("(helper-value)")
        .expect("helper-value call")
        + 2;
    let (stdin_prefix, expected_prefix) = stdio_open_prefix();

    let rename_body = format!(
        r#"{{"jsonrpc":"2.0","id":83,"method":"textDocument/rename","params":{{"uri":200,"line":1,"col":{col},"newName":"helper-val2"}}}}"#
    );
    let stdin = format!("{}{}", stdin_prefix, frame(&rename_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert!(
        output.starts_with(&expected_prefix),
        "open response prefix が一致すべき"
    );
    let rename_part = &output[expected_prefix.len()..];
    assert!(
        rename_part.contains("result"),
        "rename response に result が含まれるべき: {}",
        rename_part
    );
}

// ============================================================
// CP-04: formatting parity
// ============================================================

/// CP-04: stateful harness 上で formatting が TextEdit を返すこと
#[test]
fn test_e2e_lsp_stateful_formatting_returns_text_edit() {
    let harness = format!(
        r#"
(defn main []
  {}
          (let [params (vector-push (vector-new 1) 200)
                result (handle-formatting params state)]
            (do
              (print (vector-length result))
              0))))
"#,
        harness_open_prefix()
    );

    let lines = run_lsp_harness(&harness);
    let count: usize = lines[0].parse().expect("formatting edits count");
    assert!(
        count >= 1,
        "formatting は少なくとも 1 つの TextEdit を返すべき: {}",
        count
    );
}

/// CP-04: actual `lsp --stdio` でも formatting が TextEdit を返すこと
#[test]
fn test_e2e_lsp_actual_stdio_formatting_returns_text_edit() {
    let _guard = parity_test_lock().lock().expect("parity test lock");
    let (stdin_prefix, expected_prefix) = stdio_open_prefix();

    let fmt_body =
        r#"{"jsonrpc":"2.0","id":84,"method":"textDocument/formatting","params":{"uri":200}}"#;
    let stdin = format!("{}{}", stdin_prefix, frame(fmt_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert!(
        output.starts_with(&expected_prefix),
        "open response prefix が一致すべき"
    );
    let fmt_part = &output[expected_prefix.len()..];
    assert!(
        fmt_part.contains("result"),
        "formatting response に result が含まれるべき: {}",
        fmt_part
    );
}

// ============================================================
// CP-04: completion cross-document parity (既存)
// ============================================================

/// CP-04: actual `lsp --stdio` でも hier fixture 同形の dotted import を
/// open 済み別 document から completion 候補に引けること
#[test]
fn test_e2e_lsp_actual_stdio_completion_resolves_hier_fixture_shape_open_document() {
    let _guard = parity_test_lock().lock().expect("parity test lock");
    let helper_source = "(module Syntax.SimpleHelper) (defn helper-value [] 42)";
    let main_source = "(module App.Main) (import Syntax.SimpleHelper) (defn main [] (helper-val))";
    let completion_col = main_source.find("helper-val").expect("helper-val") + "helper-val".len() + 1;
    let open_helper_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":201,"source":"{}"}}}}"#,
        helper_source
    );
    let open_main_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":200,"source":"{}"}}}}"#,
        main_source
    );
    let completion_body = format!(
        r#"{{"jsonrpc":"2.0","id":90,"method":"textDocument/completion","params":{{"uri":200,"line":1,"col":{completion_col}}}}}"#
    );
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_body.len(),
        open_helper_body,
        open_main_body.len(),
        open_main_body,
        completion_body.len(),
        completion_body
    );
    let open_helper_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":201,"sourceBytes":{}}}}}"#,
        helper_source.len()
    );
    let open_main_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":200,"sourceBytes":{}}}}}"#,
        main_source.len()
    );
    let completion_response =
        r#"{"jsonrpc":"2.0","id":90,"result":[["helper-value",3,"helper-value"]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_response.len(),
        open_helper_response,
        open_main_response.len(),
        open_main_response,
        completion_response.len(),
        completion_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "actual lsp --stdio は hier fixture 同形の dotted import でも cross-document completion を返すべき"
    );
}
