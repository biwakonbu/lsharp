use super::support::*;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static CLI_TEST_FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn lsp_stdio_snapshot(name: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/snapshots/lsp/stdio")
        .join(name);
    let snapshot = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("snapshot 読み込み失敗 {}: {}", path.display(), e));
    serde_json::from_str(&snapshot)
        .unwrap_or_else(|e| panic!("snapshot JSON parse 失敗 {}: {}", path.display(), e))
}

fn parse_lsp_stdio_frames(output: &str) -> Vec<Value> {
    let bytes = output.as_bytes();
    let mut cursor = 0;
    let mut frames = Vec::new();

    while cursor < bytes.len() {
        let header_end = bytes[cursor..]
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|offset| cursor + offset)
            .unwrap_or_else(|| {
                panic!(
                    "LSP frame header terminator が見つからない: cursor={} output={:?}",
                    cursor, output
                )
            });
        let header = std::str::from_utf8(&bytes[cursor..header_end])
            .unwrap_or_else(|e| panic!("LSP frame header は UTF-8 であるべき: {}", e));
        let content_length = header
            .strip_prefix("Content-Length: ")
            .unwrap_or_else(|| {
                panic!(
                    "LSP frame header は Content-Length で始まるべき: {:?}",
                    header
                )
            })
            .parse::<usize>()
            .unwrap_or_else(|e| panic!("Content-Length parse 失敗 {:?}: {}", header, e));
        let body_start = header_end + 4;
        let body_end = body_start + content_length;
        assert!(
            body_end <= bytes.len(),
            "LSP frame body が途中で切れている: header={:?} bytes={} output={:?}",
            header,
            bytes.len(),
            output
        );
        let body = std::str::from_utf8(&bytes[body_start..body_end])
            .unwrap_or_else(|e| panic!("LSP frame body は UTF-8 であるべき: {}", e));
        let payload = serde_json::from_str(body).unwrap_or_else(|e| {
            panic!(
                "LSP frame body は valid JSON であるべき: {}\nbody={:?}",
                e, body
            )
        });
        frames.push(payload);
        cursor = body_end;
    }

    frames
}

fn assert_lsp_stdio_snapshot(output: &str, snapshot_name: &str, message: &str) {
    let actual = parse_lsp_stdio_frames(output);
    let expected = lsp_stdio_snapshot(snapshot_name);
    assert_eq!(actual, expected, "{}", message);
}

fn cli_text_snapshot(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/snapshots/cli")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cli snapshot 読み込み失敗 {}: {}", path.display(), e))
}

fn assert_cli_text_snapshot(output: &str, snapshot_name: &str, message: &str) {
    let expected = cli_text_snapshot(snapshot_name);
    assert_eq!(output, expected, "{}", message);
}

fn doctools_json_snapshot(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/snapshots/doctools")
        .join(name);
    let snapshot = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("doctools snapshot 読み込み失敗 {}: {}", path.display(), e));
    serde_json::from_str(&snapshot).unwrap_or_else(|e| {
        panic!(
            "doctools snapshot JSON parse 失敗 {}: {}",
            path.display(),
            e
        )
    })
}

fn cli_test_fixture_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lsharp_test_cli_core_{}_{}_{}",
        prefix,
        std::process::id(),
        CLI_TEST_FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn write_cli_fixture_files(dir: &std::path::Path, files: &[(&str, &str)]) {
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir).expect("fixture directory の作成に失敗");
    for (relative, source) in files {
        let path = dir.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|e| {
                panic!("fixture parent の作成に失敗 {}: {}", parent.display(), e)
            });
        }
        std::fs::write(&path, source)
            .unwrap_or_else(|e| panic!("fixture file の書き込みに失敗 {}: {}", path.display(), e));
    }
}

fn cli_multifile_nested_fixture_files() -> [(&'static str, &'static str); 3] {
    [
        (
            "Support/Base.ls",
            "(module Support.Base)\n(defn base-val [] 10)",
        ),
        (
            "Support/Mid.ls",
            "(module Support.Mid)\n(import Support.Base)\n(defn mid-val [] (* (base-val) 2))",
        ),
        (
            "main.ls",
            "(module Main)\n(import Support.Mid)\n(defn main [] (mid-val))",
        ),
    ]
}

fn cli_lsp_nested_fixture_files() -> [(&'static str, &'static str); 3] {
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

fn make_lsp_did_open_with_path(uri: u32, path: &str, source: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":{},"path":"{}","source":"{}"}}}}"#,
        uri, path, source
    )
}

fn lsp_frame(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", body.len(), body)
}

fn run_lsp_stdio_with_dir(stdin: &str, dir: &std::path::Path) -> String {
    let wasm = compile_only(selfhost_cli_runtime_bundle());
    lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin(
        &wasm,
        Some(dir),
        &["lsp", "--stdio"],
        stdin,
    )
    .expect("filesystem-backed lsp stdio 実行に失敗")
}

fn run_lsp_filesystem_snapshot_request(
    prefix: &str,
    open_uri: u32,
    open_path: &str,
    open_source: &str,
    request_body: &str,
) -> String {
    let dir = cli_test_fixture_dir(prefix);
    write_cli_fixture_files(&dir, &cli_lsp_nested_fixture_files());
    let open_body = make_lsp_did_open_with_path(open_uri, open_path, open_source);
    let stdin = format!("{}{}", lsp_frame(&open_body), lsp_frame(request_body));
    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    output
}

fn parse_wasm_size_line(line: &str, context: &str) -> i64 {
    assert!(
        line.starts_with("wasm-size:"),
        "{}: wasm-size:<n> 形式であるべき: {:?}",
        context,
        line
    );
    line["wasm-size:".len()..]
        .parse()
        .unwrap_or_else(|e| panic!("{}: wasm size parse 失敗 {:?}: {}", context, line, e))
}

fn parse_i64_line(line: &str, context: &str) -> i64 {
    line.parse()
        .unwrap_or_else(|e| panic!("{}: integer parse 失敗 {:?}: {}", context, line, e))
}

fn run_cli_multifile_helper_size(dir: &std::path::Path, file_path: &str, target: i64) -> i64 {
    let harness = format!(
        r#"
(defn main []
  (print (compile-file-wasm-size "{file_path}" {target})))
"#
    );
    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, dir);
    parse_i64_line(
        output
            .trim()
            .lines()
            .next()
            .expect("compile-file-wasm-size output が必要"),
        "compile-file-wasm-size helper output",
    )
}

/// TEST-CLI-02-C: selfhost/src/App/Cli.ls に repl/lsp/fmt/doc コマンド定義
///
/// T4-4 AC-013: ユーティリティコマンドが L# 実装で動作すること
/// Red Phase: selfhost/src/App/Cli.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_cli_repl_lsp_fmt() {
    let cli_path = selfhost_source_path("Cli.ls");
    assert!(cli_path.exists(), "selfhost/src/App/Cli.ls が存在しない");
    let source =
        std::fs::read_to_string(&cli_path).expect("selfhost/src/App/Cli.ls の読み込みに失敗");

    // ユーティリティコマンドの定義を確認 (T4-4 AC-013)
    let commands = ["repl", "lsp", "fmt", "doc"];
    for cmd in &commands {
        assert!(
            source.contains(cmd),
            "selfhost/src/App/Cli.ls に '{}' コマンドの定義がない (AC-013)",
            cmd
        );
    }
}

/// TEST-CLI-02-C2: canonical App/Cli.ls が file-path compile gate を通過すること
#[test]
fn test_e2e_selfhost_cli_canonical_file_compile() {
    let wasm = compile_file_only(&selfhost_source_path("Cli.ls"));
    assert!(
        wasm.len() > 1000,
        "canonical Cli.ls の Wasm が小さすぎる: {} bytes",
        wasm.len()
    );
}

/// TEST-CLI-01-B: selfhost/src/App/Cli.ls の --help 相当出力が主要コマンドを列挙できること
///
/// T4a-2 AC-104/AC-106: help 出力が usage とサブコマンド一覧を含むこと
#[test]
fn test_e2e_selfhost_cli_help_output() {
    let harness = r#"
(defn main []
  (do
    (show-help)
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert!(
        output.contains("Usage: lsharp <command>"),
        "help 出力に usage 行が必要: {:?}",
        output
    );
    for cmd in [
        "parse",
        "check",
        "compile",
        "build",
        "test",
        "review",
        "doc-ack",
        "doc-check",
        "install",
        "repl",
        "lsp",
        "fmt",
        "doc",
    ] {
        assert!(
            output.contains(cmd),
            "help 出力に '{}' が必要: {:?}",
            cmd,
            output
        );
    }
}

/// TEST-CLI-01-B2: selfhost/src/App/Cli.ls の compile target parser helper が preview1/component/alias を区別できること
#[test]
fn test_e2e_selfhost_cli_compile_target_parser_helper() {
    let harness = r#"
(defn main []
  (do
    (print (parse-compile-target-name "wasi-preview1"))
    (print (parse-compile-target-name "wasi-component"))
    (print (parse-compile-target-name "wasm"))
    (print (parse-compile-target-name "bogus"))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["0", "1", "1", "-1"],
        "compile target parser helper は preview1/component/alias/invalid を区別するべき: {:?}",
        lines
    );
}

/// TEST-CLI-01-B3: compile/build subcommand help に target option が明示されること
#[test]
fn test_e2e_selfhost_cli_compile_help_mentions_target_option() {
    let harness = r#"
(defn main []
  (do
    (print-string (format-subcommand-help "compile"))
    (print-string "
")
    (print-string (format-subcommand-help "build"))
    (print-string "
")
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "subcommand help 出力が不足: {:?}", lines);
    assert!(
        lines[0].contains("--target"),
        "compile help は --target option を案内するべき: {:?}",
        lines[0]
    );
    assert!(
        lines[1].contains("--target"),
        "build help は --target option を案内するべき: {:?}",
        lines[1]
    );
}

/// TEST-CLI-01-C: selfhost/src/App/Cli.ls の --version 相当出力が `lsharp x.y.z` 形式であること
///
/// T4a-2 AC-105: version 出力形式を固定する
#[test]
fn test_e2e_selfhost_cli_version_output() {
    let harness = r#"
(defn main []
  (do
    (show-version)
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(output.trim(), "lsharp 0.1.0");
}

/// TEST-CLI-02-D: selfhost/src/App/Cli.ls の parse core helper が source を parse できること
///
/// CLI-02 の最小 tranche として、file I/O 抜きで parse-program を CLI helper へ接続する。
#[test]
fn test_e2e_selfhost_cli_parse_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-parse-source "(defn main [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 5, "cli parse core 出力が不足: {:?}", lines);
    assert_eq!(
        lines[0], "decls:1",
        "program decl-count text は 1 であるべき"
    );
    assert_eq!(
        lines[1], "first-decl:defn",
        "先頭 decl は defn text であるべき"
    );
    assert_eq!(
        lines[2], "first-body:int",
        "defn body は int text であるべき"
    );
    assert_eq!(
        lines[3], "diagnostics:0",
        "parse diagnostics summary は 0 件であるべき"
    );
    assert_eq!(
        lines[4], "0",
        "run-parse-source の終了コードは success であるべき"
    );
}

/// TEST-CLI-02-E: selfhost/src/App/Cli.ls の check core helper が source を型推論できること
///
/// CLI-02 の最小 tranche として、file I/O 抜きで TypeInfer.infer を CLI helper へ接続する。
#[test]
fn test_e2e_selfhost_cli_check_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-check-source "(defn main [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 3, "cli check core 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "Int", "check 結果は型名 Int を返すべき");
    assert_eq!(
        lines[1], "diagnostics:0",
        "check diagnostics summary は 0 件であるべき"
    );
    assert_eq!(
        lines[2], "0",
        "run-check-source の終了コードは success であるべき"
    );
}

/// TEST-CLI-02-F: selfhost/src/App/Cli.ls の run-parse が file-path から source を読めること
#[test]
fn test_e2e_selfhost_cli_parse_file_handler() {
    let dir =
        std::env::temp_dir().join(format!("lsharp_test_cli_parse_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-parse "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "cli parse file handler 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "decls:1",
        "program decl-count text は 1 であるべき"
    );
    assert_eq!(
        lines[1], "first-decl:defn",
        "先頭 decl は defn text であるべき"
    );
    assert_eq!(
        lines[2], "first-body:int",
        "defn body は int text であるべき"
    );
    assert_eq!(
        lines[3], "diagnostics:0",
        "parse diagnostics summary は 0 件であるべき"
    );
    assert_eq!(lines[4], "0", "run-parse の終了コードは success であるべき");
}

/// TEST-CLI-02-G: selfhost/src/App/Cli.ls の run-check が file-path から source を読めること
#[test]
fn test_e2e_selfhost_cli_check_file_handler() {
    let dir =
        std::env::temp_dir().join(format!("lsharp_test_cli_check_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-check "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "cli check file handler 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "Int", "check 結果は型名 Int を返すべき");
    assert_eq!(
        lines[1], "diagnostics:0",
        "check diagnostics summary は 0 件であるべき"
    );
    assert_eq!(lines[2], "0", "run-check の終了コードは success であるべき");
}

/// TEST-CLI-02-G2: run-parse-source が recovery 入力でも diagnostics summary を返すこと
#[test]
fn test_e2e_selfhost_cli_parse_source_recovery_summary() {
    let harness = r#"
(defn main []
  (do
    (print (run-parse-source ")" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "cli parse recovery 出力が不足: {:?}",
        lines
    );
    assert!(
        lines
            .iter()
            .any(|line| *line == "diagnostics:1,P0001@1:1,first-body:unexpected token )"),
        "parse recovery summary は code/location を含むべき: {:?}",
        lines
    );
    assert_eq!(
        lines.last(),
        Some(&"0"),
        "run-parse-source は recovery summary 後も success を返すべき"
    );
}

/// TEST-CLI-02-G2b: run-parse-source が `]` recovery でも token 別 diagnostics body を返すこと
#[test]
fn test_e2e_selfhost_cli_parse_source_recovery_unexpected_bracket_summary() {
    let harness = r#"
(defn main []
  (do
    (print (run-parse-source "]" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "cli parse recovery `]` 出力が不足: {:?}",
        lines
    );
    assert!(
        lines
            .iter()
            .any(|line| *line == "diagnostics:1,P0001@1:1,first-body:unexpected token ]"),
        "parse recovery summary は unexpected token ] を含むべき: {:?}",
        lines
    );
    assert_eq!(
        lines.last(),
        Some(&"0"),
        "run-parse-source は recovery summary 後も success を返すべき"
    );
}

/// TEST-CLI-02-G3: run-check-source が型エラー入力でも diagnostics summary を返すこと
#[test]
fn test_e2e_selfhost_cli_check_source_type_error_summary() {
    let harness = r#"
(defn main []
  (do
    (print (run-check-source "(defn main [] (if 42 1 0))" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "cli check type-error 出力が不足: {:?}",
        lines
    );
    assert!(
        lines
            .iter()
            .any(|line| *line == "diagnostics:1,T0001@1:1,first-body:if condition must be Bool"),
        "check type-error summary は code/location を含むべき: {:?}",
        lines
    );
    assert_eq!(
        lines.last(),
        Some(&"0"),
        "run-check-source は type-error summary 後も success を返すべき"
    );
}

/// TEST-CLI-02-G3b: run-check-source が未定義シンボルでも code 別 diagnostics body を返すこと
#[test]
fn test_e2e_selfhost_cli_check_source_undefined_symbol_summary() {
    let harness = r#"
(defn main []
  (do
    (print (run-check-source "(defn main [] missing)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "cli check undefined-symbol 出力が不足: {:?}",
        lines
    );
    assert!(
        lines
            .iter()
            .any(|line| *line == "diagnostics:1,T0001@1:1,first-body:undefined symbol"),
        "check undefined-symbol summary は code 別 body を含むべき: {:?}",
        lines
    );
    assert_eq!(
        lines.last(),
        Some(&"0"),
        "run-check-source は diagnostics summary 後も success を返すべき"
    );
}

/// TEST-CLI-02-H: selfhost/src/App/Cli.ls の file-path handler は missing file を compile error で返す
#[test]
fn test_e2e_selfhost_cli_file_handler_missing_file() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_missing_file_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-parse "missing.ls" 0))
    (print (run-check "missing.ls" 0))
    (print (run-build "missing.ls" 0))
    (print (run-test "missing.ls" 0))
    (print (run-review "missing.ls" 0))
    (print (run-fmt "missing.ls" 0))
    (print (run-compile "missing.ls" 0))
    (print (run-doc-ack "missing.ls" 0))
    (print (run-doc-check "missing.ls" 0))
    (print (run-doc "missing.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "1", "1", "1", "1", "1", "1", "1", "1"],
        "missing file は parse/check/build/test/review/fmt/compile/doc-ack/doc-check/doc とも compile error=1 を返すべき"
    );
}

/// TEST-CLI-02-I: selfhost/src/App/Cli.ls の arg-parse がコマンド文字列を command id へ変換できること
#[test]
fn test_e2e_selfhost_cli_arg_parse_strings() {
    let harness = r#"
(defn main []
  (do
    (print (arg-parse "parse"))
    (print (arg-parse "check"))
    (print (arg-parse "compile"))
    (print (arg-parse "doc"))
    (print (arg-parse "unknown"))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "2", "3", "13", "0"],
        "arg-parse は既知コマンドを対応する id へ変換し、未知コマンドは 0 を返すべき"
    );
}

/// TEST-CLI-02-J: selfhost/src/App/Cli.ls の run-fmt-source が format-program を呼べること
#[test]
fn test_e2e_selfhost_cli_fmt_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-fmt-source "(defn a [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.len(),
        2,
        "run-fmt-source は 1 つの fmt 出力と success code を返すべき"
    );
    assert_eq!(
        lines[0], "(defn a [] 42)",
        "run-fmt-source は format-program の canonical text を stdout へ返すべき"
    );
    assert_eq!(lines[1], "0", "run-fmt-source は success=0 を返すべき");
}

/// TEST-CLI-02-J2: run-fmt-source が string literal を fallback せず返すこと
#[test]
fn test_e2e_selfhost_cli_fmt_source_string_literal() {
    let harness = r#"
(defn main []
  (do
    (print (run-fmt-source "\"abc\"" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.len(),
        2,
        "run-fmt-source string literal は fmt 出力と success code を返すべき"
    );
    assert_eq!(
        lines[0], "\"abc\"",
        "run-fmt-source は string literal を source-aware formatter で返すべき"
    );
    assert_eq!(lines[1], "0", "run-fmt-source は success=0 を返すべき");
}

/// TEST-CLI-02-J3: run-fmt-source string literal output を snapshot に固定すること
#[test]
fn test_e2e_selfhost_cli_fmt_source_string_literal_snapshot() {
    let harness = r#"
(defn main []
  (do
    (print (run-fmt-source "\"abc\"" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_cli_text_snapshot(
        &output,
        "fmt-source-string-literal.txt",
        "run-fmt-source string literal output は representative text snapshot と一致するべき",
    );
}

/// TEST-CLI-02-J4: run-fmt-source が defn metadata を canonical 順で保持すること
#[test]
fn test_e2e_selfhost_cli_fmt_source_defn_metadata() {
    let harness = r#"
(defn main []
  (do
    (print (run-fmt-source "(defn add [x y] :doc \"Add two ints\" :params [(x \"left\") (y \"right\")] :returns \"sum\" :example [(add 1 2)] (+ x y))" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.len(),
        2,
        "run-fmt-source metadata は fmt 出力と success code を返すべき"
    );
    assert_eq!(
        lines[0],
        "(defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y))",
        "run-fmt-source は defn metadata を canonical 順で返すべき"
    );
    assert_eq!(lines[1], "0", "run-fmt-source は success=0 を返すべき");
}

/// TEST-CLI-02-K: selfhost/src/App/Cli.ls の run-fmt が file-path から source を読めること
#[test]
fn test_e2e_selfhost_cli_fmt_file_handler() {
    let dir = std::env::temp_dir().join(format!("lsharp_test_cli_fmt_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn a [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-fmt "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.len(),
        2,
        "run-fmt は 1 つの fmt 出力と success code を返すべき"
    );
    assert_eq!(
        lines[0], "(defn a [] 42)",
        "run-fmt は file-path 経由でも canonical text を stdout へ返すべき"
    );
    assert_eq!(lines[1], "0", "run-fmt は success=0 を返すべき");
}

/// TEST-CLI-02-L: selfhost/src/App/Cli.ls の run-compile-source が compile PoC を呼べること
#[test]
fn test_e2e_selfhost_cli_compile_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-compile-source "(defn main [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "run-compile-source 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "run-compile-source は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    let wasm_size: i64 = lines[0]["wasm-size:".len()..]
        .parse()
        .expect("wasm size は整数であるべき");
    assert!(
        wasm_size > 8,
        "wasm size は header 超であるべき: {}",
        wasm_size
    );
    assert_eq!(
        lines[1], "0",
        "run-compile-source の終了コードは success であるべき"
    );
}

/// TEST-CLI-02-L2: emit-wasm-with-target が preview1/component で size を切り替えること
#[test]
fn test_e2e_selfhost_cli_emit_wasm_with_target_changes_wasm_size() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42)")
    ir (lower program)]
    (do
      (print (emit-wasm-with-target ir (compile-target-preview1)))
      (print (emit-wasm-with-target ir (compile-target-component)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.len(),
        2,
        "target 別 wasm size が 2 行必要: {:?}",
        lines
    );
    let preview1_size: i64 = lines[0]
        .parse()
        .expect("preview1 wasm size は整数であるべき");
    let component_size: i64 = lines[1]
        .parse()
        .expect("component wasm size は整数であるべき");
    assert!(
        preview1_size > component_size,
        "preview1 target は component target より大きい import layout を持つべき: preview1={preview1_size}, component={component_size}"
    );
}

/// TEST-CLI-02-M: selfhost/src/App/Cli.ls の run-compile が file-path から source を読めること
#[test]
fn test_e2e_selfhost_cli_compile_file_handler() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_compile_file_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-compile "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "run-compile 出力が不足: {:?}", lines);
    assert!(
        lines[0].starts_with("wasm-size:"),
        "run-compile は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    let wasm_size: i64 = lines[0]["wasm-size:".len()..]
        .parse()
        .expect("wasm size は整数であるべき");
    assert!(
        wasm_size > 8,
        "wasm size は header 超であるべき: {}",
        wasm_size
    );
    assert_eq!(
        lines[1], "0",
        "run-compile の終了コードは success であるべき"
    );
}

/// TEST-CLI-02-M1B: selfhost/src/App/Cli.ls の run-compile は nested import fixture を import-aware helper 経由で解決すること
#[test]
fn test_e2e_selfhost_cli_compile_file_handler_multifile_nested_imports() {
    let dir = cli_test_fixture_dir("compile_multifile_nested");
    write_cli_fixture_files(&dir, &cli_multifile_nested_fixture_files());

    let harness = r#"
(defn main []
  (let [src (read-file "main.ls")]
    (do
      (print (run-compile "main.ls" 0))
      (print (compile-file-wasm-size "main.ls" 0))
      (print (run-compile-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "run-compile multi-file nested fixture 出力が不足: {:?}",
        lines
    );
    let file_size = parse_wasm_size_line(lines[0], "run-compile multi-file nested fixture");
    let helper_size = parse_i64_line(lines[2], "compile-file-wasm-size nested fixture");
    let source_only_size =
        parse_wasm_size_line(lines[3], "run-compile-source nested fixture baseline");
    assert_eq!(lines[1], "0", "run-compile は success=0 を返すべき");
    assert_eq!(
        lines[4], "0",
        "run-compile-source baseline は success=0 を返すべき"
    );
    assert!(
        file_size == helper_size,
        "run-compile は import-aware helper と同じ wasm-size を返すべき: cli={file_size}, helper={helper_size}"
    );
    assert!(
        helper_size > source_only_size,
        "compile-file-wasm-size helper は source-only baseline より大きい wasm-size を返すべき: helper={helper_size}, source-only={source_only_size}"
    );
}

/// TEST-CLI-02-M1C: selfhost/src/App/Cli.ls は shared cache helper 経由で clean hit 時の再 parse を避けること
#[test]
fn test_e2e_selfhost_cli_compile_functions_data_with_cache_reuses_clean_hit() {
    let dir = cli_test_fixture_dir("compile_functions_data_cache");
    write_cli_fixture_files(&dir, &cli_multifile_nested_fixture_files());

    let harness = r#"
(defn main []
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        pair1 (compile-file-functions-data-with-cache "main.ls" cache-ref parse-count-ref)
        count1 (ref-get parse-count-ref)
        pair2 (compile-file-functions-data-with-cache "main.ls" cache-ref parse-count-ref)
        count2 (ref-get parse-count-ref)
        functions1 (vector-get pair1 0)
        data1 (vector-get pair1 1)
        functions2 (vector-get pair2 0)
        data2 (vector-get pair2 1)]
    (do
      (print count1)
      (print count2)
      (print (vector-length functions1))
      (print (vector-length functions2))
      (print (vector-length data1))
      (print (vector-length data2))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "compile-file-functions-data-with-cache 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "3",
        "初回 compile では main/mid/base の 3 モジュールを parse するべき"
    );
    assert_eq!(lines[1], "3", "clean hit では parse-count が増えないべき");
    assert_eq!(lines[2], "3", "functions1 は 3 個保持するべき");
    assert_eq!(lines[3], "3", "functions2 は 3 個保持するべき");
    assert_eq!(
        lines[4], lines[5],
        "data section 長は cache hit 前後で一致するべき"
    );
}

/// TEST-CLI-02-M1D: selfhost/src/App/Cli.ls は shared cache helper で module path invalidation を反映すること
#[test]
fn test_e2e_selfhost_cli_compile_functions_data_with_cache_invalidates_changed_module_path() {
    let dir = cli_test_fixture_dir("compile_functions_data_cache_invalidation");
    write_cli_fixture_files(
        &dir,
        &[
            (
                "src/Main.ls",
                "(module Main)\n(import App.Lib)\n(defn main [] (helper))",
            ),
            ("vendor/App/Lib.ls", "(module App.Lib)\n(defn helper [] 7)"),
            (".lsharp/module-index/App/Lib.path", "vendor/App/Lib.ls"),
            (
                "src/App/Placeholder.ls",
                "(module App.Placeholder)\n(defn unused [] 0)",
            ),
        ],
    );

    let harness = r#"
(defn main []
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        pair1 (compile-file-functions-data-with-cache "src/Main.ls" cache-ref parse-count-ref)
        count1 (ref-get parse-count-ref)
        _ (write-file "src/App/Lib.ls" "(module App.Lib) (defn helper [] 9)")
        pair2 (compile-file-functions-data-with-cache "src/Main.ls" cache-ref parse-count-ref)
        count2 (ref-get parse-count-ref)
        functions1 (vector-get pair1 0)
        functions2 (vector-get pair2 0)]
    (do
      (print count1)
      (print count2)
      (print (vector-length functions1))
      (print (vector-length functions2))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "compile-file-functions-data-with-cache invalidation 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "2",
        "初回 compile では main と vendor lib を parse するべき"
    );
    assert_eq!(
        lines[1], "3",
        "module path 更新後は local lib だけ再 parse するべき"
    );
    assert_eq!(lines[2], "2", "functions1 は 2 個保持するべき");
    assert_eq!(lines[3], "2", "functions2 も 2 個保持するべき");
}

/// TEST-CLI-02-M1D: selfhost cached payload helper (func_idx=7) は compiler-mode inline path と同じ Wasm を組めること
#[test]
fn test_e2e_selfhost_cli_compile_file_payload_with_cache_matches_inline_main() {
    let dir = selfhost_package_root();

    let harness = r#"
(defn main []
  (let [path "src/App/Main.ls"
        src (read-file path)
        program (parse-program src)
        source-root (resolve-source-root path)
        package-root (resolve-package-root path)
        seen-ref (ref-new (map-new))
        imported-pairs (load-imports-from-decls program src 0 (vector-length program) seen-ref (vector-new 8) source-root package-root)
        all-pairs (vector-push imported-pairs (make-src-decl-pair src program))
        n (vector-length all-pairs)
        reg-result (register-all-pairs all-pairs 0 n (ftable-new) 7)
        ftable (vector-get reg-result 0)
        data-ref (ref-new (vector-new 8))
        functions (compile-all-src-decl-pairs all-pairs 0 n ftable data-ref (vector-new 8))
        inline-data (ref-get data-ref)
        inline-bytes (build-wasm-bytes-wasi functions inline-data)
        cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        payload (compile-file-functions-payload-with-cache path 7 cache-ref parse-count-ref)
        cached-functions (vector-get payload 0)
        cached-data (vector-get payload 1)
        cached-bytes (build-wasm-bytes-wasi cached-functions cached-data)]
    (do
      (print (vector-length functions))
      (print (vector-length cached-functions))
      (print (vector-length inline-data))
      (print (vector-length cached-data))
      (print (vector-length inline-bytes))
      (print (vector-length cached-bytes))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "compile-file-functions-payload-with-cache main 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], lines[1],
        "functions length は inline/cached で一致するべき"
    );
    assert_eq!(
        lines[2], lines[3],
        "data section length は inline/cached で一致するべき"
    );
    assert_eq!(
        lines[4], lines[5],
        "build-wasm-bytes-wasi length は inline/cached で一致するべき"
    );
}

/// TEST-CLI-02-M2: selfhost/src/App/Cli.ls の run-build が file-path から source を読めること
#[test]
fn test_e2e_selfhost_cli_build_file_handler() {
    let dir =
        std::env::temp_dir().join(format!("lsharp_test_cli_build_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-build "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "run-build 出力が不足: {:?}", lines);
    assert!(
        lines[0].starts_with("wasm-size:"),
        "run-build は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    let wasm_size: i64 = lines[0]["wasm-size:".len()..]
        .parse()
        .expect("wasm size は整数であるべき");
    assert!(
        wasm_size > 8,
        "wasm size は header 超であるべき: {}",
        wasm_size
    );
    assert_eq!(lines[1], "0", "run-build の終了コードは success であるべき");
}

/// TEST-CLI-02-M2B: selfhost/src/App/Cli.ls の run-build は nested import fixture を import-aware helper 経由で解決すること
#[test]
fn test_e2e_selfhost_cli_build_file_handler_multifile_nested_imports() {
    let dir = cli_test_fixture_dir("build_multifile_nested");
    write_cli_fixture_files(&dir, &cli_multifile_nested_fixture_files());

    let harness = r#"
(defn main []
  (let [src (read-file "main.ls")]
    (do
      (print (run-build "main.ls" 0))
      (print (compile-file-wasm-size "main.ls" 0))
      (print (run-compile-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "run-build multi-file nested fixture 出力が不足: {:?}",
        lines
    );
    let file_size = parse_wasm_size_line(lines[0], "run-build multi-file nested fixture");
    let helper_size = parse_i64_line(lines[2], "compile-file-wasm-size nested fixture");
    let source_only_size =
        parse_wasm_size_line(lines[3], "run-compile-source nested fixture baseline");
    assert_eq!(lines[1], "0", "run-build は success=0 を返すべき");
    assert_eq!(
        lines[4], "0",
        "run-compile-source baseline は success=0 を返すべき"
    );
    assert!(
        file_size == helper_size,
        "run-build は import-aware helper と同じ wasm-size を返すべき: cli={file_size}, helper={helper_size}"
    );
    assert!(
        helper_size > source_only_size,
        "compile-file-wasm-size helper は source-only baseline より大きい wasm-size を返すべき: helper={helper_size}, source-only={source_only_size}"
    );
}

/// TEST-CLI-02-M3: selfhost/src/App/Cli.ls の run-install が install plan text を返せること
#[test]
fn test_e2e_selfhost_cli_install_package_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-install "core" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["package:core", "status:planned", "0"],
        "run-install は package install plan text と success=0 を返すべき"
    );
}

/// TEST-CLI-02-M4: selfhost/src/App/Cli.ls の run-install は空 package を compile error にする
#[test]
fn test_e2e_selfhost_cli_install_empty_package() {
    let harness = r#"
(defn main []
  (print (run-install "" 0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1"],
        "run-install は空 package に compile error=1 を返すべき"
    );
}

/// TEST-CLI-02-M5: selfhost/src/App/Cli.ls の run-repl が warmup session summary を返せること
#[test]
fn test_e2e_selfhost_cli_repl_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-repl 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["type:Int", "evals:1", "input-bytes:17", "0"],
        "run-repl は warmup session summary と success=0 を返すべき"
    );
}

/// TEST-CLI-02-M6: selfhost/src/App/Cli.ls の run-lsp が capability summary text を返せること
#[test]
fn test_e2e_selfhost_cli_lsp_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-lsp 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "sync:full",
            "hover:true",
            "completion:true",
            "definition:true",
            "references:true",
            "rename:true",
            "formatting:true",
            "requests:1",
            "documents:0",
            "source-bytes:0",
            "0",
        ],
        "run-lsp は capability + shared-state summary text と success=0 を返すべき"
    );
}

/// TEST-CLI-02-M7: selfhost/src/App/Cli.ls の LSP transport helper が initialize request を frame response にできること
#[test]
fn test_e2e_selfhost_cli_lsp_transport_initialize_frame() {
    let body = r#"{"jsonrpc":"2.0","id":7,"result":[1,1,1,1,1,1,1]}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let harness = r#"
(defn main []
  (let [request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                7)
              (lsp-method-initialize))
            0)]
    (print-string (run-lsp-transport-request request))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は initialize request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M8: selfhost/src/App/Cli.ls の LSP transport helper が未知メソッドを JSON-RPC error frame にできること
#[test]
fn test_e2e_selfhost_cli_lsp_transport_unknown_method_error() {
    let body = r#"{"jsonrpc":"2.0","id":9,"error":{"code":-32601,"message":"Method not found"}}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let harness = r#"
(defn main []
  (let [request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                9)
              999)
            0)]
    (print-string (run-lsp-transport-request request))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は未知メソッドに Method not found frame を返すべき"
    );
}

/// TEST-CLI-02-M8b: selfhost/src/App/Cli.ls の LSP transport helper は shutdown 後 request を error frame にすること
#[test]
fn test_e2e_selfhost_cli_lsp_transport_request_after_shutdown_error() {
    let body = r#"{"jsonrpc":"2.0","id":10,"error":{"code":-32600,"message":"Invalid Request"}}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let harness = r#"
(defn make-request [id method-id params]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 2)
        id)
      method-id)
    params))

(defn main []
  (let [shutdown-request
          (make-request 9 (lsp-method-shutdown) 0)
        hover-request
          (make-request
            10
            (lsp-method-hover)
            (vector-push
              (vector-push
                (vector-push (vector-new 3) 42)
                1)
              1))
        requests
          (vector-push
            (vector-push (vector-new 2) shutdown-request)
            hover-request)
        summary (run-lsp-transport-sequence requests)
        frames (vector-get summary 0)]
    (print-string (vector-get frames 1))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は shutdown 後 request を Invalid Request frame で拒否するべき"
    );
}

/// TEST-CLI-02-M9: selfhost/src/App/Cli.ls の LSP transport helper sequence が shared-state で複数 request を捌けること
#[test]
fn test_e2e_selfhost_cli_lsp_transport_goto_definition_frame() {
    let body = r#"{"jsonrpc":"2.0","id":7,"result":[10,1,7]}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let source = "(defn helper [x] x)\n(defn main [] (helper 1))";
    let harness = format!(
        r#"
(defn main []
  (let [params
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 10)
                2)
              16)
            "{source}")
        request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                7)
              (lsp-method-goto-def))
            params)]
    (print-string (run-lsp-transport-request request))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は goto-definition request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M9b: selfhost/src/App/Cli.ls の LSP transport helper が hover request を framed response にできること
#[test]
fn test_e2e_selfhost_cli_lsp_transport_hover_frame() {
    let body =
        r#"{"jsonrpc":"2.0","id":8,"result":{"range":[2,16,2,22],"contents":"defn square"}}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let source = "(defn square [x] x)\n(defn main [] (square 1) (square 2))";
    let harness = format!(
        r#"
(defn main []
  (let [params
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 99)
                2)
              17)
            "{source}")
        request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                8)
              (lsp-method-hover))
            params)]
    (print-string (run-lsp-transport-request request))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は hover request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M9c: selfhost/src/App/Cli.ls の LSP transport helper が references request を framed response にできること
#[test]
fn test_e2e_selfhost_cli_lsp_transport_references_frame() {
    let body = r#"{"jsonrpc":"2.0","id":10,"result":[[99,1,7],[99,2,16],[99,2,27]]}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let source = "(defn square [x] x)\n(defn main [] (square 1) (square 2))";
    let harness = format!(
        r#"
(defn main []
  (let [params
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 99)
                2)
              17)
            "{source}")
        request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                10)
              (lsp-method-references))
            params)]
    (print-string (run-lsp-transport-request request))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は references request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M9d: selfhost/src/App/Cli.ls の LSP transport helper が completion request を framed response にできること
#[test]
fn test_e2e_selfhost_cli_lsp_transport_completion_frame() {
    let body = r#"{"jsonrpc":"2.0","id":11,"result":[["defn",14,"defn"],["let",14,"let"],["if",14,"if"],["match",14,"match"],["do",14,"do"],["fn",14,"fn"],["module",14,"module"]]}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let harness = r#"
(defn main []
  (let [request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                11)
              (lsp-method-completion))
            0)]
    (print-string (run-lsp-transport-request request))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は completion request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M9e: selfhost/src/App/Cli.ls の LSP transport helper が formatting request を framed response にできること
#[test]
fn test_e2e_selfhost_cli_lsp_transport_formatting_frame() {
    let body = "{\"jsonrpc\":\"2.0\",\"id\":12,\"result\":[[1,1,2,4,\"(defn main [] 1)\\n\"]]}";
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let source = "(defn main []\n 1)";
    let harness = format!(
        r#"
(defn main []
  (let [params
          (vector-push
            (vector-push (vector-new 2) 77)
            "{source}")
        request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                12)
              (lsp-method-formatting))
            params)]
    (print-string (run-lsp-transport-request request))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は formatting request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M9f: selfhost/src/App/Cli.ls の LSP transport helper が rename request を framed response にできること
#[test]
fn test_e2e_selfhost_cli_lsp_transport_rename_frame() {
    let body = r#"{"jsonrpc":"2.0","id":13,"result":[[99,[[1,7,1,13,"cube"],[2,16,2,22,"cube"],[2,27,2,33,"cube"]]]]}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let source = "(defn square [x] x)\n(defn main [] (square 1) (square 2))";
    let harness = format!(
        r#"
(defn main []
  (let [params
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push (vector-new 5) 99)
                  2)
                17)
              "{source}")
            "cube")
        request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                13)
              (lsp-method-rename))
            params)]
    (print-string (run-lsp-transport-request request))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output, expected,
        "run-lsp-transport-request は rename request を framed response に変換すべき"
    );
}

/// TEST-CLI-02-M9: selfhost/src/App/Cli.ls の LSP transport helper sequence が shared-state で複数 request を捌けること
#[test]
fn test_e2e_selfhost_cli_lsp_transport_sequence_summary() {
    let init_body = r#"{"jsonrpc":"2.0","id":3,"result":[1,1,1,1,1,1,1]}"#;
    let init_frame = format!("Content-Length: {}\r\n\r\n{}", init_body.len(), init_body);
    let shutdown_body = r#"{"jsonrpc":"2.0","id":4,"result":0}"#;
    let shutdown_frame = format!(
        "Content-Length: {}\r\n\r\n{}",
        shutdown_body.len(),
        shutdown_body
    );
    let harness = r#"
(defn main []
  (let [init-request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                3)
              (lsp-method-initialize))
            0)
        shutdown-request
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                4)
              (lsp-method-shutdown))
            0)
        requests
          (vector-push
            (vector-push (vector-new 2) init-request)
            shutdown-request)
        summary (run-lsp-transport-sequence requests)
        frames (vector-get summary 0)]
    (do
      (print-string (vector-get frames 0))
      (print-string "\n---\n")
      (print-string (vector-get frames 1))
      (print-string "\n---\n")
      (print (vector-length frames))
      (print (vector-get summary 2)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "transport sequence output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], init_frame,
        "frame0 は initialize response であるべき"
    );
    assert_eq!(
        parts[1], shutdown_frame,
        "frame1 は shutdown response であるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "2"],
        "sequence summary は frame-count=2 / request-count=2 を返すべき"
    );
}

/// TEST-CLI-02-M10: publishDiagnostics notification が deterministic JSON/frame と request-count を返すこと
#[test]
fn test_e2e_selfhost_cli_lsp_transport_publish_diagnostics_frame() {
    let diagnostics_json =
        r#"[{"source":1,"severity":1,"rule":203,"line":2,"col":4,"messageHash":7003}]"#;
    let notification = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{{"uri":42,"diagnostics":{}}}}}"#,
        diagnostics_json
    );
    let expected_frame = format!(
        "Content-Length: {}\r\n\r\n{}",
        notification.len(),
        notification
    );
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        diag (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 6) 1)
                       203)
                     2)
                   4)
                 7003)
               1)
        diags (vector-push (vector-new 1) diag)
        params (vector-push (vector-push (vector-new 2) 42) diags)
        result (json-rpc-dispatch (lsp-method-publish-diagnostics) params state)]
    (do
      (print-string (vector-get result 1))
      (print-string "\n---\n")
      (print-string (lsp-render-publish-diagnostics-frame 42 diags))
      (print-string "\n---\n")
      (print (server-state-request-count state)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "publishDiagnostics output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], diagnostics_json,
        "handle-publish-diagnostics は deterministic diagnostics JSON を返すべき"
    );
    assert_eq!(
        parts[1], expected_frame,
        "lsp-render-publish-diagnostics-frame は notification frame を返すべき"
    );
    assert_eq!(
        parts[2].trim(),
        "1",
        "publishDiagnostics dispatch は request-count を 1 増やすべき"
    );
}

/// TEST-CLI-02-M11: didOpen dispatch + frame helper が deterministic に動くこと
#[test]
fn test_e2e_selfhost_cli_lsp_transport_didopen_frame() {
    let payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":16}}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload);
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        params (vector-push (vector-push (vector-new 2) 42) "(defn main [] 0)")
        result (json-rpc-dispatch (lsp-method-did-open) params state)]
    (do
      (print result)
      (print-string "\n---\n")
      (print-string (lsp-render-didopen-frame 42 result)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        2,
        "didOpen helper output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0].trim(),
        "16",
        "didOpen dispatch は source length=16 を返すべき"
    );
    assert_eq!(
        parts[1], expected,
        "didOpen frame は deterministic であるべき"
    );
}

/// TEST-CLI-02-M12: didOpen -> didChange shared-state sequence が framed notifications と state summary を返すこと
#[test]
fn test_e2e_selfhost_cli_lsp_transport_document_sequence() {
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":16}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload
    );
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":22}}"#;
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload
    );
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        open-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] 0)")
        change-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] (+ 0 1))")
        open-result (json-rpc-dispatch (lsp-method-did-open) open-params state)
        change-result (json-rpc-dispatch (lsp-method-did-change) change-params state)]
    (do
      (print-string (lsp-render-didopen-frame 42 open-result))
      (print-string "\n---\n")
      (print-string (lsp-render-didchange-frame 42 change-result))
      (print-string "\n---\n")
      (print (server-state-doc-count state))
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "document sequence output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "frame0 は didOpen notification であるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "frame1 は didChange notification であるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["1", "2", "22"],
        "sequence summary は doc-count=1 / request-count=2 / source-bytes=22 を返すべき"
    );
}

/// TEST-CLI-02-M12b: raw stdio frame helper が Content-Length header 付き initialize request を捌けること
#[test]
fn test_e2e_selfhost_cli_lsp_stdio_frame_initialize() {
    let body = r#"{"jsonrpc":"2.0","id":14,"result":[1,1,1,1,1,1,1]}"#;
    let expected = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let harness = format!(
        r#"
(defn main []
  (let [msg
          (vector-push
            (vector-push
              (vector-push
                (vector-push (vector-new 4) 2)
                14)
              (lsp-method-initialize))
            0)
        frame (vector-push (vector-push (vector-new 2) "{header}") msg)
        result (run-lsp-stdio-frame frame)]
    (do
      (print-string (vector-get result 0))
      (print-string "\n---\n")
      (print (vector-get result 1)))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        2,
        "stdio frame output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], expected,
        "run-lsp-stdio-frame は initialize frame を返すべき"
    );
    assert_eq!(
        parts[1].trim(),
        body.len().to_string(),
        "run-lsp-stdio-frame は parsed Content-Length を返すべき"
    );
}

/// TEST-CLI-02-M12c: raw stdio frame sequence helper が shared-state で didOpen -> didChange を捌けること
#[test]
fn test_e2e_selfhost_cli_lsp_stdio_frame_sequence() {
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":16}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload
    );
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":22}}"#;
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload
    );
    let open_header = format!("Content-Length: {}\r\n\r\n", open_payload.len());
    let change_header = format!("Content-Length: {}\r\n\r\n", change_payload.len());
    let harness = format!(
        r#"
(defn make-wire-msg [id method-id params]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 2)
        id)
      method-id)
    params))

(defn make-wire-frame [header msg]
  (vector-push (vector-push (vector-new 2) header) msg))

(defn main []
  (let [open-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] 0)")
        change-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] (+ 0 1))")
        open-frame (make-wire-frame "{open_header}" (make-wire-msg 0 (lsp-method-did-open) open-params))
        change-frame (make-wire-frame "{change_header}" (make-wire-msg 0 (lsp-method-did-change) change-params))
        frames (vector-push (vector-push (vector-new 2) open-frame) change-frame)
        summary (run-lsp-stdio-sequence frames)
        rendered (vector-get summary 0)]
    (do
      (print-string (vector-get rendered 0))
      (print-string "\n---\n")
      (print-string (vector-get rendered 1))
      (print-string "\n---\n")
      (print (vector-get summary 1))
      (print (vector-get summary 2))
      (print (vector-get summary 3)))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "stdio frame sequence output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "frame0 は didOpen notification であるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "frame1 は didChange notification であるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "22", &change_payload.len().to_string()],
        "stdio frame sequence summary は request-count=2 / source-length=22 / last-content-length を返すべき"
    );
}

/// TEST-CLI-02-M12e: didOpen/didChange は parse diagnostics refresh を publishDiagnostics frame で返すこと
#[test]
fn test_e2e_selfhost_cli_lsp_transport_document_sequence_publishes_diagnostics_refresh() {
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":1}}"#;
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":1001,"line":1,"col":1,"messageHash":0}]}}"#;
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":16}}"#;
    let change_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[]}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload,
        open_diagnostics.len(),
        open_diagnostics
    );
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload,
        change_diagnostics.len(),
        change_diagnostics
    );
    let harness = r#"
(defn make-request [id method-id params]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 2)
        id)
      method-id)
    params))

(defn main []
  (let [state (server-state-new)
        open-params (vector-push (vector-push (vector-new 2) 42) ")")
        change-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] 0)")
        open-frame (lsp-transport-dispatch-request state (make-request 0 (lsp-method-did-open) open-params))
        change-frame (lsp-transport-dispatch-request state (make-request 0 (lsp-method-did-change) change-params))]
    (do
      (print-string open-frame)
      (print-string "\n---\n")
      (print-string change-frame)
      (print-string "\n---\n")
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "transport diagnostics refresh output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "didOpen は parse diagnostics frame を後続させるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "didChange は diagnostics clear frame を後続させるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "16"],
        "transport diagnostics refresh summary は request-count=2 / latest-source-bytes=16 を返すべき"
    );
}

/// TEST-CLI-02-M12f: stdio body parser は spec 寄り didOpen/didChange params でも diagnostics refresh を返すこと
#[test]
fn test_e2e_selfhost_cli_lsp_stdio_body_document_sequence_spec_params_publishes_diagnostics_refresh()
 {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":")"}}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn main [] 0)"}]}}"#;
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":1}}"#;
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":1001,"line":1,"col":1,"messageHash":0}]}}"#;
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":16}}"#;
    let change_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[]}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload,
        open_diagnostics.len(),
        open_diagnostics
    );
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload,
        change_diagnostics.len(),
        change_diagnostics
    );
    let open_body_lsharp = open_body.replace('\\', "\\\\").replace('"', "\\\"");
    let change_body_lsharp = change_body.replace('\\', "\\\\").replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        open-body "{open_body_lsharp}"
        change-body "{change_body_lsharp}"
        open-frame (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message open-body)))
        change-frame (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message change-body)))]
    (do
      (print-string open-frame)
      (print-string "\n---\n")
      (print-string change-frame)
      (print-string "\n---\n")
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "spec document params diagnostics refresh output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "spec didOpen params でも parse diagnostics frame を後続させるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "spec didChange params でも diagnostics clear frame を後続させるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "16"],
        "spec document params summary は request-count=2 / latest-source-bytes=16 を返すべき"
    );
}

/// TEST-CLI-02-M12f2: didOpen/didChange は type diagnostics refresh を publishDiagnostics frame で返すこと
#[test]
fn test_e2e_selfhost_cli_lsp_transport_document_sequence_publishes_type_diagnostics_refresh() {
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":26}}"#;
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":2,"severity":1,"rule":2,"line":1,"col":1,"messageHash":2}]}}"#;
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":16}}"#;
    let change_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[]}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload,
        open_diagnostics.len(),
        open_diagnostics
    );
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload,
        change_diagnostics.len(),
        change_diagnostics
    );
    let harness = r#"
(defn make-request [id method-id params]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 2)
        id)
      method-id)
    params))

(defn main []
  (let [state (server-state-new)
        open-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] (if 42 1 0))")
        change-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] 0)")
        open-frame (lsp-transport-dispatch-request state (make-request 0 (lsp-method-did-open) open-params))
        change-frame (lsp-transport-dispatch-request state (make-request 0 (lsp-method-did-change) change-params))]
    (do
      (print-string open-frame)
      (print-string "\n---\n")
      (print-string change-frame)
      (print-string "\n---\n")
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "transport type diagnostics refresh output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "didOpen は type diagnostics frame を後続させるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "didChange は type diagnostics clear frame を後続させるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "16"],
        "transport type diagnostics refresh summary は request-count=2 / latest-source-bytes=16 を返すべき"
    );
}

/// TEST-CLI-02-M12f3: didOpen/didChange は lint diagnostics refresh を publishDiagnostics frame で返すこと
#[test]
fn test_e2e_selfhost_cli_lsp_transport_document_sequence_publishes_lint_diagnostics_refresh() {
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":29}}"#;
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":3,"severity":2,"rule":100,"line":1,"col":1,"messageHash":100}]}}"#;
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":16}}"#;
    let change_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[]}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload,
        open_diagnostics.len(),
        open_diagnostics
    );
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload,
        change_diagnostics.len(),
        change_diagnostics
    );
    let harness = r#"
(defn make-request [id method-id params]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 2)
        id)
      method-id)
    params))

(defn main []
  (let [state (server-state-new)
        open-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] (let [x 42] 0))")
        change-params (vector-push (vector-push (vector-new 2) 42) "(defn main [] 0)")
        open-frame (lsp-transport-dispatch-request state (make-request 0 (lsp-method-did-open) open-params))
        change-frame (lsp-transport-dispatch-request state (make-request 0 (lsp-method-did-change) change-params))]
    (do
      (print-string open-frame)
      (print-string "\n---\n")
      (print-string change-frame)
      (print-string "\n---\n")
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "transport lint diagnostics refresh output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "didOpen は lint diagnostics frame を後続させるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "didChange は lint diagnostics clear frame を後続させるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "16"],
        "transport lint diagnostics refresh summary は request-count=2 / latest-source-bytes=16 を返すべき"
    );
}

/// TEST-CLI-02-M12f4: stdio body parser は spec 寄り didOpen/didChange params でも type diagnostics refresh を返すこと
#[test]
fn test_e2e_selfhost_cli_lsp_stdio_body_document_sequence_spec_params_publishes_type_diagnostics_refresh()
 {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":"(defn main [] (if 42 1 0))"}}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn main [] 0)"}]}}"#;
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":26}}"#;
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":2,"severity":1,"rule":2,"line":1,"col":1,"messageHash":2}]}}"#;
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":16}}"#;
    let change_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[]}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload,
        open_diagnostics.len(),
        open_diagnostics
    );
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload,
        change_diagnostics.len(),
        change_diagnostics
    );
    let open_body_lsharp = open_body.replace('\\', "\\\\").replace('"', "\\\"");
    let change_body_lsharp = change_body.replace('\\', "\\\\").replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        open-body "{open_body_lsharp}"
        change-body "{change_body_lsharp}"
        open-frame (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message open-body)))
        change-frame (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message change-body)))]
    (do
      (print-string open-frame)
      (print-string "\n---\n")
      (print-string change-frame)
      (print-string "\n---\n")
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "spec document params type diagnostics refresh output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "spec didOpen params でも type diagnostics frame を後続させるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "spec didChange params でも type diagnostics clear frame を後続させるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "16"],
        "spec document params type summary は request-count=2 / latest-source-bytes=16 を返すべき"
    );
}

/// TEST-CLI-02-M12f5: stdio body parser は spec 寄り didOpen/didChange params でも lint diagnostics refresh を返すこと
#[test]
fn test_e2e_selfhost_cli_lsp_stdio_body_document_sequence_spec_params_publishes_lint_diagnostics_refresh()
 {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":"(defn main [] (let [x 42] 0))"}}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn main [] 0)"}]}}"#;
    let open_payload =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":29}}"#;
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":3,"severity":2,"rule":100,"line":1,"col":1,"messageHash":100}]}}"#;
    let change_payload = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":16}}"#;
    let change_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[]}}"#;
    let open_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_payload.len(),
        open_payload,
        open_diagnostics.len(),
        open_diagnostics
    );
    let change_frame = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        change_payload.len(),
        change_payload,
        change_diagnostics.len(),
        change_diagnostics
    );
    let open_body_lsharp = open_body.replace('\\', "\\\\").replace('"', "\\\"");
    let change_body_lsharp = change_body.replace('\\', "\\\\").replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        open-body "{open_body_lsharp}"
        change-body "{change_body_lsharp}"
        open-frame (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message open-body)))
        change-frame (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message change-body)))]
    (do
      (print-string open-frame)
      (print-string "\n---\n")
      (print-string change-frame)
      (print-string "\n---\n")
      (print (server-state-request-count state))
      (print (server-state-source-length state)))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let parts: Vec<&str> = output.split("\n---\n").collect();

    assert_eq!(
        parts.len(),
        3,
        "spec document params lint diagnostics refresh output format が不正: {:?}",
        output
    );
    assert_eq!(
        parts[0], open_frame,
        "spec didOpen params でも lint diagnostics frame を後続させるべき"
    );
    assert_eq!(
        parts[1], change_frame,
        "spec didChange params でも lint diagnostics clear frame を後続させるべき"
    );
    assert_eq!(
        parts[2].trim().lines().collect::<Vec<_>>(),
        vec!["2", "16"],
        "spec document params lint summary は request-count=2 / latest-source-bytes=16 を返すべき"
    );
}

/// TEST-CLI-02-M12g: stdio body parser は spec 寄り hover params の position.character を col として読むこと
#[test]
fn test_e2e_selfhost_cli_lsp_stdio_body_hover_spec_position_character_params() {
    let hover_body = r#"{"jsonrpc":"2.0","id":66,"method":"textDocument/hover","params":{"textDocument":{"uri":42},"position":{"line":1,"character":38}}}"#;
    let hover_body_lsharp = hover_body.replace('\\', "\\\\").replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [msg (lsp-stdio-body-message "{hover_body_lsharp}")
        params (vector-get msg 3)]
    (do
      (print (vector-get params 0))
      (print (vector-get params 1))
      (print (vector-get params 2)))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["42", "1", "38"],
        "spec hover params は [uri,line,col]=[42,1,38] として読まれるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M12h: stdio body parser は spec 寄り rename params の position.character と newName を読むこと
#[test]
fn test_e2e_selfhost_cli_lsp_stdio_body_rename_spec_position_character_params() {
    let rename_body = r#"{"jsonrpc":"2.0","id":70,"method":"textDocument/rename","params":{"textDocument":{"uri":42},"position":{"line":1,"character":38},"newName":"cube"}}"#;
    let rename_body_lsharp = rename_body.replace('\\', "\\\\").replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [msg (lsp-stdio-body-message "{rename_body_lsharp}")
        params (vector-get msg 3)]
    (do
      (print (vector-get params 0))
      (print (vector-get params 1))
      (print (vector-get params 2))
      (print-string (vector-get params 4))
      (print-string "\n"))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["42", "1", "38", "cube"],
        "spec rename params は [uri,line,col,newName]=[42,1,38,cube] として読まれるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-M12d: raw stdio wire helper が長めの open/hover/change/completion/formatting 系列を最後まで捌けること
#[test]
fn test_e2e_selfhost_cli_lsp_stdio_wire_repeated_sequence() {
    let render_lsp_wire_frame =
        |body: &str| format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let repeat_rendered_frames = |frames: &[String], iterations: usize| {
        let mut rendered = String::new();
        for _ in 0..iterations {
            for frame in frames {
                rendered.push_str(frame);
            }
        }
        rendered
    };

    let open_source = "(defn helper [] 1)\n(defn main [] (helper 1))";
    let change_source = "(defn helper [] 1)\n(defn main []  (he))";
    let iterations = 12usize;

    let init_body = r#"{"jsonrpc":"2.0","id":80,"method":"initialize","params":0}"#;
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":81,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":21}}"#;
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        change_source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":82,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":23}}"#;
    let formatting_body =
        r#"{"jsonrpc":"2.0","id":83,"method":"textDocument/formatting","params":{"uri":42}}"#;

    let stdin = format!(
        "{}{}",
        render_lsp_wire_frame(init_body),
        repeat_rendered_frames(
            &[
                render_lsp_wire_frame(&open_body),
                render_lsp_wire_frame(hover_body),
                render_lsp_wire_frame(&change_body),
                render_lsp_wire_frame(completion_body),
                render_lsp_wire_frame(formatting_body),
            ],
            iterations
        )
    );

    let harness = format!(
        r#"
(defn main []
  (let [wire {stdin:?}]
    (print-string (run-lsp-stdio-wire wire))))
"#
    );

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let frames = parse_lsp_stdio_frames(&output);
    let init_response = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 80,
        "result": [1, 1, 1, 1, 1, 1, 1]
    });
    let open_response = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "uri": 42,
            "sourceBytes": open_source.len()
        }
    });
    let change_response = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "uri": 42,
            "sourceBytes": change_source.len()
        }
    });
    let first_open_diagnostics = frames
        .get(2)
        .cloned()
        .expect("1 回目 didOpen diagnostics frame が必要");
    let first_hover_response = frames.get(3).cloned().expect("1 回目 hover frame が必要");
    let first_change_diagnostics = frames
        .get(5)
        .cloned()
        .expect("1 回目 didChange diagnostics frame が必要");
    let first_completion_response = frames
        .get(6)
        .cloned()
        .expect("1 回目 completion frame が必要");
    let first_formatting_response = frames
        .get(7)
        .cloned()
        .expect("1 回目 formatting frame が必要");

    assert_eq!(
        frames.len(),
        1 + (iterations * 7),
        "raw stdio wire helper は initialize + 各反復 7 frame を返すべき"
    );
    assert_eq!(
        frames[0], init_response,
        "frame0 は initialize response であるべき"
    );

    assert_eq!(
        first_open_diagnostics["method"],
        serde_json::json!("textDocument/publishDiagnostics"),
        "didOpen 後は publishDiagnostics frame を返すべき"
    );
    assert_eq!(
        first_open_diagnostics["params"]["uri"],
        serde_json::json!(42),
        "didOpen diagnostics は uri=42 を対象にすべき"
    );
    assert!(
        first_open_diagnostics["params"]["diagnostics"].is_array(),
        "didOpen diagnostics は配列であるべき"
    );
    assert_eq!(
        first_change_diagnostics["method"],
        serde_json::json!("textDocument/publishDiagnostics"),
        "didChange 後は publishDiagnostics frame を返すべき"
    );
    assert_eq!(
        first_change_diagnostics["params"]["uri"],
        serde_json::json!(42),
        "didChange diagnostics は uri=42 を対象にすべき"
    );
    assert!(
        first_change_diagnostics["params"]["diagnostics"].is_array(),
        "didChange diagnostics は配列であるべき"
    );
    assert_eq!(
        first_hover_response["id"],
        serde_json::json!(81),
        "hover frame は id=81 を保持すべき"
    );
    assert!(
        first_hover_response["result"].is_object(),
        "hover frame は result object を返すべき"
    );
    assert_eq!(
        first_completion_response["id"],
        serde_json::json!(82),
        "completion frame は id=82 を保持すべき"
    );
    assert!(
        first_completion_response["result"].is_array(),
        "completion frame は result array を返すべき"
    );
    assert_eq!(
        first_formatting_response["id"],
        serde_json::json!(83),
        "formatting frame は id=83 を保持すべき"
    );
    assert!(
        first_formatting_response["result"].is_array(),
        "formatting frame は result array を返すべき"
    );

    for iteration in 0..iterations {
        let base = 1 + (iteration * 7);
        assert_eq!(
            frames[base], open_response,
            "iteration {} の didOpen response が不正",
            iteration
        );
        assert_eq!(
            frames[base + 1],
            first_open_diagnostics,
            "iteration {} の didOpen diagnostics は決定的であるべき",
            iteration
        );
        assert_eq!(
            frames[base + 2],
            first_hover_response,
            "iteration {} の hover response が不正",
            iteration
        );
        assert_eq!(
            frames[base + 3],
            change_response,
            "iteration {} の didChange response が不正",
            iteration
        );
        assert_eq!(
            frames[base + 4],
            first_change_diagnostics,
            "iteration {} の didChange diagnostics は決定的であるべき",
            iteration
        );
        assert_eq!(
            frames[base + 5],
            first_completion_response,
            "iteration {} の completion response が不正",
            iteration
        );
        assert_eq!(
            frames[base + 6],
            first_formatting_response,
            "iteration {} の formatting response が不正",
            iteration
        );
    }
}

/// TEST-CLI-02-N: selfhost/src/App/Cli.ls の run-test-source が TestRunner.generate-tests を呼べること
#[test]
fn test_e2e_selfhost_cli_test_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-test-source "(defn main [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["examples:0", "invariants:0", "failures:0", "0"],
        "run-test-source は labeled summary と success=0 を返すべき"
    );
}

/// TEST-CLI-02-O: selfhost/src/App/Cli.ls の run-test が file-path から source を読めること
#[test]
fn test_e2e_selfhost_cli_test_file_handler() {
    let dir =
        std::env::temp_dir().join(format!("lsharp_test_cli_test_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-test "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["examples:0", "invariants:0", "failures:0", "0"],
        "run-test は labeled summary と success=0 を返すべき"
    );
}

/// TEST-CLI-02-O2: selfhost/src/Tools/Test/TestRunner.ls が supported subset の metadata suite を実行できること
#[test]
fn test_e2e_selfhost_test_runner_extracts_supported_metadata_suite() {
    let harness = r#"
(defn main []
  (let [src "(defn abs [x] :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"
        examples (extract-examples src)
        invariants (extract-invariants src)]
    (do
      (print (vector-length examples))
      (print (vector-length invariants))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["2", "1"],
        "extract-examples / extract-invariants は supported metadata を 2 / 1 件抽出できるべき"
    );
}

/// TEST-CLI-02-O2b: selfhost/src/Tools/Test/TestRunner.ls が supported subset の metadata suite を実行できること
#[test]
fn test_e2e_selfhost_test_runner_executes_examples_only() {
    let harness = r#"
(defn main []
  (let [src "(defn abs [x] :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)] (if (< x 0) (- 0 x) x))"
        program (parse-program src)
        results (run-examples program (extract-examples src))
        example0 (vector-get results 0)
        example1 (vector-get results 1)]
    (do
      (print (vector-length results))
      (print (vector-get example0 1))
      (print (vector-get example1 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["2", "1", "1"],
        "run-examples は supported examples を 2 件成功として実行できるべき"
    );
}

/// TEST-CLI-02-O2c: selfhost/src/Tools/Test/TestRunner.ls が supported invariant suite を materialize できること
#[test]
fn test_e2e_selfhost_test_runner_executes_invariant_only() {
    let harness = r#"
(defn main []
  (let [src "(defn abs [x] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"
        suite (generate-tests-from-source src)
        results (vector-get suite 1)
        invariant0 (vector-get results 0)]
    (do
      (print (vector-length results))
      (print (vector-get invariant0 1))
      (print (vector-get invariant0 2))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1", "1", "5"],
        "run-invariants は supported invariant を 5 サンプル計画付きで materialize できるべき"
    );
}
/// TEST-CLI-02-O2d: selfhost/src/Tools/Test/TestRunner.ls が supported subset の metadata suite を実行できること
#[test]
fn test_e2e_selfhost_test_runner_executes_supported_metadata_suite() {
    let harness = r#"
(defn main []
  (let [src "(defn abs [x] :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"
        suite (generate-tests-from-source src)
        examples (vector-get suite 0)
        invariants (vector-get suite 1)
        example0 (vector-get examples 0)
        example1 (vector-get examples 1)
        invariant0 (vector-get invariants 0)]
    (do
      (print (vector-length examples))
      (print (vector-length invariants))
      (print (vector-get example0 1))
      (print (vector-get example1 1))
      (print (vector-get invariant0 1))
      (print (vector-get invariant0 2))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["2", "1", "1", "1", "1", "5"],
        "generate-tests-from-source は 2 example + 1 invariant を実行し、invariant は 5 サンプル通過を返すべき"
    );
}

/// TEST-CLI-02-O3: selfhost/src/App/Cli.ls の run-test-source が supported subset の metadata を成功終了できること
#[test]
fn test_e2e_selfhost_cli_test_source_metadata_pass() {
    let harness = r#"
(defn main []
  (let [src "(defn abs [x] :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"]
    (do
      (print (run-test-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["examples:2", "invariants:1", "failures:0", "0"],
        "run-test-source は passing metadata suite に labeled summary と success=0 を返すべき"
    );
}

/// TEST-CLI-02-O4: selfhost/src/App/Cli.ls の run-test-source が failing example を runtime error にできること
#[test]
fn test_e2e_selfhost_cli_test_source_metadata_fail() {
    let harness = r#"
(defn main []
  (let [src "(defn dec [x] :example [(= (dec 2) 3)] (- x 1))"]
    (do
      (print (run-test-source src 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["examples:1", "invariants:0", "failures:1", "2"],
        "run-test-source は failing example に labeled summary と runtime-error=2 を返すべき"
    );
}

/// TEST-CLI-02-O5: selfhost/src/App/Cli.ls の run-test が file-path 経由の metadata suite も実行できること
#[test]
fn test_e2e_selfhost_cli_test_file_handler_metadata_pass() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = project_root.join("target").join(format!(
        "e2e_selfhost_cli_test_metadata_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("input.ls"),
        "(defn abs [x] :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)] :invariant (>= result 0) (if (< x 0) (- 0 x) x))",
    )
    .unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-test "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["examples:2", "invariants:1", "failures:0", "0"],
        "run-test は file-path 経由でも labeled summary を返せるべき"
    );
}

/// TEST-CLI-02-P: selfhost/src/App/Cli.ls の run-review-source が review title/body を返せること
#[test]
fn test_e2e_selfhost_cli_review_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-review-source "(defn main [] (let [x 42] 0))" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "1",
            "unused-let",
            "diagnostics:1,first-body:let binding x is not used",
            "warning",
            "L0001@1:1",
            "0"
        ],
        "run-review-source は review count/title/body/severity/code-location と success=0 を返すべき"
    );
}

/// TEST-CLI-02-Q: selfhost/src/App/Cli.ls の run-review が file-path から review title/body を返せること
#[test]
fn test_e2e_selfhost_cli_review_file_handler() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_review_file_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] (let [x 42] 0))").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-review "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "1",
            "unused-let",
            "diagnostics:1,first-body:let binding x is not used",
            "warning",
            "L0001@1:1",
            "0"
        ],
        "run-review は review count/title/body/severity/code-location と success=0 を返すべき"
    );
}

/// TEST-CLI-02-Q2: selfhost/src/App/Cli.ls の run-review-source が empty-do rule も返せること
#[test]
fn test_e2e_selfhost_cli_review_source_empty_do() {
    let harness = r#"
(defn main []
  (do
    (print (run-review-source "(defn main [] (do))" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "1",
            "empty-do",
            "diagnostics:1,first-body:do block has no expressions",
            "warning",
            "L0002@1:1",
            "0",
        ],
        "run-review-source は empty-do rule でも review summary/severity/code-location を返すべき"
    );
}

/// TEST-CLI-02-Q3: selfhost/src/App/Cli.ls の run-review-source が schema-object JSON を返せること
#[test]
fn test_e2e_selfhost_cli_review_source_json_snapshot() {
    let harness = r#"
(defn main []
  (do
    (print (run-review-source "(defn first [] (let [unused 42] 0)) (defn second [] (do))" 1))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    let actual: Value =
        serde_json::from_str(lines[0]).expect("run-review-source json line は valid JSON");

    assert_eq!(
        actual,
        doctools_json_snapshot("review-schema-object.json"),
        "run-review-source json output は representative review schema snapshot と一致するべき"
    );
    assert_eq!(
        lines[1], "0",
        "run-review-source json mode は success=0 を返すべき"
    );
}

/// TEST-CLI-02-R: selfhost/src/App/Cli.ls の run-doc-source が DocTools.generate を呼べること
#[test]
fn test_e2e_selfhost_cli_doc_source_core() {
    let harness = r#"
(defn main []
  (do
    (print (run-doc-source "(defn main [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["module-global", "functions:1,types:0,first-fn:main", "0"],
        "run-doc-source は deterministic な title/body と success=0 を返すべき"
    );
}

/// TEST-CLI-02-R2: run-doc-source output を snapshot に固定すること
#[test]
fn test_e2e_selfhost_cli_doc_source_snapshot() {
    let harness = r#"
(defn main []
  (do
    (print (run-doc-source "(defn main [] 42)" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert_cli_text_snapshot(
        &output,
        "doc-source-basic.txt",
        "run-doc-source output は representative text snapshot と一致するべき",
    );
}

/// TEST-CLI-02-R3: selfhost/src/App/Cli.ls の run-doc-source が schema-object JSON を返せること
#[test]
fn test_e2e_selfhost_cli_doc_source_json_snapshot() {
    let harness = r#"
(defn main []
  (do
    (print (run-doc-source "(module Demo (defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y)) (type Doc Int) (type-alias Alias Int))" 1))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    let actual: Value =
        serde_json::from_str(lines[0]).expect("run-doc-source json line は valid JSON");

    assert_eq!(
        actual,
        doctools_json_snapshot("doc-output-schema-object.json"),
        "run-doc-source json output は representative doc-output schema snapshot と一致するべき"
    );
    assert_eq!(
        lines[1], "0",
        "run-doc-source json mode は success=0 を返すべき"
    );
}

/// TEST-CLI-02-S: selfhost/src/App/Cli.ls の run-doc が file-path から source を読めること
#[test]
fn test_e2e_selfhost_cli_doc_file_handler() {
    let dir = std::env::temp_dir().join(format!("lsharp_test_cli_doc_file_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-doc "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["module-global", "functions:1,types:0,first-fn:main", "0"],
        "run-doc は file-path 経由でも deterministic な title/body と success=0 を返すべき"
    );
}

/// TEST-CLI-02-T: selfhost/src/App/Cli.ls の run-doc-ack が file-path から source を読めること
#[test]
fn test_e2e_selfhost_cli_doc_ack_file_handler() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_doc_ack_file_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-doc-ack "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "ack:recorded",
            "module-global",
            "functions:1,types:0,first-fn:main",
            "; Doc-Reviewed-By: anonymous",
            "0",
        ],
        "run-doc-ack は ack status と title/body と trailer と success=0 を返すべき"
    );
}

/// TEST-CLI-02-T2: selfhost/src/App/Cli.ls の run-doc-ack が trailer-only mode を返せること
#[test]
fn test_e2e_selfhost_cli_doc_ack_file_handler_trailer_only() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_doc_ack_trailer_only_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-doc-ack "input.ls" 1))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["; Doc-Reviewed-By: anonymous", "0"],
        "run-doc-ack trailer-only mode は comment trailer のみを返すべき"
    );
}

/// TEST-CLI-02-U: selfhost/src/App/Cli.ls の run-doc-check が file-path から source を読めること
#[test]
fn test_e2e_selfhost_cli_doc_check_file_handler() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_doc_check_file_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-doc-check "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "status:ok",
            "module-global",
            "functions:1,types:0,first-fn:main",
            "; Doc-Review-Status: Passed",
            "; Doc-Reviewed-By: anonymous",
            "0",
        ],
        "run-doc-check は status と title/body と trailer と success=0 を返すべき"
    );
}

/// TEST-CLI-02-U2: selfhost/src/App/Cli.ls の run-doc-check strict mode が valid trailer を受理すること
#[test]
fn test_e2e_selfhost_cli_doc_check_file_handler_strict_success() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_doc_check_strict_success_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("input.ls"),
        "(defn main [] 42)\n; Doc-Review-Status: Passed\n; Doc-Reviewed-By: anonymous\n",
    )
    .unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-doc-check "input.ls" 1))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "status:ok",
            "module-global",
            "functions:1,types:0,first-fn:main",
            "; Doc-Review-Status: Passed",
            "; Doc-Reviewed-By: anonymous",
            "0",
        ],
        "run-doc-check strict mode は valid trailer comment を受理するべき"
    );
}

/// TEST-CLI-02-U3: selfhost/src/App/Cli.ls の run-doc-check strict mode が invalid trailer を拒否すること
#[test]
fn test_e2e_selfhost_cli_doc_check_file_handler_strict_missing_trailer_fails() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_doc_check_strict_fail_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)\n").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (run-doc-check "input.ls" 1))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "error: invalid doc trailer: expected trailing comment lines",
            "1"
        ],
        "run-doc-check strict mode は trailer 欠落時に compile error を返すべき"
    );
}

/// TEST-CLI-02-V: exit-code-success が 0 を返すこと
///
/// CLI-02 contract parity: 終了コードの公開 API を検証
#[test]
fn test_e2e_selfhost_cli_exit_code_success() {
    let harness = r#"
(defn main []
  (do
    (print (exit-code-success))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.last().unwrap(),
        &"0",
        "exit-code-success は 0 であるべき"
    );
}

/// TEST-CLI-02-W: exit-code-compile-error が 1 を返すこと
///
/// CLI-02 contract parity: コンパイルエラー終了コード
#[test]
fn test_e2e_selfhost_cli_exit_code_compile_error() {
    let harness = r#"
(defn main []
  (do
    (print (exit-code-compile-error))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.last().unwrap(),
        &"1",
        "exit-code-compile-error は 1 であるべき"
    );
}

/// TEST-CLI-02-X: 不明コマンドで run-command が 127 を返すこと
///
/// CLI-02 contract parity: 不明コマンドの終了コード
#[test]
fn test_e2e_selfhost_cli_exit_code_unknown_command() {
    let harness = r#"
(defn main []
  (let [code (run-command "nonexistent" "" 0)]
    (do
      (print code)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    // run-command は cli-stderr でエラーを出力してから 127 を返す
    assert_eq!(
        lines.last().unwrap(),
        &"127",
        "不明コマンドの終了コードは 127 であるべき"
    );
    assert!(
        output.contains("error: unknown command: nonexistent"),
        "不明コマンドでエラーメッセージが出力されるべき: {:?}",
        output
    );
}

/// TEST-CLI-02-Y: help-text が 13 コマンドすべてを列挙すること
///
/// CLI-02 contract parity: ヘルプ出力の完全性
#[test]
fn test_e2e_selfhost_cli_help_lists_all_commands() {
    let harness = r#"
(defn main []
  (do
    (print-string (help-text))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    let commands = [
        "parse",
        "check",
        "compile",
        "build",
        "test",
        "review",
        "doc-ack",
        "doc-check",
        "install",
        "repl",
        "lsp",
        "fmt",
        "doc",
    ];
    let mut count = 0;
    for cmd in &commands {
        if output.contains(cmd) {
            count += 1;
        }
    }
    assert_eq!(
        count, 13,
        "help テキストは 13 コマンドすべてを列挙すべき (found {})",
        count
    );
}

/// TEST-CLI-02-Z: version-text が `lsharp x.y.z` 形式であること
///
/// CLI-02 contract parity: バージョン出力形式
#[test]
fn test_e2e_selfhost_cli_version_format() {
    let harness = r#"
(defn main []
  (do
    (print-string (version-text))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let trimmed = output.trim();

    assert!(
        trimmed.starts_with("lsharp "),
        "バージョンは 'lsharp ' で始まるべき: {:?}",
        trimmed
    );
    let version_part = trimmed.strip_prefix("lsharp ").unwrap();
    let parts: Vec<&str> = version_part.split('.').collect();
    assert_eq!(
        parts.len(),
        3,
        "バージョンは x.y.z 形式であるべき: {}",
        version_part
    );
}

/// TEST-CLI-02-AA: cli-stdout / cli-stderr の出力チャネル分離
///
/// CLI-02 contract parity: stdout は結果出力、stderr は "error: " プレフィックス付き
#[test]
fn test_e2e_selfhost_cli_stdout_stderr_separation() {
    let harness = r#"
(defn main []
  (do
    (cli-stdout "program output")
    (cli-stderr "diagnostic message")
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);

    assert!(
        output.contains("program output"),
        "cli-stdout の出力が含まれるべき: {:?}",
        output
    );
    assert!(
        output.contains("error: diagnostic message"),
        "cli-stderr の出力は 'error: ' プレフィックスを持つべき: {:?}",
        output
    );
    // stdout と stderr が別行に出力されることを確認
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 2,
        "cli-stdout と cli-stderr は別行に出力されるべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-AB: main-dispatch が parse file handler を entrypoint helper 経由で呼べること
#[test]
fn test_e2e_selfhost_cli_main_dispatch_parse_file() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_dispatch_parse_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let harness = r#"
(defn main []
  (do
    (print (main-dispatch "parse" "input.ls" 0))
    0))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "main-dispatch parse 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "decls:1");
    assert_eq!(lines[1], "first-decl:defn");
    assert_eq!(lines[2], "first-body:int");
    assert_eq!(lines[3], "diagnostics:0");
    assert_eq!(lines[4], "0");
}

/// TEST-CLI-02-AC: main-dispatch が help/version/unknown surface を保つこと
#[test]
fn test_e2e_selfhost_cli_main_dispatch_command_surface() {
    let harness = r#"
(defn main []
  (let [help-code (main-dispatch "--help" "" 0)
        version-code (main-dispatch "--version" "" 0)
        unknown-code (main-dispatch "nonexistent" "" 0)]
    (do
      (print-string "\nhelp-code:")
      (print help-code)
      (print-string "version-code:")
      (print version-code)
      (print-string "unknown-code:")
      (print unknown-code)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert!(
        output.contains("Usage: lsharp <command>"),
        "main-dispatch help は usage を出力すべき: {:?}",
        output
    );
    assert!(
        output.contains("lsharp 0.1.0"),
        "main-dispatch version は version text を出力すべき: {:?}",
        output
    );
    assert!(
        output.contains("error: unknown command: nonexistent"),
        "main-dispatch unknown は error surface を保つべき: {:?}",
        output
    );
    assert!(
        output.contains("help-code:0"),
        "help は success=0 を返すべき: {:?}",
        output
    );
    assert!(
        output.contains("version-code:0"),
        "version は success=0 を返すべき: {:?}",
        output
    );
    assert!(
        output.contains("unknown-code:127"),
        "unknown command は 127 を返すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AD: actual Cli main は引数なし実行で help surface を返すこと
#[test]
fn test_e2e_selfhost_cli_main_no_args_shows_help() {
    let output = compile_and_run(selfhost_cli_runtime_bundle());

    assert!(
        output.contains("Usage: lsharp <command>"),
        "Cli main の no-args 実行は help usage を返すべき: {:?}",
        output
    );
    assert!(
        output.contains("Commands:"),
        "Cli main の no-args 実行は command list を返すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AE: actual Cli main は argv 経由で --version を処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_args_version() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["--version"]);

    assert!(
        output.contains("lsharp 0.1.0"),
        "Cli main の argv 実行は --version を処理すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AE2: actual Cli main は argv 経由で -v alias を処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_args_short_version() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["-v"]);

    assert!(
        output.contains("lsharp 0.1.0"),
        "Cli main の argv 実行は -v alias を処理すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AF: actual Cli main は argv 経由で parse file command を処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_args_parse_file() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_parse_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["parse", "input.ls"],
    );
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "Cli main parse argv 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "decls:1");
    assert_eq!(lines[1], "first-decl:defn");
    assert_eq!(lines[2], "first-body:int");
    assert_eq!(lines[3], "diagnostics:0");
}

/// TEST-CLI-02-AF2: actual Cli main は argv 経由で compile file command を処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_args_compile_file() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_compile_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["compile", "input.ls"],
    );
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 1,
        "Cli main compile argv 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "Cli main compile argv は wasm-size:<n> を返すべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-AF2B: actual Cli main は nested import fixture の compile を import-aware helper と同じ summary で返すこと
#[test]
fn test_e2e_selfhost_cli_main_with_args_compile_file_multifile_nested_imports() {
    let dir = cli_test_fixture_dir("main_compile_multifile_nested");
    write_cli_fixture_files(&dir, &cli_multifile_nested_fixture_files());
    let expected_size = run_cli_multifile_helper_size(&dir, "main.ls", 0);

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["compile", "main.ls"],
    );
    let _ = std::fs::remove_dir_all(&dir);

    let output_line = output
        .trim()
        .lines()
        .next()
        .expect("Cli main compile multi-file output が必要");
    let output_size =
        parse_wasm_size_line(output_line, "Cli main compile multi-file nested fixture");
    assert!(
        output_size == expected_size,
        "Cli main compile は import-aware helper と同じ wasm-size を返すべき: cli={output_size}, helper={expected_size}"
    );
}

/// TEST-CLI-02-AF3: actual Cli main は argv 経由で build file command を処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_args_build_file() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_build_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["build", "input.ls"],
    );
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 1,
        "Cli main build argv 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "Cli main build argv は wasm-size:<n> を返すべき: {:?}",
        lines
    );
}

/// TEST-CLI-02-AF3B: actual Cli main は nested import fixture の build を import-aware helper と同じ summary で返すこと
#[test]
fn test_e2e_selfhost_cli_main_with_args_build_file_multifile_nested_imports() {
    let dir = cli_test_fixture_dir("main_build_multifile_nested");
    write_cli_fixture_files(&dir, &cli_multifile_nested_fixture_files());
    let expected_size = run_cli_multifile_helper_size(&dir, "main.ls", 0);

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["build", "main.ls"],
    );
    let _ = std::fs::remove_dir_all(&dir);

    let output_line = output
        .trim()
        .lines()
        .next()
        .expect("Cli main build multi-file output が必要");
    let output_size = parse_wasm_size_line(output_line, "Cli main build multi-file nested fixture");
    assert!(
        output_size == expected_size,
        "Cli main build は import-aware helper と同じ wasm-size を返すべき: cli={output_size}, helper={expected_size}"
    );
}

/// TEST-CLI-02-AF4: actual Cli main は compile <file> -o <path> で output file を書けること
#[test]
fn test_e2e_selfhost_cli_main_with_args_compile_output_path() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_compile_output_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["compile", "input.ls", "-o", "out.txt"],
    );
    let written = std::fs::read_to_string(dir.join("out.txt")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 1,
        "Cli main compile -o 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "Cli main compile -o は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    assert_eq!(
        written.trim(),
        lines[0],
        "compile -o は stdout summary を output file にも書くべき"
    );
}

/// TEST-CLI-02-AF5: actual Cli main は build <file> --output <path> で output file を書けること
#[test]
fn test_e2e_selfhost_cli_main_with_args_build_output_path() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_build_output_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["build", "input.ls", "--output", "build.txt"],
    );
    let written = std::fs::read_to_string(dir.join("build.txt")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 1,
        "Cli main build --output 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "Cli main build --output は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    assert_eq!(
        written.trim(),
        lines[0],
        "build --output は stdout summary を output file にも書くべき"
    );
}

/// TEST-CLI-02-AF6: actual Cli main は compile <file> --target ... -o <path> を併用できること
#[test]
fn test_e2e_selfhost_cli_main_with_args_compile_target_and_output_path() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_compile_target_output_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &[
            "compile",
            "input.ls",
            "--target",
            "wasi-component",
            "-o",
            "targeted.txt",
        ],
    );
    let written = std::fs::read_to_string(dir.join("targeted.txt")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 1,
        "Cli main compile --target ... -o 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "Cli main compile --target ... -o は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    assert_eq!(
        written.trim(),
        lines[0],
        "compile --target ... -o は stdout summary を output file にも書くべき"
    );
}

/// TEST-CLI-02-AF6B: actual Cli main は preview1/component target ごとに異なる wasm-size を返すこと
#[test]
fn test_e2e_selfhost_cli_main_with_args_compile_target_changes_wasm_size() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_compile_target_size_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let preview1_output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["compile", "input.ls", "--target", "wasi-preview1"],
    );
    let component_output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &["compile", "input.ls", "--target", "wasi-component"],
    );
    let _ = std::fs::remove_dir_all(&dir);

    let preview1_line = preview1_output
        .trim()
        .lines()
        .next()
        .expect("preview1 compile output が必要");
    let component_line = component_output
        .trim()
        .lines()
        .next()
        .expect("component compile output が必要");
    assert!(
        preview1_line.starts_with("wasm-size:"),
        "preview1 compile output は wasm-size:<n> を返すべき: {:?}",
        preview1_output
    );
    assert!(
        component_line.starts_with("wasm-size:"),
        "component compile output は wasm-size:<n> を返すべき: {:?}",
        component_output
    );

    let preview1_size: i64 = preview1_line["wasm-size:".len()..]
        .parse()
        .expect("preview1 wasm size は整数であるべき");
    let component_size: i64 = component_line["wasm-size:".len()..]
        .parse()
        .expect("component wasm size は整数であるべき");
    assert!(
        preview1_size > component_size,
        "Cli main compile は preview1/component target を size に反映するべき: preview1={preview1_size}, component={component_size}"
    );
}

/// TEST-CLI-02-AF7: actual Cli main は build <file> --output <path> --target wasm を併用できること
#[test]
fn test_e2e_selfhost_cli_main_with_args_build_output_path_and_target_alias() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_build_output_target_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)").unwrap();

    let output = compile_and_run_with_dir_and_args(
        selfhost_cli_runtime_bundle(),
        &dir,
        &[
            "build",
            "input.ls",
            "--output",
            "build-target.txt",
            "--target",
            "wasm",
        ],
    );
    let written = std::fs::read_to_string(dir.join("build-target.txt")).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 1,
        "Cli main build --output ... --target 出力が不足: {:?}",
        lines
    );
    assert!(
        lines[0].starts_with("wasm-size:"),
        "Cli main build --output ... --target は wasm-size:<n> を返すべき: {:?}",
        lines
    );
    assert_eq!(
        written.trim(),
        lines[0],
        "build --output ... --target は stdout summary を output file にも書くべき"
    );
}

/// TEST-CLI-02-AG: actual Cli main は subcommand --help を処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_args_subcommand_help() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["parse", "--help"]);

    assert!(
        output.contains("parse <file> - Parse source and show AST"),
        "Cli main は subcommand help text を返すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AH: actual Cli main は `-h` alias で global help を返せること
#[test]
fn test_e2e_selfhost_cli_main_with_args_short_help() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["-h"]);

    assert!(
        output.contains("Usage: lsharp <command>"),
        "Cli main は -h alias で global help を返すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AI: actual Cli main は `help <subcommand>` を処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_help_command() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["help", "parse"]);

    assert!(
        output.contains("parse <file> - Parse source and show AST"),
        "Cli main は help subcommand surface を返すべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AJ: actual Cli main は help compile に output option surface を含めること
#[test]
fn test_e2e_selfhost_cli_main_with_help_compile_output_option() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["help", "compile"]);

    assert!(
        output.contains("compile <file> [-o <file>]"),
        "Cli main は compile help に output option surface を含めるべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AK: actual Cli main は build --help に output option surface を含めること
#[test]
fn test_e2e_selfhost_cli_main_with_build_subcommand_help_output_option() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["build", "--help"]);

    assert!(
        output.contains("build <file> [--output <file>]"),
        "Cli main は build help に output option surface を含めるべき: {:?}",
        output
    );
}

/// TEST-CLI-02-AL: actual Cli main は `lsp --stdio` で stdin の initialize frame を処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_initialize() {
    let request_body = r#"{"jsonrpc":"2.0","id":21,"method":"initialize","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body = r#"{"jsonrpc":"2.0","id":21,"result":[1,1,1,1,1,1,1]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で initialize frame をそのまま返すべき"
    );
}

/// TEST-CLI-02-AM: actual Cli main は `lsp --stdio` で連続 frame を順に処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_initialize_shutdown_sequence() {
    let init_body = r#"{"jsonrpc":"2.0","id":31,"method":"initialize","params":0}"#;
    let shutdown_body = r#"{"jsonrpc":"2.0","id":32,"method":"shutdown","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        init_body.len(),
        init_body,
        shutdown_body.len(),
        shutdown_body
    );
    let init_response = r#"{"jsonrpc":"2.0","id":31,"result":[1,1,1,1,1,1,1]}"#;
    let shutdown_response = r#"{"jsonrpc":"2.0","id":32,"result":0}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        init_response.len(),
        init_response,
        shutdown_response.len(),
        shutdown_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で initialize→shutdown frame を順に返すべき"
    );
}

/// TEST-CLI-02-AN: actual Cli main は `lsp --stdio` で unknown method を Method not found frame にできること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_unknown_method() {
    let request_body = r#"{"jsonrpc":"2.0","id":41,"method":"workspace/unknown","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body =
        r#"{"jsonrpc":"2.0","id":41,"error":{"code":-32601,"message":"Method not found"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で unknown method を error frame にすべき"
    );
}

/// TEST-CLI-02-AN2: actual Cli main は `lsp --stdio` で completion request を framed response にできること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion() {
    let request_body = r#"{"jsonrpc":"2.0","id":51,"method":"textDocument/completion","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body = r#"{"jsonrpc":"2.0","id":51,"result":[["defn",14,"defn"],["let",14,"let"],["if",14,"if"],["match",14,"match"],["do",14,"do"],["fn",14,"fn"],["module",14,"module"]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で completion frame を返すべき"
    );
}

/// TEST-CLI-02-AN3: actual Cli main は `lsp --stdio` で definition request を framed response にできること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_goto_definition() {
    let request_body = r#"{"jsonrpc":"2.0","id":61,"method":"textDocument/definition","params":{"uri":10,"line":1,"col":38,"source":"(defn helper [x] x) (defn main [] (helper 1))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body = r#"{"jsonrpc":"2.0","id":61,"result":[10,1,7]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で definition frame を返すべき"
    );
}

/// TEST-CLI-02-AN4: actual Cli main は `lsp --stdio` で hover request を framed response にできること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover() {
    let request_body = r#"{"jsonrpc":"2.0","id":62,"method":"textDocument/hover","params":{"uri":10,"line":1,"col":38,"source":"(defn helper [x] x) (defn main [] (helper 1))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body =
        r#"{"jsonrpc":"2.0","id":62,"result":{"range":[1,36,1,42],"contents":"defn helper"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で hover frame を返すべき"
    );
}

/// TEST-CLI-02-AN5: actual Cli main は `lsp --stdio` で references request を framed response にできること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references() {
    let request_body = r#"{"jsonrpc":"2.0","id":63,"method":"textDocument/references","params":{"uri":10,"line":1,"col":38,"source":"(defn square [x] x) (defn main [] (square 1) (square 2))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body = r#"{"jsonrpc":"2.0","id":63,"result":[[10,1,7],[10,1,36],[10,1,47]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で references frame を返すべき"
    );
}

/// TEST-CLI-02-AN6: actual Cli main は `lsp --stdio` で formatting request を framed response にできること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_formatting() {
    let request_body = r#"{"jsonrpc":"2.0","id":64,"method":"textDocument/formatting","params":{"uri":10,"source":"(defn main [] 1)"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body =
        "{\"jsonrpc\":\"2.0\",\"id\":64,\"result\":[[1,1,1,17,\"(defn main [] 1)\\n\"]]}";
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で formatting frame を返すべき"
    );
}

/// TEST-CLI-02-AN7: actual Cli main は `lsp --stdio` で rename request を framed response にできること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename() {
    let request_body = r#"{"jsonrpc":"2.0","id":65,"method":"textDocument/rename","params":{"uri":10,"line":1,"col":38,"source":"(defn square [x] x) (defn main [] (square 1) (square 2))","newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );
    let response_body = r#"{"jsonrpc":"2.0","id":65,"result":[[10,[[1,7,1,13,"cube"],[1,36,1,42,"cube"],[1,47,1,53,"cube"]]]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で rename frame を返すべき"
    );
}

/// TEST-CLI-02-AN8: actual Cli main は `lsp --stdio` で didOpen -> didChange sequence を順に処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence() {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":"(defn main [] 0)"}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"source":"(defn main [] (+ 0 1))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );
    let open_response =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":16}}"#;
    let change_response = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"sourceBytes":22}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        change_response.len(),
        change_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は lsp --stdio で didOpen -> didChange frame を順に返すべき"
    );
}

/// TEST-CLI-02-AN9: actual Cli main は `lsp --stdio` で didOpen 後に source なし hover request も open document state から処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_uses_open_document() {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":"(defn helper [x] x) (defn main [] (helper 1))"}}"#;
    let hover_body = r#"{"jsonrpc":"2.0","id":66,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        hover_body.len(),
        hover_body
    );
    let open_response =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":45}}"#;
    let hover_response =
        r#"{"jsonrpc":"2.0","id":66,"result":{"range":[1,36,1,42],"contents":"defn helper"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        hover_response.len(),
        hover_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didOpen 後の source なし hover で open document state を使うべき"
    );
}

/// TEST-CLI-02-AN9b: actual Cli main は spec 寄り hover params でも open document state から処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_uses_open_document_spec_params() {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":"(defn helper [x] x) (defn main [] (helper 1))"}}"#;
    let hover_body = r#"{"jsonrpc":"2.0","id":66,"method":"textDocument/hover","params":{"textDocument":{"uri":42},"position":{"line":1,"character":38}}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        hover_body.len(),
        hover_body
    );
    let open_response =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"sourceBytes":45}}"#;
    let hover_response =
        r#"{"jsonrpc":"2.0","id":66,"result":{"range":[1,36,1,42],"contents":"defn helper"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        hover_response.len(),
        hover_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は spec 寄り hover params でも open document state を使うべき"
    );
}

/// TEST-CLI-02-AN10: actual Cli main は `lsp --stdio` で didOpen 後に source なし definition request も open document state から処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_uses_open_document() {
    let source = "(defn helper [x] x) (defn main [] (helper 1))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":67,"method":"textDocument/definition","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        definition_body.len(),
        definition_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let definition_response = r#"{"jsonrpc":"2.0","id":67,"result":[42,1,7]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        definition_response.len(),
        definition_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didOpen 後の source なし definition で open document state を使うべき"
    );
}

/// TEST-CLI-02-AN10b: actual Cli main は spec 寄り definition params でも open document state から処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_uses_open_document_spec_params() {
    let source = "(defn helper [x] x) (defn main [] (helper 1))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":67,"method":"textDocument/definition","params":{"textDocument":{"uri":42},"position":{"line":1,"character":38}}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        definition_body.len(),
        definition_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let definition_response = r#"{"jsonrpc":"2.0","id":67,"result":[42,1,7]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        definition_response.len(),
        definition_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は spec 寄り definition params でも open document state を使うべき"
    );
}

/// TEST-CLI-02-AN11: actual Cli main は `lsp --stdio` で didOpen 後に source なし references request も open document state から処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_uses_open_document() {
    let source = "(defn square [x] x) (defn main [] (square 1) (square 2))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let references_body = r#"{"jsonrpc":"2.0","id":68,"method":"textDocument/references","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        references_body.len(),
        references_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":1001,"line":1,"col":56,"messageHash":0}]}}"#;
    let references_response =
        r#"{"jsonrpc":"2.0","id":68,"result":[[42,1,7],[42,1,36],[42,1,47]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        open_diagnostics.len(),
        open_diagnostics,
        references_response.len(),
        references_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didOpen 後の source なし references で open document state を使うべき"
    );
}

/// TEST-CLI-02-AN11b: actual Cli main は spec 寄り references params でも open document state から処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_uses_open_document_spec_params() {
    let source = "(defn square [x] x) (defn main [] (square 1) (square 2))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let references_body = r#"{"jsonrpc":"2.0","id":68,"method":"textDocument/references","params":{"textDocument":{"uri":42},"position":{"line":1,"character":38}}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        references_body.len(),
        references_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":1001,"line":1,"col":56,"messageHash":0}]}}"#;
    let references_response =
        r#"{"jsonrpc":"2.0","id":68,"result":[[42,1,7],[42,1,36],[42,1,47]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        open_diagnostics.len(),
        open_diagnostics,
        references_response.len(),
        references_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は spec 寄り references params でも open document state を使うべき"
    );
}

/// TEST-CLI-02-AN12: actual Cli main は `lsp --stdio` で didOpen 後に source なし formatting request も open document state から処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_uses_open_document() {
    let source = "(defn main [] 1)";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let formatting_body =
        r#"{"jsonrpc":"2.0","id":69,"method":"textDocument/formatting","params":{"uri":42}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        formatting_body.len(),
        formatting_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let formatting_response =
        "{\"jsonrpc\":\"2.0\",\"id\":69,\"result\":[[1,1,1,17,\"(defn main [] 1)\\n\"]]}";
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        formatting_response.len(),
        formatting_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didOpen 後の source なし formatting で open document state を使うべき"
    );
}

/// TEST-CLI-02-AN13: actual Cli main は `lsp --stdio` で didOpen 後に source なし rename request も open document state から処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_uses_open_document() {
    let source = "(defn square [x] x) (defn main [] (square 1) (square 2))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let rename_body = r#"{"jsonrpc":"2.0","id":70,"method":"textDocument/rename","params":{"uri":42,"line":1,"col":38,"newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        rename_body.len(),
        rename_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":1001,"line":1,"col":56,"messageHash":0}]}}"#;
    let rename_response = r#"{"jsonrpc":"2.0","id":70,"result":[[42,[[1,7,1,13,"cube"],[1,36,1,42,"cube"],[1,47,1,53,"cube"]]]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        open_diagnostics.len(),
        open_diagnostics,
        rename_response.len(),
        rename_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didOpen 後の source なし rename で open document state を使うべき"
    );
}

/// TEST-CLI-02-AN13b: actual Cli main は spec 寄り rename params でも open document state から処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_uses_open_document_spec_params() {
    let source = "(defn square [x] x) (defn main [] (square 1) (square 2))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let rename_body = r#"{"jsonrpc":"2.0","id":70,"method":"textDocument/rename","params":{"textDocument":{"uri":42},"position":{"line":1,"character":38},"newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        rename_body.len(),
        rename_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let open_diagnostics = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":1001,"line":1,"col":56,"messageHash":0}]}}"#;
    let rename_response = r#"{"jsonrpc":"2.0","id":70,"result":[[42,[[1,7,1,13,"cube"],[1,36,1,42,"cube"],[1,47,1,53,"cube"]]]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        open_diagnostics.len(),
        open_diagnostics,
        rename_response.len(),
        rename_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は spec 寄り rename params でも open document state を使うべき"
    );
}

/// TEST-CLI-02-AN14: actual Cli main は `lsp --stdio` で didOpen 後に source なし completion request も open document state から処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_open_document() {
    let source = "(defn helper [] 1) (he)";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":71,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":23}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        completion_body.len(),
        completion_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let completion_response = r#"{"jsonrpc":"2.0","id":71,"result":[["helper",3,"helper"]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
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
        "Cli main は didOpen 後の source なし completion で open document state を使うべき"
    );
}

/// TEST-CLI-02-AN14b: actual Cli main は spec 寄り completion params でも open document state から処理できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_open_document_spec_params() {
    let source = "(defn helper [] 1) (he)";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":71,"method":"textDocument/completion","params":{"textDocument":{"uri":42},"position":{"line":1,"character":23}}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        completion_body.len(),
        completion_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        source.len()
    );
    let completion_response = r#"{"jsonrpc":"2.0","id":71,"result":[["helper",3,"helper"]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
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
        "Cli main は spec 寄り completion params でも open document state を使うべき"
    );
}

/// TEST-CLI-02-AN14c: actual Cli main は spec 寄り didOpen `textDocument.text` の
/// escaped quote を含む source でも formatting へ正しく渡せること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_uses_spec_document_text_with_escaped_quote()
{
    let source = r#"(defn main [] "a\"b")"#;
    let open_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": 42,
                "languageId": "lsharp",
                "version": 1,
                "text": source
            }
        }
    })
    .to_string();
    let formatting_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 169,
        "method": "textDocument/formatting",
        "params": {
            "uri": 42
        }
    })
    .to_string();
    let stdin = format!("{}{}", lsp_frame(&open_body), lsp_frame(&formatting_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    let formatted = format!("{source}\n");
    let frames = parse_lsp_stdio_frames(&output);
    let expected = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "uri": 42,
                "sourceBytes": source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 169,
            "result": [[1, 1, 1, source.len() + 1, formatted]]
        }),
    ];

    assert_eq!(
        frames, expected,
        "Cli main は escaped quote を含む spec document text でも formatting へ同じ source を渡すべき"
    );
}

/// TEST-CLI-02-AN14d: actual Cli main は spec 寄り didOpen `textDocument.text` の
/// unicode escaped quote (`\u0022`) でも formatting へ正しく渡せること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_uses_spec_document_text_with_unicode_escaped_quote()
 {
    let source = r#"(defn main [] "ab")"#;
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":"(defn main [] \u0022ab\u0022)"}}}"#;
    let formatting_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 171,
        "method": "textDocument/formatting",
        "params": {
            "uri": 42
        }
    })
    .to_string();
    let stdin = format!("{}{}", lsp_frame(open_body), lsp_frame(&formatting_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    let formatted = format!("{source}\n");
    let frames = parse_lsp_stdio_frames(&output);
    let expected = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "uri": 42,
                "sourceBytes": source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 171,
            "result": [[1, 1, 1, source.len() + 1, formatted]]
        }),
    ];

    assert_eq!(
        frames, expected,
        "Cli main は unicode escaped quote を含む spec document text でも formatting へ同じ source を渡すべき"
    );
}

/// TEST-CLI-02-AN14e: actual Cli main は didOpen 後の formatting で defn metadata を保持すること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_preserves_defn_metadata() {
    let source = r#"(defn add [x y] :doc "Add two ints" :params [(x "left") (y "right")] :returns "sum" :example [(add 1 2)] (+ x y))"#;
    let open_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": 42,
                "languageId": "lsharp",
                "version": 1,
                "text": source
            }
        }
    })
    .to_string();
    let formatting_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 173,
        "method": "textDocument/formatting",
        "params": {
            "uri": 42
        }
    })
    .to_string();
    let stdin = format!("{}{}", lsp_frame(&open_body), lsp_frame(&formatting_body));

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    let formatted = "(defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y))\n";
    let frames = parse_lsp_stdio_frames(&output);
    let expected = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "uri": 42,
                "sourceBytes": source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 173,
            "result": [[1, 1, 1, source.len() + 1, formatted]]
        }),
    ];

    assert_eq!(
        frames, expected,
        "Cli main は LSP formatting でも defn metadata を canonical 順で保持するべき"
    );
}

/// TEST-CLI-02-AN15: actual Cli main は `lsp --stdio` で open 済み別 document から source なし definition を解決できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_resolves_open_document() {
    let helper_source = "(defn helper [x] x)";
    let main_source = "(helper 1)";
    let open_helper_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":11,"source":"{}"}}}}"#,
        helper_source
    );
    let open_main_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":10,"source":"{}"}}}}"#,
        main_source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":72,"method":"textDocument/definition","params":{"uri":10,"line":1,"col":2}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_body.len(),
        open_helper_body,
        open_main_body.len(),
        open_main_body,
        definition_body.len(),
        definition_body
    );
    let open_helper_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":11,"sourceBytes":{}}}}}"#,
        helper_source.len()
    );
    let open_main_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":10,"sourceBytes":{}}}}}"#,
        main_source.len()
    );
    let definition_response = r#"{"jsonrpc":"2.0","id":72,"result":[11,1,7]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_response.len(),
        open_helper_response,
        open_main_response.len(),
        open_main_response,
        definition_response.len(),
        definition_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は open 済み別 document から source なし definition を解決すべき"
    );
}

/// TEST-CLI-02-AN16: actual Cli main は `lsp --stdio` で open 済み別 document から source なし hover contents を解決できること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_resolves_open_document() {
    let helper_source = "(defn helper [x] x)";
    let main_source = "(helper 1)";
    let open_helper_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":11,"source":"{}"}}}}"#,
        helper_source
    );
    let open_main_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":10,"source":"{}"}}}}"#,
        main_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":73,"method":"textDocument/hover","params":{"uri":10,"line":1,"col":2}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_body.len(),
        open_helper_body,
        open_main_body.len(),
        open_main_body,
        hover_body.len(),
        hover_body
    );
    let open_helper_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":11,"sourceBytes":{}}}}}"#,
        helper_source.len()
    );
    let open_main_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":10,"sourceBytes":{}}}}}"#,
        main_source.len()
    );
    let hover_response =
        r#"{"jsonrpc":"2.0","id":73,"result":{"range":[1,2,1,8],"contents":"defn helper"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_response.len(),
        open_helper_response,
        open_main_response.len(),
        open_main_response,
        hover_response.len(),
        hover_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は open 済み別 document から source なし hover contents を解決すべき"
    );
}

/// TEST-CLI-02-AN17: actual Cli main は `lsp --stdio` で didChange 後の source なし completion に最新 document state を使うこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_changed_document() {
    let open_source = "(defn alpha [] 1) (al)";
    let changed_source = "(defn helper [] 1) (he)";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":74,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":23}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        completion_body.len(),
        completion_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        open_source.len()
    );
    let change_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        changed_source.len()
    );
    let completion_response = r#"{"jsonrpc":"2.0","id":74,"result":[["helper",3,"helper"]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        change_response.len(),
        change_response,
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
        "Cli main は didChange 後の source なし completion で最新 document state を使うべき"
    );
}

/// TEST-CLI-02-AN17b: actual Cli main は spec 寄り `contentChanges[0].text` の
/// escaped newline でも didChange 後の最新 source を使うこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_spec_changed_document_with_escaped_newline()
 {
    let open_source = "(defn alpha [] 1) (al)";
    let changed_source = "(defn helper [] 1)\n(he)";
    let open_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": 42,
                "languageId": "lsharp",
                "version": 1,
                "text": open_source
            }
        }
    })
    .to_string();
    let change_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": 42,
                "version": 2
            },
            "contentChanges": [
                {
                    "text": changed_source
                }
            ]
        }
    })
    .to_string();
    let completion_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 170,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": 42
            },
            "position": {
                "line": 2,
                "character": 4
            }
        }
    })
    .to_string();
    let stdin = format!(
        "{}{}{}",
        lsp_frame(&open_body),
        lsp_frame(&change_body),
        lsp_frame(&completion_body)
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    let frames = parse_lsp_stdio_frames(&output);
    let expected = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "uri": 42,
                "sourceBytes": open_source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "uri": 42,
                "sourceBytes": changed_source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 170,
            "result": [["helper", 3, "helper"]]
        }),
    ];

    assert_eq!(
        frames, expected,
        "Cli main は escaped newline を含む spec didChange text でも最新 completion source を使うべき"
    );
}

/// TEST-CLI-02-AN17c: actual Cli main は spec 寄り `contentChanges[0].text` の
/// unicode escaped newline (`\u000a`) でも didChange 後の最新 source を使うこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_uses_spec_changed_document_with_unicode_escaped_newline()
 {
    let open_source = "(defn alpha [] 1) (al)";
    let changed_source = "(defn helper [] 1)\n(he)";
    let open_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": 42,
                "languageId": "lsharp",
                "version": 1,
                "text": open_source
            }
        }
    })
    .to_string();
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn helper [] 1)\u000a(he)"}]}}"#;
    let completion_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 172,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": 42
            },
            "position": {
                "line": 2,
                "character": 4
            }
        }
    })
    .to_string();
    let stdin = format!(
        "{}{}{}",
        lsp_frame(&open_body),
        lsp_frame(change_body),
        lsp_frame(&completion_body)
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    let frames = parse_lsp_stdio_frames(&output);
    let expected = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "uri": 42,
                "sourceBytes": open_source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "uri": 42,
                "sourceBytes": changed_source.len()
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 172,
            "result": [["helper", 3, "helper"]]
        }),
    ];

    assert_eq!(
        frames, expected,
        "Cli main は unicode escaped newline を含む spec didChange text でも最新 completion source を使うべき"
    );
}

/// TEST-CLI-02-AN18: actual Cli main は `lsp --stdio` で same-URI repeated didOpen 後に最新 source を使うこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_repeated_didopen_keeps_latest_source() {
    let first_source = "(defn alpha [] 1) (al)";
    let latest_source = "(defn beta [] 1) (be)";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":75,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":21}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        completion_body.len(),
        completion_body
    );
    let first_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        first_source.len()
    );
    let second_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        latest_source.len()
    );
    let completion_response = r#"{"jsonrpc":"2.0","id":75,"result":[["beta",3,"beta"]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_response.len(),
        first_open_response,
        second_open_response.len(),
        second_open_response,
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
        "Cli main は same-URI repeated didOpen 後に最新 source を保持するべき"
    );
}

/// TEST-CLI-02-AN19: actual Cli main は `lsp --stdio` で didChange 後の source なし hover に最新 document state を使うこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_uses_changed_document() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":76,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        hover_body.len(),
        hover_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        open_source.len()
    );
    let change_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        changed_source.len()
    );
    let hover_response =
        r#"{"jsonrpc":"2.0","id":76,"result":{"range":[1,36,1,42],"contents":"defn helper"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        change_response.len(),
        change_response,
        hover_response.len(),
        hover_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didChange 後の source なし hover で最新 document state を使うべき"
    );
}

/// TEST-CLI-02-AN20: actual Cli main は `lsp --stdio` で same-URI repeated didOpen 後の source なし definition に最新 source を使うこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_uses_latest_reopened_document() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":77,"method":"textDocument/definition","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        definition_body.len(),
        definition_body
    );
    let first_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        first_source.len()
    );
    let second_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        latest_source.len()
    );
    let definition_response = r#"{"jsonrpc":"2.0","id":77,"result":[42,1,7]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_response.len(),
        first_open_response,
        second_open_response.len(),
        second_open_response,
        definition_response.len(),
        definition_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は same-URI repeated didOpen 後の source なし definition で最新 source を使うべき"
    );
}

/// TEST-CLI-02-AN21: actual Cli main は `lsp --stdio` で didChange 後の source なし definition に最新 document state を使うこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_uses_changed_document() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":78,"method":"textDocument/definition","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        definition_body.len(),
        definition_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        open_source.len()
    );
    let change_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        changed_source.len()
    );
    let definition_response = r#"{"jsonrpc":"2.0","id":78,"result":[42,1,7]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        change_response.len(),
        change_response,
        definition_response.len(),
        definition_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didChange 後の source なし definition で最新 document state を使うべき"
    );
}

/// TEST-CLI-02-AN21b: actual Cli main は `lsp --stdio` で didChange 後の source なし references に最新 document state を使うこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_uses_changed_document() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let references_body = r#"{"jsonrpc":"2.0","id":82,"method":"textDocument/references","params":{"uri":42,"line":1,"col":40}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        references_body.len(),
        references_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        open_source.len()
    );
    let change_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        changed_source.len()
    );
    let references_response =
        r#"{"jsonrpc":"2.0","id":82,"result":[[42,1,7],[42,1,40],[42,1,51]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        change_response.len(),
        change_response,
        references_response.len(),
        references_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didChange 後の source なし references で最新 document state を使うべき"
    );
}

/// TEST-CLI-02-AN21c: actual Cli main は `lsp --stdio` で didChange 後の source なし rename に最新 document state を使うこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_uses_changed_document() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let rename_body = r#"{"jsonrpc":"2.0","id":84,"method":"textDocument/rename","params":{"uri":42,"line":1,"col":40,"newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        rename_body.len(),
        rename_body
    );
    let open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        open_source.len()
    );
    let change_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        changed_source.len()
    );
    let rename_response = r#"{"jsonrpc":"2.0","id":84,"result":[[42,[[1,7,1,13,"cube"],[1,40,1,46,"cube"],[1,51,1,57,"cube"]]]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_response.len(),
        open_response,
        change_response.len(),
        change_response,
        rename_response.len(),
        rename_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は didChange 後の source なし rename で最新 document state を使うべき"
    );
}

/// TEST-CLI-02-AN22: actual Cli main は `lsp --stdio` で same-URI repeated didOpen 後の source なし hover に最新 source を使うこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_uses_latest_reopened_document() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":79,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        hover_body.len(),
        hover_body
    );
    let first_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        first_source.len()
    );
    let second_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        latest_source.len()
    );
    let hover_response =
        r#"{"jsonrpc":"2.0","id":79,"result":{"range":[1,36,1,42],"contents":"defn helper"}}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_response.len(),
        first_open_response,
        second_open_response.len(),
        second_open_response,
        hover_response.len(),
        hover_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は same-URI repeated didOpen 後の source なし hover で最新 source を使うべき"
    );
}

/// TEST-CLI-02-AN22b: actual Cli main は `lsp --stdio` で same-URI repeated didOpen 後の source なし references に最新 source を使うこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_uses_latest_reopened_document() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let references_body = r#"{"jsonrpc":"2.0","id":83,"method":"textDocument/references","params":{"uri":42,"line":1,"col":40}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        references_body.len(),
        references_body
    );
    let first_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        first_source.len()
    );
    let second_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        latest_source.len()
    );
    let references_response =
        r#"{"jsonrpc":"2.0","id":83,"result":[[42,1,7],[42,1,40],[42,1,51]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_response.len(),
        first_open_response,
        second_open_response.len(),
        second_open_response,
        references_response.len(),
        references_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は same-URI repeated didOpen 後の source なし references で最新 source を使うべき"
    );
}

/// TEST-CLI-02-AN22c: actual Cli main は `lsp --stdio` で same-URI repeated didOpen 後の source なし rename に最新 source を使うこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_uses_latest_reopened_document() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let rename_body = r#"{"jsonrpc":"2.0","id":85,"method":"textDocument/rename","params":{"uri":42,"line":1,"col":40,"newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        rename_body.len(),
        rename_body
    );
    let first_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        first_source.len()
    );
    let second_open_response = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"sourceBytes":{}}}}}"#,
        latest_source.len()
    );
    let rename_response = r#"{"jsonrpc":"2.0","id":85,"result":[[42,[[1,7,1,13,"cube"],[1,40,1,46,"cube"],[1,51,1,57,"cube"]]]]}"#;
    let expected = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_response.len(),
        first_open_response,
        second_open_response.len(),
        second_open_response,
        rename_response.len(),
        rename_response
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_eq!(
        output, expected,
        "Cli main は same-URI repeated didOpen 後の source なし rename で最新 source を使うべき"
    );
}

/// TEST-CLI-02-AN23: actual Cli main は `lsp --stdio` completion response を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","id":51,"method":"textDocument/completion","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "completion.json",
        "Cli main は lsp --stdio completion response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN24: actual Cli main は `lsp --stdio` の open 済み別 document definition response を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_open_document_schema_snapshot() {
    let helper_source = "(defn helper [x] x)";
    let main_source = "(helper 1)";
    let open_helper_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":11,"source":"{}"}}}}"#,
        helper_source
    );
    let open_main_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":10,"source":"{}"}}}}"#,
        main_source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":72,"method":"textDocument/definition","params":{"uri":10,"line":1,"col":2}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_helper_body.len(),
        open_helper_body,
        open_main_body.len(),
        open_main_body,
        definition_body.len(),
        definition_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "definition-open-document.json",
        "Cli main は lsp --stdio definition open-document response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN25: actual Cli main は `lsp --stdio` formatting response を valid JSON schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_formatting_open_document_schema_snapshot() {
    let source = "(defn main [] 1)";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        source
    );
    let formatting_body =
        r#"{"jsonrpc":"2.0","id":69,"method":"textDocument/formatting","params":{"uri":42}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        formatting_body.len(),
        formatting_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "formatting-open-document.json",
        "Cli main は lsp --stdio formatting response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN26: actual Cli main は `lsp --stdio` hover response を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","id":62,"method":"textDocument/hover","params":{"uri":10,"line":1,"col":38,"source":"(defn helper [x] x) (defn main [] (helper 1))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "hover.json",
        "Cli main は lsp --stdio hover response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN27: actual Cli main は `lsp --stdio` references response を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","id":63,"method":"textDocument/references","params":{"uri":10,"line":1,"col":38,"source":"(defn square [x] x) (defn main [] (square 1) (square 2))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "references.json",
        "Cli main は lsp --stdio references response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN28: actual Cli main は `lsp --stdio` rename response を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","id":65,"method":"textDocument/rename","params":{"uri":10,"line":1,"col":38,"source":"(defn square [x] x) (defn main [] (square 1) (square 2))","newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "rename.json",
        "Cli main は lsp --stdio rename response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN29: actual Cli main は `lsp --stdio` initialize response を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_initialize_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","id":21,"method":"initialize","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "initialize.json",
        "Cli main は lsp --stdio initialize response schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN30: actual Cli main は `lsp --stdio` initialize→shutdown sequence を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_initialize_shutdown_schema_snapshot() {
    let init_body = r#"{"jsonrpc":"2.0","id":31,"method":"initialize","params":0}"#;
    let shutdown_body = r#"{"jsonrpc":"2.0","id":32,"method":"shutdown","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        init_body.len(),
        init_body,
        shutdown_body.len(),
        shutdown_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "initialize-shutdown-sequence.json",
        "Cli main は lsp --stdio initialize→shutdown schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN31: actual Cli main は `lsp --stdio` unknown method error を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_unknown_method_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","id":41,"method":"workspace/unknown","params":0}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "unknown-method.json",
        "Cli main は lsp --stdio unknown method error schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN31b: actual Cli main は `lsp --stdio` shutdown 後 request error を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_request_after_shutdown_schema_snapshot() {
    let shutdown_body = r#"{"jsonrpc":"2.0","id":51,"method":"shutdown","params":0}"#;
    let hover_body = r#"{"jsonrpc":"2.0","id":52,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":1}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        shutdown_body.len(),
        shutdown_body,
        hover_body.len(),
        hover_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "shutdown-request-after-error.json",
        "Cli main は lsp --stdio shutdown 後 request error schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN32: actual Cli main は `lsp --stdio` didOpen→didChange sequence を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_schema_snapshot() {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":"(defn main [] 0)"}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"source":"(defn main [] (+ 0 1))"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence.json",
        "Cli main は lsp --stdio didOpen→didChange schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN33: actual Cli main は `lsp --stdio` publishDiagnostics notification を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_publish_diagnostics_schema_snapshot() {
    let request_body = r#"{"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":42,"diagnostics":[{"source":1,"severity":1,"rule":203,"line":2,"col":4,"messageHash":7003}]}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}",
        request_body.len(),
        request_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "publish-diagnostics.json",
        "Cli main は lsp --stdio publishDiagnostics notification schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN34: actual Cli main は `lsp --stdio` の didChange 後 hover fallback を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_changed_document_schema_snapshot() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":76,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        hover_body.len(),
        hover_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "hover-changed-document.json",
        "Cli main は lsp --stdio didChange 後 hover fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN34c: actual Cli main は `lsp --stdio` の didChange 後 completion fallback を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_changed_document_schema_snapshot() {
    let open_source = "(defn alpha [] 1) (al)";
    let changed_source = "(defn helper [] 1) (he)";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":74,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":23}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        completion_body.len(),
        completion_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "completion-changed-document.json",
        "Cli main は lsp --stdio didChange 後 completion fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN34b: actual Cli main は `lsp --stdio` の didChange 後 references fallback を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_changed_document_schema_snapshot() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let references_body = r#"{"jsonrpc":"2.0","id":82,"method":"textDocument/references","params":{"uri":42,"line":1,"col":40}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        references_body.len(),
        references_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "references-changed-document.json",
        "Cli main は lsp --stdio didChange 後 references fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN34d: actual Cli main は `lsp --stdio` の didChange 後 definition fallback を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_changed_document_schema_snapshot() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":78,"method":"textDocument/definition","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        definition_body.len(),
        definition_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "definition-changed-document.json",
        "Cli main は lsp --stdio didChange 後 definition fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN34e: actual Cli main は `lsp --stdio` の didChange 後 rename fallback を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_changed_document_schema_snapshot() {
    let open_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let changed_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        open_source
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":42,"source":"{}"}}}}"#,
        changed_source
    );
    let rename_body = r#"{"jsonrpc":"2.0","id":84,"method":"textDocument/rename","params":{"uri":42,"line":1,"col":40,"newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body,
        rename_body.len(),
        rename_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "rename-changed-document.json",
        "Cli main は lsp --stdio didChange 後 rename fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN35: actual Cli main は `lsp --stdio` の repeated didOpen 後 definition fallback を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_latest_reopened_schema_snapshot() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let definition_body = r#"{"jsonrpc":"2.0","id":77,"method":"textDocument/definition","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        definition_body.len(),
        definition_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "definition-latest-reopened.json",
        "Cli main は lsp --stdio repeated didOpen 後 definition fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN35c: actual Cli main は `lsp --stdio` の repeated didOpen 後 hover fallback を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_latest_reopened_schema_snapshot() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (helper 1))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let hover_body = r#"{"jsonrpc":"2.0","id":79,"method":"textDocument/hover","params":{"uri":42,"line":1,"col":38}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        hover_body.len(),
        hover_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "hover-latest-reopened.json",
        "Cli main は lsp --stdio repeated didOpen 後 hover fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN35b: actual Cli main は `lsp --stdio` の repeated didOpen 後 references fallback を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_latest_reopened_schema_snapshot() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let references_body = r#"{"jsonrpc":"2.0","id":83,"method":"textDocument/references","params":{"uri":42,"line":1,"col":40}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        references_body.len(),
        references_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "references-latest-reopened.json",
        "Cli main は lsp --stdio repeated didOpen 後 references fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN35d: actual Cli main は `lsp --stdio` の repeated didOpen 後 completion fallback を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_latest_reopened_schema_snapshot() {
    let first_source = "(defn alpha [] 1) (al)";
    let latest_source = "(defn beta [] 1) (be)";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let completion_body = r#"{"jsonrpc":"2.0","id":75,"method":"textDocument/completion","params":{"uri":42,"line":1,"col":21}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        completion_body.len(),
        completion_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "completion-latest-reopened.json",
        "Cli main は lsp --stdio repeated didOpen 後 completion fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN35e: actual Cli main は `lsp --stdio` の repeated didOpen 後 rename fallback を schema snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_latest_reopened_schema_snapshot() {
    let first_source = "(defn alpha [x] x) (defn main [] (alpha 1))";
    let latest_source = "(defn helper [x] x) (defn main [] (do (helper 1) (helper 2)))";
    let first_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        first_source
    );
    let second_open_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"uri":42,"source":"{}"}}}}"#,
        latest_source
    );
    let rename_body = r#"{"jsonrpc":"2.0","id":85,"method":"textDocument/rename","params":{"uri":42,"line":1,"col":40,"newName":"cube"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        first_open_body.len(),
        first_open_body,
        second_open_body.len(),
        second_open_body,
        rename_body.len(),
        rename_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "rename-latest-reopened.json",
        "Cli main は lsp --stdio repeated didOpen 後 rename fallback schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN36: actual Cli main は `lsp --stdio` で didChange 時に diagnostics refresh frame を返すこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_diagnostics_refresh_snapshot() {
    let open_body =
        r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":")"}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"source":"(defn main [] 0)"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence-diagnostics-refresh.json",
        "Cli main は lsp --stdio didChange diagnostics refresh schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN37: actual Cli main は spec 寄り didOpen/didChange params でも diagnostics refresh snapshot に一致すること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_spec_params_diagnostics_refresh_snapshot()
 {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":")"}}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn main [] 0)"}]}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence-diagnostics-refresh.json",
        "Cli main は spec 寄り lsp --stdio didChange diagnostics refresh でも snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN37a: actual Cli main は `lsp --stdio` で type diagnostics refresh frame を返すこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_type_diagnostics_refresh_snapshot() {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":"(defn main [] (if 42 1 0))"}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"source":"(defn main [] 0)"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence-type-diagnostics-refresh.json",
        "Cli main は lsp --stdio didChange type diagnostics refresh schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN37b: actual Cli main は `lsp --stdio` で lint diagnostics refresh frame を返すこと
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_lint_diagnostics_refresh_snapshot() {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"uri":42,"source":"(defn main [] (let [x 42] 0))"}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"uri":42,"source":"(defn main [] 0)"}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence-lint-diagnostics-refresh.json",
        "Cli main は lsp --stdio didChange lint diagnostics refresh schema を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN37c: actual Cli main は spec 寄り didOpen/didChange params でも type diagnostics refresh snapshot に一致すること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_spec_params_type_diagnostics_refresh_snapshot()
 {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":"(defn main [] (if 42 1 0))"}}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn main [] 0)"}]}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence-type-diagnostics-refresh.json",
        "Cli main は spec 寄り lsp --stdio type diagnostics refresh でも snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN37d: actual Cli main は spec 寄り didOpen/didChange params でも lint diagnostics refresh snapshot に一致すること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_document_sequence_spec_params_lint_diagnostics_refresh_snapshot()
 {
    let open_body = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":42,"languageId":"lsharp","version":1,"text":"(defn main [] (let [x 42] 0))"}}}"#;
    let change_body = r#"{"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":42,"version":2},"contentChanges":[{"text":"(defn main [] 0)"}]}}"#;
    let stdin = format!(
        "Content-Length: {}\r\n\r\n{}Content-Length: {}\r\n\r\n{}",
        open_body.len(),
        open_body,
        change_body.len(),
        change_body
    );

    let output = compile_and_run_with_args_and_stdin(
        selfhost_cli_runtime_bundle(),
        &["lsp", "--stdio"],
        &stdin,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "document-sequence-lint-diagnostics-refresh.json",
        "Cli main は spec 寄り lsp --stdio lint diagnostics refresh でも snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN38: actual Cli main は document path 付き hover の filesystem import response を snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_hover_filesystem_import_schema_snapshot() {
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let hover_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let hover_body = format!(
        r#"{{"jsonrpc":"2.0","id":191,"method":"textDocument/hover","params":{{"uri":200,"line":1,"col":{hover_col}}}}}"#
    );

    let output = run_lsp_filesystem_snapshot_request(
        "hover_filesystem_snapshot",
        200,
        "src/Main.ls",
        main_source,
        &hover_body,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "hover-filesystem-import.json",
        "Cli main は document path 付き hover の filesystem import response を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN39: actual Cli main は document path 付き completion の filesystem import response を snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_completion_filesystem_import_schema_snapshot() {
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-va))";
    let completion_col = main_source.find("mid-va").expect("mid-va call") + "mid-va".len() + 1;
    let completion_body = format!(
        r#"{{"jsonrpc":"2.0","id":192,"method":"textDocument/completion","params":{{"uri":200,"line":1,"col":{completion_col}}}}}"#
    );

    let output = run_lsp_filesystem_snapshot_request(
        "completion_filesystem_snapshot",
        200,
        "src/Main.ls",
        main_source,
        &completion_body,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "completion-filesystem-import.json",
        "Cli main は document path 付き completion の filesystem import response を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN40: actual Cli main は document path 付き definition の filesystem import response を snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_definition_filesystem_import_schema_snapshot() {
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let definition_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let definition_body = format!(
        r#"{{"jsonrpc":"2.0","id":193,"method":"textDocument/definition","params":{{"uri":200,"line":1,"col":{definition_col}}}}}"#
    );

    let output = run_lsp_filesystem_snapshot_request(
        "definition_filesystem_snapshot",
        200,
        "src/Main.ls",
        main_source,
        &definition_body,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "definition-filesystem-import.json",
        "Cli main は document path 付き definition の filesystem import response を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN41: actual Cli main は document path 付き references の filesystem import response を snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_references_filesystem_import_schema_snapshot() {
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let references_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let references_body = format!(
        r#"{{"jsonrpc":"2.0","id":194,"method":"textDocument/references","params":{{"uri":200,"line":1,"col":{references_col}}}}}"#
    );

    let output = run_lsp_filesystem_snapshot_request(
        "references_filesystem_snapshot",
        200,
        "src/Main.ls",
        main_source,
        &references_body,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "references-filesystem-import.json",
        "Cli main は document path 付き references の filesystem import response を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN42: actual Cli main は document path 付き rename の filesystem import response を snapshot に一致させること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_rename_filesystem_import_schema_snapshot() {
    let main_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let rename_col = main_source.find("(mid-val)").expect("mid-val call") + 2;
    let rename_body = format!(
        r#"{{"jsonrpc":"2.0","id":195,"method":"textDocument/rename","params":{{"uri":200,"line":1,"col":{rename_col},"newName":"mid-next"}}}}"#
    );

    let output = run_lsp_filesystem_snapshot_request(
        "rename_filesystem_snapshot",
        200,
        "src/Main.ls",
        main_source,
        &rename_body,
    );

    assert_lsp_stdio_snapshot(
        &output,
        "rename-filesystem-import.json",
        "Cli main は document path 付き rename の filesystem import response を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN43: actual Cli main は filesystem-backed path state を
/// 複数 request と didChange を跨いで保持し、代表 sequence snapshot に一致すること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_filesystem_document_sequence_schema_snapshot() {
    let dir = cli_test_fixture_dir("filesystem_document_sequence_snapshot");
    write_cli_fixture_files(&dir, &cli_lsp_nested_fixture_files());

    let open_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let changed_source = "(module Main) (import Support.Mid) (defn main [] (mid-va))";
    let symbol_col = open_source.find("(mid-val)").expect("mid-val call") + 2;
    let completion_col = changed_source.find("mid-va").expect("mid-va call") + "mid-va".len() + 1;

    let open_body = make_lsp_did_open_with_path(200, "src/Main.ls", open_source);
    let hover_body = format!(
        r#"{{"jsonrpc":"2.0","id":196,"method":"textDocument/hover","params":{{"uri":200,"line":1,"col":{symbol_col}}}}}"#
    );
    let definition_body = format!(
        r#"{{"jsonrpc":"2.0","id":197,"method":"textDocument/definition","params":{{"uri":200,"line":1,"col":{symbol_col}}}}}"#
    );
    let references_body = format!(
        r#"{{"jsonrpc":"2.0","id":198,"method":"textDocument/references","params":{{"uri":200,"line":1,"col":{symbol_col}}}}}"#
    );
    let rename_body = format!(
        r#"{{"jsonrpc":"2.0","id":199,"method":"textDocument/rename","params":{{"uri":200,"line":1,"col":{symbol_col},"newName":"mid-next"}}}}"#
    );
    let change_body = format!(
        r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"uri":200,"source":"{}"}}}}"#,
        changed_source
    );
    let completion_body = format!(
        r#"{{"jsonrpc":"2.0","id":200,"method":"textDocument/completion","params":{{"uri":200,"line":1,"col":{completion_col}}}}}"#
    );
    let stdin = format!(
        "{}{}{}{}{}{}{}",
        lsp_frame(&open_body),
        lsp_frame(&hover_body),
        lsp_frame(&definition_body),
        lsp_frame(&references_body),
        lsp_frame(&rename_body),
        lsp_frame(&change_body),
        lsp_frame(&completion_body)
    );

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_lsp_stdio_snapshot(
        &output,
        "filesystem-document-sequence.json",
        "Cli main は filesystem-backed long-lived document sequence を snapshot と一致させるべき",
    );
}

/// TEST-CLI-02-AN43b: actual Cli main は filesystem-backed path state を
/// spec 寄り request shape + didChange を跨いでも保持し、同じ representative snapshot に収束すること
#[test]
fn test_e2e_selfhost_cli_main_with_lsp_stdio_filesystem_document_sequence_spec_style_snapshot() {
    let dir = cli_test_fixture_dir("filesystem_document_sequence_spec_style_snapshot");
    write_cli_fixture_files(&dir, &cli_lsp_nested_fixture_files());

    let open_source = "(module Main) (import Support.Mid) (defn main [] (mid-val))";
    let changed_source = "(module Main) (import Support.Mid) (defn main [] (mid-va))";
    let symbol_col = open_source.find("(mid-val)").expect("mid-val call") + 2;
    let completion_col = changed_source.find("mid-va").expect("mid-va call") + "mid-va".len();

    let open_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": 200,
                "languageId": "lsharp",
                "version": 1,
                "text": open_source
            },
            "path": "src/Main.ls"
        }
    })
    .to_string();
    let hover_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 196,
        "method": "textDocument/hover",
        "params": {
            "textDocument": {
                "uri": 200
            },
            "position": {
                "line": 1,
                "character": symbol_col
            }
        }
    })
    .to_string();
    let definition_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 197,
        "method": "textDocument/definition",
        "params": {
            "textDocument": {
                "uri": 200
            },
            "position": {
                "line": 1,
                "character": symbol_col
            }
        }
    })
    .to_string();
    let references_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 198,
        "method": "textDocument/references",
        "params": {
            "textDocument": {
                "uri": 200
            },
            "position": {
                "line": 1,
                "character": symbol_col
            }
        }
    })
    .to_string();
    let rename_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 199,
        "method": "textDocument/rename",
        "params": {
            "textDocument": {
                "uri": 200
            },
            "position": {
                "line": 1,
                "character": symbol_col
            },
            "newName": "mid-next"
        }
    })
    .to_string();
    let change_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didChange",
        "params": {
            "textDocument": {
                "uri": 200,
                "version": 2
            },
            "contentChanges": [
                {
                    "text": changed_source
                }
            ]
        }
    })
    .to_string();
    let completion_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 200,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": 200
            },
            "position": {
                "line": 1,
                "character": completion_col
            }
        }
    })
    .to_string();
    let stdin = format!(
        "{}{}{}{}{}{}{}",
        lsp_frame(&open_body),
        lsp_frame(&hover_body),
        lsp_frame(&definition_body),
        lsp_frame(&references_body),
        lsp_frame(&rename_body),
        lsp_frame(&change_body),
        lsp_frame(&completion_body)
    );

    let output = run_lsp_stdio_with_dir(&stdin, &dir);
    let _ = std::fs::remove_dir_all(&dir);

    assert_lsp_stdio_snapshot(
        &output,
        "filesystem-document-sequence.json",
        "Cli main は filesystem-backed long-lived document sequence を spec params でも同じ snapshot に収束させるべき",
    );
}

/// TEST-CLI-02-AO: actual Cli main は help lsp に `--stdio` surface を含めること
#[test]
fn test_e2e_selfhost_cli_main_with_help_lsp_stdio_option() {
    let output = compile_and_run_with_args(selfhost_cli_runtime_bundle(), &["help", "lsp"]);

    assert!(
        output.contains("lsp [--stdio] - Start LSP server"),
        "Cli main は lsp help に --stdio surface を含めるべき: {:?}",
        output
    );
}

/// TEST-LSP-01: selfhost/src/Tools/Lsp/LspServer.ls 存在 + JSON-RPC dispatch 構造
///
/// T4-2: L# 製 LSP の正式化 -- LspServer.ls が存在し JSON-RPC dispatch を持つこと
/// Red Phase: selfhost/src/Tools/Lsp/LspServer.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_lsp_skeleton_v2() {
    let lsp_path = selfhost_source_path("LspServer.ls");
    assert!(
        lsp_path.exists(),
        "selfhost/src/Tools/Lsp/LspServer.ls が存在しない (T4-2: L# 製 LSP の正式化)"
    );
    let source = std::fs::read_to_string(&lsp_path)
        .expect("selfhost/src/Tools/Lsp/LspServer.ls の読み込みに失敗");

    // JSON-RPC dispatch 構造を確認
    assert!(
        source.contains("jsonrpc")
            || source.contains("json-rpc")
            || source.contains("JsonRpc")
            || source.contains("dispatch"),
        "selfhost/src/Tools/Lsp/LspServer.ls に JSON-RPC dispatch 構造がない"
    );
    // module 宣言
    assert!(
        source.contains("(module Tools.Lsp.LspServer)") || source.contains("(module Tools.Lsp"),
        "selfhost/src/Tools/Lsp/LspServer.ls に module 宣言がない"
    );
}

/// TEST-LSP-02: selfhost/src/Tools/Lsp/LspServer.ls に LSP 3.17 の 10 メソッドが定義されていること
///
/// T4-2 AC-005: initialize/shutdown/didOpen/didChange/hover/goto_definition/
///              references/rename/formatting/completion の 10 メソッド
/// Red Phase: selfhost/src/Tools/Lsp/LspServer.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_lsp_10_methods() {
    let lsp_path = selfhost_source_path("LspServer.ls");
    assert!(
        lsp_path.exists(),
        "selfhost/src/Tools/Lsp/LspServer.ls が存在しない"
    );
    let source = std::fs::read_to_string(&lsp_path)
        .expect("selfhost/src/Tools/Lsp/LspServer.ls の読み込みに失敗");

    // T4-2 AC-005: 10 メソッドが LSP 3.17 仕様に準拠
    let methods = [
        "initialize",
        "shutdown",
        "didOpen",
        "didChange",
        "hover",
        "goto_definition",
        "references",
        "rename",
        "formatting",
        "completion",
    ];
    // メソッド名のバリエーション (キャメルケース / スネークケース / ハイフン区切り)
    for method in &methods {
        let snake = method.to_string();
        let kebab = snake.replace('_', "-");
        let found = source.contains(&snake) || source.contains(&kebab);
        assert!(
            found,
            "selfhost/src/Tools/Lsp/LspServer.ls に LSP メソッド '{}' の定義がない (AC-005)",
            method
        );
    }
}
