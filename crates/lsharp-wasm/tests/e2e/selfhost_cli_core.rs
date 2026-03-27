use super::support::*;

/// TEST-CLI-02-C: selfhost/Cli.ls に repl/lsp/fmt/doc コマンド定義
///
/// T4-4 AC-013: ユーティリティコマンドが L# 実装で動作すること
/// Red Phase: selfhost/Cli.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_cli_repl_lsp_fmt() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cli_path = project_root.join("selfhost/Cli.ls");
    assert!(cli_path.exists(), "selfhost/Cli.ls が存在しない");
    let source = std::fs::read_to_string(&cli_path).expect("selfhost/Cli.ls の読み込みに失敗");

    // ユーティリティコマンドの定義を確認 (T4-4 AC-013)
    let commands = ["repl", "lsp", "fmt", "doc"];
    for cmd in &commands {
        assert!(
            source.contains(cmd),
            "selfhost/Cli.ls に '{}' コマンドの定義がない (AC-013)",
            cmd
        );
    }
}

/// TEST-CLI-01-B: selfhost/Cli.ls の --help 相当出力が主要コマンドを列挙できること
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

/// TEST-CLI-01-C: selfhost/Cli.ls の --version 相当出力が `lsharp x.y.z` 形式であること
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

/// TEST-CLI-02-D: selfhost/Cli.ls の parse core helper が source を parse できること
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

/// TEST-CLI-02-E: selfhost/Cli.ls の check core helper が source を型推論できること
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

/// TEST-CLI-02-F: selfhost/Cli.ls の run-parse が file-path から source を読めること
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

/// TEST-CLI-02-G: selfhost/Cli.ls の run-check が file-path から source を読めること
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

/// TEST-CLI-02-H: selfhost/Cli.ls の file-path handler は missing file を compile error で返す
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

/// TEST-CLI-02-I: selfhost/Cli.ls の arg-parse がコマンド文字列を command id へ変換できること
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

/// TEST-CLI-02-J: selfhost/Cli.ls の run-fmt-source が format-program を呼べること
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

/// TEST-CLI-02-K: selfhost/Cli.ls の run-fmt が file-path から source を読めること
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

/// TEST-CLI-02-L: selfhost/Cli.ls の run-compile-source が compile PoC を呼べること
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

/// TEST-CLI-02-M: selfhost/Cli.ls の run-compile が file-path から source を読めること
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

/// TEST-CLI-02-M2: selfhost/Cli.ls の run-build が file-path から source を読めること
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

/// TEST-CLI-02-M3: selfhost/Cli.ls の run-install が install plan text を返せること
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

/// TEST-CLI-02-M4: selfhost/Cli.ls の run-install は空 package を compile error にする
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

/// TEST-CLI-02-M5: selfhost/Cli.ls の run-repl が warmup session summary を返せること
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

/// TEST-CLI-02-M6: selfhost/Cli.ls の run-lsp が capability summary text を返せること
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

/// TEST-CLI-02-M7: selfhost/Cli.ls の LSP transport helper が initialize request を frame response にできること
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

/// TEST-CLI-02-M8: selfhost/Cli.ls の LSP transport helper が未知メソッドを JSON-RPC error frame にできること
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

/// TEST-CLI-02-M9: selfhost/Cli.ls の LSP transport helper sequence が shared-state で複数 request を捌けること
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

/// TEST-CLI-02-M9b: selfhost/Cli.ls の LSP transport helper が hover request を framed response にできること
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

/// TEST-CLI-02-M9c: selfhost/Cli.ls の LSP transport helper が references request を framed response にできること
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

/// TEST-CLI-02-M9d: selfhost/Cli.ls の LSP transport helper が completion request を framed response にできること
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

/// TEST-CLI-02-M9e: selfhost/Cli.ls の LSP transport helper が formatting request を framed response にできること
#[test]
fn test_e2e_selfhost_cli_lsp_transport_formatting_frame() {
    let body = "{\"jsonrpc\":\"2.0\",\"id\":12,\"result\":[[1,1,2,4,\"(defn main [] 1)\n\"]]}";
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

/// TEST-CLI-02-M9f: selfhost/Cli.ls の LSP transport helper が rename request を framed response にできること
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

/// TEST-CLI-02-M9: selfhost/Cli.ls の LSP transport helper sequence が shared-state で複数 request を捌けること
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

/// TEST-CLI-02-N: selfhost/Cli.ls の run-test-source が TestRunner.generate-tests を呼べること
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

/// TEST-CLI-02-O: selfhost/Cli.ls の run-test が file-path から source を読めること
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

/// TEST-CLI-02-O2: selfhost/TestRunner.ls が supported subset の metadata suite を実行できること
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

/// TEST-CLI-02-O2b: selfhost/TestRunner.ls が supported subset の metadata suite を実行できること
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

/// TEST-CLI-02-O2c: selfhost/TestRunner.ls が supported invariant suite を materialize できること
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
/// TEST-CLI-02-O2d: selfhost/TestRunner.ls が supported subset の metadata suite を実行できること
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

/// TEST-CLI-02-O3: selfhost/Cli.ls の run-test-source が supported subset の metadata を成功終了できること
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

/// TEST-CLI-02-O4: selfhost/Cli.ls の run-test-source が failing example を runtime error にできること
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

/// TEST-CLI-02-O5: selfhost/Cli.ls の run-test が file-path 経由の metadata suite も実行できること
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

/// TEST-CLI-02-P: selfhost/Cli.ls の run-review-source が review title/body を返せること
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

/// TEST-CLI-02-Q: selfhost/Cli.ls の run-review が file-path から review title/body を返せること
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

/// TEST-CLI-02-Q2: selfhost/Cli.ls の run-review-source が empty-do rule も返せること
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

/// TEST-CLI-02-R: selfhost/Cli.ls の run-doc-source が DocTools.generate を呼べること
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

/// TEST-CLI-02-S: selfhost/Cli.ls の run-doc が file-path から source を読めること
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

/// TEST-CLI-02-T: selfhost/Cli.ls の run-doc-ack が file-path から source を読めること
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
            "Doc-Reviewed-By: anonymous",
            "0",
        ],
        "run-doc-ack は ack status と title/body と trailer と success=0 を返すべき"
    );
}

/// TEST-CLI-02-U: selfhost/Cli.ls の run-doc-check が file-path から source を読めること
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
            "Doc-Review-Status: Passed",
            "Doc-Reviewed-By: anonymous",
            "0",
        ],
        "run-doc-check は status と title/body と trailer と success=0 を返すべき"
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

/// TEST-LSP-01: selfhost/LspServer.ls 存在 + JSON-RPC dispatch 構造
///
/// T4-2: L# 製 LSP の正式化 -- LspServer.ls が存在し JSON-RPC dispatch を持つこと
/// Red Phase: selfhost/LspServer.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_lsp_skeleton_v2() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lsp_path = project_root.join("selfhost/LspServer.ls");
    assert!(
        lsp_path.exists(),
        "selfhost/LspServer.ls が存在しない (T4-2: L# 製 LSP の正式化)"
    );
    let source =
        std::fs::read_to_string(&lsp_path).expect("selfhost/LspServer.ls の読み込みに失敗");

    // JSON-RPC dispatch 構造を確認
    assert!(
        source.contains("jsonrpc")
            || source.contains("json-rpc")
            || source.contains("JsonRpc")
            || source.contains("dispatch"),
        "selfhost/LspServer.ls に JSON-RPC dispatch 構造がない"
    );
    // module 宣言
    assert!(
        source.contains("(module LspServer)") || source.contains("(module Lsp"),
        "selfhost/LspServer.ls に module 宣言がない"
    );
}

/// TEST-LSP-02: selfhost/LspServer.ls に LSP 3.17 の 10 メソッドが定義されていること
///
/// T4-2 AC-005: initialize/shutdown/didOpen/didChange/hover/goto_definition/
///              references/rename/formatting/completion の 10 メソッド
/// Red Phase: selfhost/LspServer.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_lsp_10_methods() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lsp_path = project_root.join("selfhost/LspServer.ls");
    assert!(lsp_path.exists(), "selfhost/LspServer.ls が存在しない");
    let source =
        std::fs::read_to_string(&lsp_path).expect("selfhost/LspServer.ls の読み込みに失敗");

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
            "selfhost/LspServer.ls に LSP メソッド '{}' の定義がない (AC-005)",
            method
        );
    }
}
