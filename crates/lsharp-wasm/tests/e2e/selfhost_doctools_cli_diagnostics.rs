use super::support::*;


/// D-3: DocTools.ls に generate-html 関数が存在し、deterministic な結果を返すこと
#[test]
fn test_e2e_selfhost_doctools_generate_html_basic() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42)")
        html (generate-html program 0)]
    (do
      (print (vector-length html))
      (print (vector-get html 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    // HTML doc 構造は 5 スロット: [tag, title, body, functions-count, types-count]
    assert_eq!(lines[lines.len() - 2], "5", "HTML doc 構造は 5 スロットであるべき");
    // tag=1 は HTML ドキュメント
    assert_eq!(lines[lines.len() - 1], "1", "HTML doc の tag は 1 であるべき");
}

/// D-3: DocTools.ls の generate-html が idempotent であること (2回実行で同一結果)
#[test]
fn test_e2e_selfhost_doctools_generate_html_idempotent() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y))")
        html1 (generate-html program 0)
        html2 (generate-html program 0)]
    (do
      (print (vector-length html1))
      (print (vector-length html2))
      (print (= (vector-get html1 3) (vector-get html2 3)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    // 同一サイズ
    assert_eq!(lines[lines.len() - 3], lines[lines.len() - 2], "2回の generate-html で同一サイズ");
    // functions-count が一致
    assert_eq!(lines[lines.len() - 1], "1", "同一入力で同一 functions-count");
}

/// D-4: Cli.ls の parse-diagnostics-count が正常ソースで 0 を返すこと
#[test]
fn test_e2e_selfhost_cli_parse_diagnostics() {
    let harness = r#"
(defn main []
  (let [diag-count (parse-diagnostics-count "(defn main [] 42)")]
    (do
      (print diag-count)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.last().unwrap(), &"0", "正常ソースの parse diagnostics は 0 件であるべき");
}

/// D-4: Cli.ls の check-diagnostics-count が正常ソースで 0 を返すこと
#[test]
fn test_e2e_selfhost_cli_check_diagnostics() {
    let harness = r#"
(defn main []
  (let [diag-count (check-diagnostics-count "(defn main [] 42)")]
    (do
      (print diag-count)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.last().unwrap(), &"0", "正常ソースの check diagnostics は 0 件であるべき");
}
