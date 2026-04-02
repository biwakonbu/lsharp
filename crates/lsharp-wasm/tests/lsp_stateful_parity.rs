#[path = "e2e/support.rs"]
mod support;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use support::*;

fn parity_test_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

static FILESYSTEM_LSP_FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);
const FILESYSTEM_LSP_RUNTIME_MODULES: &[&str] = &[
    "Token.ls",
    "AST.ls",
    "Lexer.ls",
    "Parser.ls",
    "ModuleResolver.ls",
    "FormatterExpr.ls",
    "FormatterDecl.ls",
    "Formatter.ls",
    "Linter.ls",
    "JsonRpc.ls",
    "LspServerCore.ls",
    "LspServerNav.ls",
    "LspServer.ls",
];

fn filesystem_lsp_fixture_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lsharp_test_lsp_stateful_{}_{}_{}",
        prefix,
        std::process::id(),
        FILESYSTEM_LSP_FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn write_filesystem_lsp_fixture_files(dir: &Path, files: &[(&str, &str)]) {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).expect("filesystem fixture directory の作成に失敗");
    for (relative, source) in files {
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                panic!(
                    "filesystem fixture parent の作成に失敗 {}: {}",
                    parent.display(),
                    e
                )
            });
        }
        std::fs::write(&path, source).unwrap_or_else(|e| {
            panic!(
                "filesystem fixture file の書き込みに失敗 {}: {}",
                path.display(),
                e
            )
        });
    }
}

fn filesystem_nested_fixture_files() -> [(&'static str, &'static str); 3] {
    [
        (
            "src/Support/Base.ls",
            "(module Support.Base) (defn base-val [] 10)",
        ),
        (
            "src/Support/Mid.ls",
            "(module Support.Mid) (import Support.Base) (defn mid-val [] (base-val))",
        ),
        (
            "src/Main.ls",
            "(module Main) (import Support.Mid) (defn main [] (mid-val))",
        ),
    ]
}

fn lsharp_name_hash(text: &str) -> i64 {
    text.chars().fold(0_i64, |acc, ch| {
        acc.wrapping_mul(31).wrapping_add(i64::from(u32::from(ch)))
    })
}

fn run_lsp_harness(harness: &str) -> Vec<String> {
    let _guard = parity_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let source = selfhost_lsp_runtime_bundle();
    let output = compile_and_run(&format!("{}\n{}", source, harness));
    output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect()
}

fn write_filesystem_lsp_runtime_modules(dir: &Path) {
    for name in FILESYSTEM_LSP_RUNTIME_MODULES {
        let path = dir
            .join("src")
            .join(selfhost_fixture_module_relative_path(name));
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                panic!(
                    "filesystem lsp runtime parent の作成に失敗 {}: {}",
                    parent.display(),
                    e
                )
            });
        }
        std::fs::write(&path, selfhost_module(name)).unwrap_or_else(|e| {
            panic!(
                "filesystem lsp runtime module の書き込みに失敗 {}: {}",
                path.display(),
                e
            )
        });
    }
}

fn run_lsp_harness_with_dir(harness: &str, dir: &Path) -> Vec<String> {
    let _guard = parity_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    write_filesystem_lsp_runtime_modules(dir);
    let entry_path = dir.join("src/App/Main.ls");
    if let Some(parent) = entry_path.parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| {
            panic!(
                "filesystem lsp harness entry parent の作成に失敗 {}: {}",
                parent.display(),
                e
            )
        });
    }
    let entry_source = format!(
        "(module App.Main)\n(import Tools.Lsp.LspServerCore)\n(import Tools.Lsp.LspServerNav)\n(import Tools.Lsp.LspServer)\n{}",
        harness
    );
    std::fs::write(&entry_path, entry_source).unwrap_or_else(|e| {
        panic!(
            "filesystem lsp harness entry の書き込みに失敗 {}: {}",
            entry_path.display(),
            e
        )
    });
    let wasm = compile_file_only(&entry_path);
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir(&wasm, Some(dir))
        .expect("filesystem lsp harness 実行に失敗");
    output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect()
}

fn run_lsp_stdio_with_dir(stdin: &str, dir: &Path) -> String {
    let _guard = parity_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let wasm = compile_only(selfhost_cli_runtime_bundle());
    lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin(
        &wasm,
        Some(dir),
        &["lsp", "--stdio"],
        stdin,
    )
    .expect("filesystem-backed lsp stdio 実行に失敗")
}

fn parity_test_guard() -> std::sync::MutexGuard<'static, ()> {
    parity_test_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// CP-04: stateful session 上で hier fixture 同形の dotted import を
/// open 済み別 document から completion 候補に引けること
#[test]
fn test_e2e_lsp_stateful_completion_resolves_hier_fixture_shape_open_document() {
    let helper_source = "(module Syntax.SimpleHelper) (defn helper-value [] 42)";
    let main_source = "(module App.Main) (import Syntax.SimpleHelper) (defn main [] (helper-val))";
    let completion_col =
        main_source.find("helper-val").expect("helper-val") + "helper-val".len() + 1;
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

    assert_eq!(
        lines[0], "1",
        "stateful completion は cross-document 候補を 1 件返すべき"
    );
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

fn make_did_open_with_path(uri: u32, path: &str, source: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":{},"path":"{}","source":"{}"}}}}"#,
        uri, path, source
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
    let _guard = parity_test_guard();
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
    let _guard = parity_test_guard();
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
    let _guard = parity_test_guard();
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
    assert!(count >= 1, "rename は workspace edit を返すべき: {}", count);
}

/// CP-04: actual `lsp --stdio` でも rename が workspace edit を返すこと
#[test]
fn test_e2e_lsp_actual_stdio_rename_returns_workspace_edit() {
    let _guard = parity_test_guard();
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
    let _guard = parity_test_guard();
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
    let _guard = parity_test_guard();
    let helper_source = "(module Syntax.SimpleHelper) (defn helper-value [] 42)";
    let main_source = "(module App.Main) (import Syntax.SimpleHelper) (defn main [] (helper-val))";
    let completion_col =
        main_source.find("helper-val").expect("helper-val") + "helper-val".len() + 1;
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
    let expected_prefix = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_response.len(),
        open_helper_response,
        open_main_response.len(),
        open_main_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert!(
        output.starts_with(&expected_prefix),
        "actual lsp --stdio は didOpen response prefix を保つべき: {}",
        output
    );
    let completion_part = &output[expected_prefix.len()..];
    assert!(
        completion_part.contains("helper-value"),
        "actual lsp --stdio は hier fixture 同形の dotted import でも cross-document completion を返すべき: {}",
        completion_part
    );
}

/// CP-04: stateful harness 上で open 済み document の path を起点に
/// filesystem import 先の hover を解決できること
#[test]
fn test_e2e_lsp_stateful_hover_resolves_filesystem_import_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("hover_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let hover_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let harness = format!(
        r#"
(defn make-doc-open-params [uri src path]
  (vector-push (vector-push (vector-push (vector-new 3) uri) src) path))

(defn main []
  (let [state (server-state-new)
        _ (handle-didOpen (make-doc-open-params 200 "{main_source}" "src/Main.ls") state)
        params (vector-push (vector-push (vector-push (vector-new 3) 200) 1) {hover_col})
        hover (handle-hover params state)]
    (do
      (print-string (vector-get hover 1))
      0)))
"#
    );

    let lines = run_lsp_harness_with_dir(&harness, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        lines[0], "defn mid-val",
        "document path 付き didOpen は filesystem import 先の hover を返すべき"
    );
}

/// CP-04: stateful harness 上で nested import も document path を起点に
/// filesystem から辿って hover を解決できること
#[test]
fn test_e2e_lsp_stateful_hover_resolves_nested_filesystem_import_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("hover_nested_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let mid_source = "(module Support.Mid) (import Support.Base) (defn mid-val [] (base-val))";
    let hover_col = mid_source.find("(base-val)").expect("base-val call") + 2;
    let harness = format!(
        r#"
(defn make-doc-open-params [uri src path]
  (vector-push (vector-push (vector-push (vector-new 3) uri) src) path))

(defn main []
  (let [state (server-state-new)
        _ (handle-didOpen (make-doc-open-params 201 "{mid_source}" "src/Support/Mid.ls") state)
        params (vector-push (vector-push (vector-push (vector-new 3) 201) 1) {hover_col})
        hover (handle-hover params state)]
    (do
      (print-string (vector-get hover 1))
      0)))
"#
    );

    let lines = run_lsp_harness_with_dir(&harness, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        lines[0], "defn base-val",
        "document path 付き didOpen は nested filesystem import 先の hover も返すべき"
    );
}

/// CP-04: actual `lsp --stdio` でも document path を起点に
/// filesystem import 先の hover を解決できること
#[test]
fn test_e2e_lsp_actual_stdio_hover_resolves_filesystem_import_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("stdio_hover_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let hover_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let open_main_body = make_did_open_with_path(200, "src/Main.ls", main_source);
    let hover_body = format!(
        r#"{{"jsonrpc":"2.0","id":91,"method":"textDocument/hover","params":{{"uri":200,"line":1,"col":{hover_col}}}}}"#
    );
    let stdin = format!("{}{}", frame(&open_main_body), frame(&hover_body));
    let open_main_response = make_did_open_response(200, main_source);
    let expected_prefix = frame(&open_main_response);

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.starts_with(&expected_prefix),
        "document path 付き didOpen response prefix が一致すべき: {}",
        output
    );
    let hover_part = &output[expected_prefix.len()..];
    assert!(
        hover_part.contains("mid-val"),
        "actual lsp --stdio は filesystem import 先の hover contents を返すべき: {}",
        hover_part
    );
}

/// CP-04: actual `lsp --stdio` でも nested import を document path から
/// filesystem で辿って hover を解決できること
#[test]
fn test_e2e_lsp_actual_stdio_hover_resolves_nested_filesystem_import_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("stdio_hover_nested_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let mid_source = "(module Support.Mid) (import Support.Base) (defn mid-val [] (base-val))";
    let hover_col = mid_source.find("(base-val)").expect("base-val call") + 2;
    let open_mid_body = make_did_open_with_path(201, "src/Support/Mid.ls", mid_source);
    let hover_body = format!(
        r#"{{"jsonrpc":"2.0","id":92,"method":"textDocument/hover","params":{{"uri":201,"line":1,"col":{hover_col}}}}}"#
    );
    let stdin = format!("{}{}", frame(&open_mid_body), frame(&hover_body));
    let open_mid_response = make_did_open_response(201, mid_source);
    let expected_prefix = frame(&open_mid_response);

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.starts_with(&expected_prefix),
        "nested document path 付き didOpen response prefix が一致すべき: {}",
        output
    );
    let hover_part = &output[expected_prefix.len()..];
    assert!(
        hover_part.contains("base-val"),
        "actual lsp --stdio は nested filesystem import 先の hover contents を返すべき: {}",
        hover_part
    );
}

/// CP-04: stateful harness 上で open 済み document の path を起点に
/// filesystem import 先の goto_definition を解決できること
#[test]
fn test_e2e_lsp_stateful_goto_definition_resolves_filesystem_import_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("definition_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let definition_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let harness = format!(
        r#"
(defn make-doc-open-params [uri src path]
  (vector-push (vector-push (vector-push (vector-new 3) uri) src) path))

(defn main []
  (let [state (server-state-new)
        _ (handle-didOpen (make-doc-open-params 200 "{main_source}" "src/Main.ls") state)
        params (vector-push (vector-push (vector-push (vector-new 3) 200) 1) {definition_col})
        location (handle-goto-definition params state)]
    (do
      (print (vector-get location 0))
      0)))
"#
    );

    let lines = run_lsp_harness_with_dir(&harness, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        lines[0],
        lsharp_name_hash("src/Support/Mid.ls").to_string(),
        "document path 付き didOpen は filesystem import 先の virtual URI を返すべき"
    );
}

/// CP-04: stateful harness 上で nested import も document path を起点に
/// filesystem から辿って goto_definition を解決できること
#[test]
fn test_e2e_lsp_stateful_goto_definition_resolves_nested_filesystem_import_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("definition_nested_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let mid_source = "(module Support.Mid) (import Support.Base) (defn mid-val [] (base-val))";
    let definition_col = mid_source.find("(base-val)").expect("base-val call") + 2;
    let harness = format!(
        r#"
(defn make-doc-open-params [uri src path]
  (vector-push (vector-push (vector-push (vector-new 3) uri) src) path))

(defn main []
  (let [state (server-state-new)
        _ (handle-didOpen (make-doc-open-params 201 "{mid_source}" "src/Support/Mid.ls") state)
        params (vector-push (vector-push (vector-push (vector-new 3) 201) 1) {definition_col})
        location (handle-goto-definition params state)]
    (do
      (print (vector-get location 0))
      0)))
"#
    );

    let lines = run_lsp_harness_with_dir(&harness, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        lines[0],
        lsharp_name_hash("src/Support/Base.ls").to_string(),
        "document path 付き didOpen は nested filesystem import 先の virtual URI を返すべき"
    );
}

/// CP-04: actual `lsp --stdio` でも document path を起点に
/// filesystem import 先の goto_definition を解決できること
#[test]
fn test_e2e_lsp_actual_stdio_goto_definition_resolves_filesystem_import_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("stdio_definition_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let definition_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let open_main_body = make_did_open_with_path(200, "src/Main.ls", main_source);
    let definition_body = format!(
        r#"{{"jsonrpc":"2.0","id":95,"method":"textDocument/definition","params":{{"uri":200,"line":1,"col":{definition_col}}}}}"#
    );
    let stdin = format!("{}{}", frame(&open_main_body), frame(&definition_body));
    let open_main_response = make_did_open_response(200, main_source);
    let expected_prefix = frame(&open_main_response);

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.starts_with(&expected_prefix),
        "document path 付き didOpen response prefix が一致すべき: {}",
        output
    );
    let definition_part = &output[expected_prefix.len()..];
    assert!(
        definition_part.contains(&lsharp_name_hash("src/Support/Mid.ls").to_string()),
        "actual lsp --stdio は filesystem import 先の virtual URI を返すべき: {}",
        definition_part
    );
}

/// CP-04: actual `lsp --stdio` でも nested import を document path から
/// filesystem で辿って goto_definition を解決できること
#[test]
fn test_e2e_lsp_actual_stdio_goto_definition_resolves_nested_filesystem_import_from_document_path()
{
    let dir = filesystem_lsp_fixture_dir("stdio_definition_nested_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let mid_source = "(module Support.Mid) (import Support.Base) (defn mid-val [] (base-val))";
    let definition_col = mid_source.find("(base-val)").expect("base-val call") + 2;
    let open_mid_body = make_did_open_with_path(201, "src/Support/Mid.ls", mid_source);
    let definition_body = format!(
        r#"{{"jsonrpc":"2.0","id":96,"method":"textDocument/definition","params":{{"uri":201,"line":1,"col":{definition_col}}}}}"#
    );
    let stdin = format!("{}{}", frame(&open_mid_body), frame(&definition_body));
    let open_mid_response = make_did_open_response(201, mid_source);
    let expected_prefix = frame(&open_mid_response);

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.starts_with(&expected_prefix),
        "nested document path 付き didOpen response prefix が一致すべき: {}",
        output
    );
    let definition_part = &output[expected_prefix.len()..];
    assert!(
        definition_part.contains(&lsharp_name_hash("src/Support/Base.ls").to_string()),
        "actual lsp --stdio は nested filesystem import 先の virtual URI を返すべき: {}",
        definition_part
    );
}

/// CP-04: stateful harness 上で open 済み document の path を起点に
/// filesystem import 先の references を解決できること
#[test]
fn test_e2e_lsp_stateful_references_find_filesystem_import_occurrences_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("references_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let references_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let harness = format!(
        r#"
(defn make-doc-open-params [uri src path]
  (vector-push (vector-push (vector-push (vector-new 3) uri) src) path))

(defn main []
  (let [state (server-state-new)
        _ (handle-didOpen (make-doc-open-params 200 "{main_source}" "src/Main.ls") state)
        params (vector-push (vector-push (vector-push (vector-new 3) 200) 1) {references_col})
        locations (handle-references params state)]
    (do
      (print (vector-length locations))
      (print (if (> (vector-length locations) 1) (vector-get (vector-get locations 1) 0) (- 0 1)))
      0)))
"#
    );

    let lines = run_lsp_harness_with_dir(&harness, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        lines[0], "2",
        "document path 付き didOpen は filesystem import 先の references を 2 件返すべき"
    );
    assert_eq!(
        lines[1],
        lsharp_name_hash("src/Support/Mid.ls").to_string(),
        "document path 付き didOpen は filesystem import 先の virtual URI を references に含めるべき"
    );
}

/// CP-04: stateful harness 上で nested import も document path を起点に
/// filesystem から辿って references を解決できること
#[test]
fn test_e2e_lsp_stateful_references_find_nested_filesystem_import_occurrences_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("references_nested_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let mid_source = "(module Support.Mid) (import Support.Base) (defn mid-val [] (base-val))";
    let references_col = mid_source.find("(base-val)").expect("base-val call") + 2;
    let harness = format!(
        r#"
(defn make-doc-open-params [uri src path]
  (vector-push (vector-push (vector-push (vector-new 3) uri) src) path))

(defn main []
  (let [state (server-state-new)
        _ (handle-didOpen (make-doc-open-params 201 "{mid_source}" "src/Support/Mid.ls") state)
        params (vector-push (vector-push (vector-push (vector-new 3) 201) 1) {references_col})
        locations (handle-references params state)]
    (do
      (print (vector-length locations))
      (print (if (> (vector-length locations) 1) (vector-get (vector-get locations 1) 0) (- 0 1)))
      0)))
"#
    );

    let lines = run_lsp_harness_with_dir(&harness, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        lines[0], "2",
        "document path 付き didOpen は nested filesystem import 先の references を 2 件返すべき"
    );
    assert_eq!(
        lines[1],
        lsharp_name_hash("src/Support/Base.ls").to_string(),
        "document path 付き didOpen は nested filesystem import 先の virtual URI を references に含めるべき"
    );
}

/// CP-04: actual `lsp --stdio` でも document path を起点に
/// filesystem import 先の references を解決できること
#[test]
fn test_e2e_lsp_actual_stdio_references_find_filesystem_import_occurrences_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("stdio_references_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let references_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let open_main_body = make_did_open_with_path(200, "src/Main.ls", main_source);
    let references_body = format!(
        r#"{{"jsonrpc":"2.0","id":97,"method":"textDocument/references","params":{{"uri":200,"line":1,"col":{references_col}}}}}"#
    );
    let stdin = format!("{}{}", frame(&open_main_body), frame(&references_body));
    let open_main_response = make_did_open_response(200, main_source);
    let expected_prefix = frame(&open_main_response);

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.starts_with(&expected_prefix),
        "document path 付き didOpen response prefix が一致すべき: {}",
        output
    );
    let references_part = &output[expected_prefix.len()..];
    assert!(
        references_part.contains(&lsharp_name_hash("src/Support/Mid.ls").to_string()),
        "actual lsp --stdio は filesystem import 先の virtual URI を references に含めるべき: {}",
        references_part
    );
}

/// CP-04: actual `lsp --stdio` でも nested import を document path から
/// filesystem で辿って references を解決できること
#[test]
fn test_e2e_lsp_actual_stdio_references_find_nested_filesystem_import_occurrences_from_document_path()
 {
    let dir = filesystem_lsp_fixture_dir("stdio_references_nested_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let mid_source = "(module Support.Mid) (import Support.Base) (defn mid-val [] (base-val))";
    let references_col = mid_source.find("(base-val)").expect("base-val call") + 2;
    let open_mid_body = make_did_open_with_path(201, "src/Support/Mid.ls", mid_source);
    let references_body = format!(
        r#"{{"jsonrpc":"2.0","id":98,"method":"textDocument/references","params":{{"uri":201,"line":1,"col":{references_col}}}}}"#
    );
    let stdin = format!("{}{}", frame(&open_mid_body), frame(&references_body));
    let open_mid_response = make_did_open_response(201, mid_source);
    let expected_prefix = frame(&open_mid_response);

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.starts_with(&expected_prefix),
        "nested document path 付き didOpen response prefix が一致すべき: {}",
        output
    );
    let references_part = &output[expected_prefix.len()..];
    assert!(
        references_part.contains(&lsharp_name_hash("src/Support/Base.ls").to_string()),
        "actual lsp --stdio は nested filesystem import 先の virtual URI を references に含めるべき: {}",
        references_part
    );
}

/// CP-04: stateful harness 上で open 済み document の path を起点に
/// filesystem import 先を含む rename workspace edit を返せること
#[test]
fn test_e2e_lsp_stateful_rename_returns_filesystem_import_workspace_edit_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("rename_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let rename_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let harness = format!(
        r#"
(defn make-doc-open-params [uri src path]
  (vector-push (vector-push (vector-push (vector-new 3) uri) src) path))

(defn main []
  (let [state (server-state-new)
        _ (handle-didOpen (make-doc-open-params 200 "{main_source}" "src/Main.ls") state)
        params (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 200) 1) {rename_col}) "") "mid-next")
        changes (handle-rename params state)]
    (do
      (print (vector-length changes))
      (print (if (> (vector-length changes) 1) (vector-get (vector-get changes 1) 0) (- 0 1)))
      0)))
"#
    );

    let lines = run_lsp_harness_with_dir(&harness, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        lines[0], "2",
        "document path 付き didOpen は filesystem import 先を含む rename workspace edit を 2 change 返すべき"
    );
    assert_eq!(
        lines[1],
        lsharp_name_hash("src/Support/Mid.ls").to_string(),
        "document path 付き didOpen は filesystem import 先の virtual URI を rename workspace edit に含めるべき"
    );
}

/// CP-04: stateful harness 上で nested import も document path を起点に
/// filesystem から辿って rename workspace edit を返せること
#[test]
fn test_e2e_lsp_stateful_rename_returns_nested_filesystem_import_workspace_edit_from_document_path()
{
    let dir = filesystem_lsp_fixture_dir("rename_nested_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let mid_source = "(module Support.Mid) (import Support.Base) (defn mid-val [] (base-val))";
    let rename_col = mid_source.find("(base-val)").expect("base-val call") + 2;
    let harness = format!(
        r#"
(defn make-doc-open-params [uri src path]
  (vector-push (vector-push (vector-push (vector-new 3) uri) src) path))

(defn main []
  (let [state (server-state-new)
        _ (handle-didOpen (make-doc-open-params 201 "{mid_source}" "src/Support/Mid.ls") state)
        params (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 201) 1) {rename_col}) "") "base-next")
        changes (handle-rename params state)]
    (do
      (print (vector-length changes))
      (print (if (> (vector-length changes) 1) (vector-get (vector-get changes 1) 0) (- 0 1)))
      0)))
"#
    );

    let lines = run_lsp_harness_with_dir(&harness, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        lines[0], "2",
        "document path 付き didOpen は nested filesystem import 先を含む rename workspace edit を 2 change 返すべき"
    );
    assert_eq!(
        lines[1],
        lsharp_name_hash("src/Support/Base.ls").to_string(),
        "document path 付き didOpen は nested filesystem import 先の virtual URI を rename workspace edit に含めるべき"
    );
}

/// CP-04: actual `lsp --stdio` でも document path を起点に
/// filesystem import 先を含む rename workspace edit を返せること
#[test]
fn test_e2e_lsp_actual_stdio_rename_returns_filesystem_import_workspace_edit_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("stdio_rename_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let rename_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let open_main_body = make_did_open_with_path(200, "src/Main.ls", main_source);
    let rename_body = format!(
        r#"{{"jsonrpc":"2.0","id":99,"method":"textDocument/rename","params":{{"uri":200,"line":1,"col":{rename_col},"newName":"mid-next"}}}}"#
    );
    let stdin = format!("{}{}", frame(&open_main_body), frame(&rename_body));
    let open_main_response = make_did_open_response(200, main_source);
    let expected_prefix = frame(&open_main_response);

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.starts_with(&expected_prefix),
        "document path 付き didOpen response prefix が一致すべき: {}",
        output
    );
    let rename_part = &output[expected_prefix.len()..];
    assert!(
        rename_part.contains(&lsharp_name_hash("src/Support/Mid.ls").to_string()),
        "actual lsp --stdio は filesystem import 先の virtual URI を rename workspace edit に含めるべき: {}",
        rename_part
    );
}

/// CP-04: actual `lsp --stdio` でも nested import を document path から
/// filesystem で辿って rename workspace edit を返せること
#[test]
fn test_e2e_lsp_actual_stdio_rename_returns_nested_filesystem_import_workspace_edit_from_document_path()
 {
    let dir = filesystem_lsp_fixture_dir("stdio_rename_nested_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let mid_source = "(module Support.Mid) (import Support.Base) (defn mid-val [] (base-val))";
    let rename_col = mid_source.find("(base-val)").expect("base-val call") + 2;
    let open_mid_body = make_did_open_with_path(201, "src/Support/Mid.ls", mid_source);
    let rename_body = format!(
        r#"{{"jsonrpc":"2.0","id":100,"method":"textDocument/rename","params":{{"uri":201,"line":1,"col":{rename_col},"newName":"base-next"}}}}"#
    );
    let stdin = format!("{}{}", frame(&open_mid_body), frame(&rename_body));
    let open_mid_response = make_did_open_response(201, mid_source);
    let expected_prefix = frame(&open_mid_response);

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.starts_with(&expected_prefix),
        "nested document path 付き didOpen response prefix が一致すべき: {}",
        output
    );
    let rename_part = &output[expected_prefix.len()..];
    assert!(
        rename_part.contains(&lsharp_name_hash("src/Support/Base.ls").to_string()),
        "actual lsp --stdio は nested filesystem import 先の virtual URI を rename workspace edit に含めるべき: {}",
        rename_part
    );
}

/// CP-04: stateful harness 上で open 済み document の path を起点に
/// filesystem import 先の completion 候補を解決できること
#[test]
fn test_e2e_lsp_stateful_completion_resolves_filesystem_import_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("completion_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-va))";
    let completion_col = main_source.find("mid-va").expect("mid-va call") + "mid-va".len() + 1;
    let harness = format!(
        r#"
(defn make-doc-open-params [uri src path]
  (vector-push (vector-push (vector-push (vector-new 3) uri) src) path))

(defn main []
  (let [state (server-state-new)
        _ (handle-didOpen (make-doc-open-params 200 "{main_source}" "src/Main.ls") state)
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

    let lines = run_lsp_harness_with_dir(&harness, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        lines[0], "1",
        "document path 付き didOpen は filesystem import 先の completion 候補を 1 件返すべき"
    );
    assert_eq!(
        lines[1], "mid-val",
        "document path 付き didOpen は filesystem import 先の defn 名を completion 候補に出すべき"
    );
}

/// CP-04: stateful harness 上で nested import も document path を起点に
/// filesystem から辿って completion 候補を解決できること
#[test]
fn test_e2e_lsp_stateful_completion_resolves_nested_filesystem_import_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("completion_nested_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let mid_source = "(module Support.Mid) (import Support.Base) (defn mid-val [] (base-va))";
    let completion_col = mid_source.find("base-va").expect("base-va call") + "base-va".len() + 1;
    let harness = format!(
        r#"
(defn make-doc-open-params [uri src path]
  (vector-push (vector-push (vector-push (vector-new 3) uri) src) path))

(defn main []
  (let [state (server-state-new)
        _ (handle-didOpen (make-doc-open-params 201 "{mid_source}" "src/Support/Mid.ls") state)
        params (vector-push (vector-push (vector-push (vector-new 3) 201) 1) {completion_col})
        items (handle-completion params state)]
    (do
      (print (vector-length items))
      (if (> (vector-length items) 0)
        (print-string (vector-get (vector-get items 0) 0))
        (print-string "none"))
      0)))
"#
    );

    let lines = run_lsp_harness_with_dir(&harness, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        lines[0], "1",
        "document path 付き didOpen は nested filesystem import 先の completion 候補を 1 件返すべき"
    );
    assert_eq!(
        lines[1], "base-val",
        "document path 付き didOpen は nested filesystem import 先の defn 名を completion 候補に出すべき"
    );
}

/// CP-04: actual `lsp --stdio` でも document path を起点に
/// filesystem import 先の completion 候補を解決できること
#[test]
fn test_e2e_lsp_actual_stdio_completion_resolves_filesystem_import_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("stdio_completion_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-va))";
    let completion_col = main_source.find("mid-va").expect("mid-va call") + "mid-va".len() + 1;
    let open_main_body = make_did_open_with_path(200, "src/Main.ls", main_source);
    let completion_body = format!(
        r#"{{"jsonrpc":"2.0","id":93,"method":"textDocument/completion","params":{{"uri":200,"line":1,"col":{completion_col}}}}}"#
    );
    let stdin = format!("{}{}", frame(&open_main_body), frame(&completion_body));
    let open_main_response = make_did_open_response(200, main_source);
    let expected_prefix = frame(&open_main_response);

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.starts_with(&expected_prefix),
        "document path 付き didOpen response prefix が一致すべき: {}",
        output
    );
    let completion_part = &output[expected_prefix.len()..];
    assert!(
        completion_part.contains("mid-val"),
        "actual lsp --stdio は filesystem import 先の completion 候補を返すべき: {}",
        completion_part
    );
}

/// CP-04: actual `lsp --stdio` でも nested import を document path から
/// filesystem で辿って completion 候補を解決できること
#[test]
fn test_e2e_lsp_actual_stdio_completion_resolves_nested_filesystem_import_from_document_path() {
    let dir = filesystem_lsp_fixture_dir("stdio_completion_nested_filesystem_import");
    write_filesystem_lsp_fixture_files(&dir, &filesystem_nested_fixture_files());
    let mid_source = "(module Support.Mid) (import Support.Base) (defn mid-val [] (base-va))";
    let completion_col = mid_source.find("base-va").expect("base-va call") + "base-va".len() + 1;
    let open_mid_body = make_did_open_with_path(201, "src/Support/Mid.ls", mid_source);
    let completion_body = format!(
        r#"{{"jsonrpc":"2.0","id":94,"method":"textDocument/completion","params":{{"uri":201,"line":1,"col":{completion_col}}}}}"#
    );
    let stdin = format!("{}{}", frame(&open_mid_body), frame(&completion_body));
    let open_mid_response = make_did_open_response(201, mid_source);
    let expected_prefix = frame(&open_mid_response);

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        output.starts_with(&expected_prefix),
        "nested document path 付き didOpen response prefix が一致すべき: {}",
        output
    );
    let completion_part = &output[expected_prefix.len()..];
    assert!(
        completion_part.contains("base-val"),
        "actual lsp --stdio は nested filesystem import 先の completion 候補を返すべき: {}",
        completion_part
    );
}

// ============================================================
// CP-04: changed/latest document state parity
// ============================================================

/// CP-04: stateful harness 上で didChange 後の completion が最新 source を使うこと
#[test]
fn test_e2e_lsp_stateful_completion_uses_changed_document() {
    let open_source = "(defn alpha [] 1) (al)";
    let changed_source = "(defn helper [] 1) (he)";
    let completion_col = changed_source.find("he").expect("he call") + "he".len() + 1;
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 42 "{open_source}")
        _ (server-state-change-document state 42 "{changed_source}")
        params (vector-push (vector-push (vector-push (vector-new 3) 42) 1) {completion_col})
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
    assert_eq!(
        lines[0], "1",
        "didChange 後の completion 候補は 1 件であるべき"
    );
    assert_eq!(
        lines[1], "helper",
        "didChange 後の completion は最新 source の helper を返すべき"
    );
}

/// CP-04: stateful harness 上で didChange 後の hover が最新 source を使うこと
#[test]
fn test_e2e_lsp_stateful_hover_uses_changed_document() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let hover_col = changed_source.find("(helper 1)").expect("helper call") + 2;
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 42 "{open_source}")
        _ (server-state-change-document state 42 "{changed_source}")
        params (vector-push (vector-push (vector-push (vector-new 3) 42) 1) {hover_col})
        hover (handle-hover params state)]
    (do
      (print-string (vector-get hover 1))
      0)))
"#
    );

    let lines = run_lsp_harness(&harness);
    assert_eq!(
        lines[0], "defn helper",
        "didChange 後の hover は最新 source の helper を返すべき"
    );
}

/// CP-04: stateful harness 上で didChange 後の definition が最新 source を使うこと
#[test]
fn test_e2e_lsp_stateful_definition_uses_changed_document() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let definition_col = changed_source.find("(helper 1)").expect("helper call") + 2;
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 42 "{open_source}")
        _ (server-state-change-document state 42 "{changed_source}")
        params (vector-push (vector-push (vector-push (vector-new 3) 42) 1) {definition_col})
        location (handle-goto-definition params state)]
    (do
      (print (vector-get location 0))
      (print (vector-get location 1))
      (print (vector-get location 2))
      0)))
"#
    );

    let lines = run_lsp_harness(&harness);
    assert_eq!(
        lines[0], "42",
        "didChange 後の definition は同一 URI を返すべき"
    );
    assert_eq!(
        lines[1], "1",
        "didChange 後の definition line は defn 行を指すべき"
    );
    assert_eq!(
        lines[2], "7",
        "didChange 後の definition col は helper 定義を指すべき"
    );
}

/// CP-04: stateful harness 上で didChange 後の references が最新 source を使うこと
#[test]
fn test_e2e_lsp_stateful_references_uses_changed_document() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let references_col = changed_source.find("(helper 1)").expect("helper call") + 2;
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 42 "{open_source}")
        _ (server-state-change-document state 42 "{changed_source}")
        params (vector-push (vector-push (vector-push (vector-new 3) 42) 1) {references_col})
        locations (handle-references params state)]
    (do
      (print (vector-length locations))
      (print (vector-get (vector-get locations 0) 0))
      (print (vector-get (vector-get locations 1) 2))
      (print (vector-get (vector-get locations 2) 2))
      0)))
"#
    );

    let lines = run_lsp_harness(&harness);
    assert_eq!(
        lines[0], "3",
        "didChange 後の references は helper の 3 箇所を返すべき"
    );
    assert_eq!(
        lines[1], "42",
        "didChange 後の references は同一 URI を返すべき"
    );
    assert_eq!(
        lines[2], "40",
        "didChange 後の references 2 件目は helper call 位置を指すべき"
    );
    assert_eq!(
        lines[3], "51",
        "didChange 後の references 3 件目は helper call 位置を指すべき"
    );
}

/// CP-04: stateful harness 上で didChange 後の rename が最新 source を使うこと
#[test]
fn test_e2e_lsp_stateful_rename_uses_changed_document() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let rename_col = changed_source.find("(helper 1)").expect("helper call") + 2;
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 42 "{open_source}")
        _ (server-state-change-document state 42 "{changed_source}")
        params (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 42) 1) {rename_col}) "") "cube")
        changes (handle-rename params state)]
    (do
      (print (vector-length changes))
      (print (vector-get (vector-get changes 0) 0))
      (print (vector-length (vector-get (vector-get changes 0) 1)))
      0)))
"#
    );

    let lines = run_lsp_harness(&harness);
    assert_eq!(
        lines[0], "1",
        "didChange 後の rename は 1 URI 分の workspace edit を返すべき"
    );
    assert_eq!(
        lines[1], "42",
        "didChange 後の rename は同一 URI の edit を返すべき"
    );
    assert_eq!(
        lines[2], "3",
        "didChange 後の rename は helper の 3 箇所を書き換えるべき"
    );
}

/// CP-04: stateful harness 上で same-URI repeated didOpen 後に最新 source を保持すること
#[test]
fn test_e2e_lsp_stateful_repeated_didopen_keeps_latest_source() {
    let first_source = "(defn alpha [] 1) (al)";
    let latest_source = "(defn beta [] 1) (be)";
    let completion_col = latest_source.find("be").expect("be call") + "be".len() + 1;
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 42 "{first_source}")
        _ (server-state-open-document state 42 "{latest_source}")
        params (vector-push (vector-push (vector-push (vector-new 3) 42) 1) {completion_col})
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
    assert_eq!(
        lines[0], "1",
        "repeated didOpen 後の completion 候補は 1 件であるべき"
    );
    assert_eq!(
        lines[1], "beta",
        "repeated didOpen 後の completion は最新 source の beta を返すべき"
    );
}
