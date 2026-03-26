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

/// DOC-01: generate-knowledge の出力が knowledge スキーマ構造に準拠すること
/// スキーマ: docs/schemas/knowledge.schema.json
/// 構造: [module-id, functions-count, types-count]
#[test]
fn test_e2e_selfhost_doctools_schema_knowledge() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42) (type Doc Int)")
        kb (generate-knowledge program 100)]
    (do
      (print (vector-length kb))
      (print (vector-get kb 0))
      (print (vector-get kb 1))
      (print (vector-get kb 2))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    // knowledge 構造は 3 スロット: [module-id, functions-count, types-count]
    assert_eq!(lines[lines.len() - 4], "3", "knowledge 構造は 3 スロットであるべき");
    // module-id = 100
    assert_eq!(lines[lines.len() - 3], "100", "module-id が正しいこと");
    // functions-count = 1 (defn main)
    assert_eq!(lines[lines.len() - 2], "1", "functions-count = 1");
    // types-count = 1 (type Doc Int)
    assert_eq!(lines[lines.len() - 1], "1", "types-count = 1");
}

/// DOC-01: generate-review の出力が review スキーマ構造に準拠すること
/// スキーマ: docs/schemas/review.schema.json
/// 構造: [source-id, diagnostics-count]
#[test]
fn test_e2e_selfhost_doctools_schema_review() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42)")
        rev (generate-review program 200)]
    (do
      (print (vector-length rev))
      (print (vector-get rev 0))
      (print (vector-get rev 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    // review 構造は 2 スロット: [source-id, diagnostics-count]
    assert_eq!(lines[lines.len() - 3], "2", "review 構造は 2 スロットであるべき");
    // source-id = 200
    assert_eq!(lines[lines.len() - 2], "200", "source-id が正しいこと");
    // 正常ソースでは diagnostics-count = 0
    assert_eq!(lines[lines.len() - 1], "0", "正常ソースの diagnostics は 0 件");
}

/// DOC-01: generate-doc-output の出力が doc-output スキーマ構造に準拠すること
/// スキーマ: docs/schemas/doc-output.schema.json
/// 構造: [module-id, public-functions, types-count, html-title, html-sections]
#[test]
fn test_e2e_selfhost_doctools_schema_doc_output() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42) (type Doc Int)")
        doc-out (generate-doc-output program 300)]
    (do
      (print (vector-length doc-out))
      (print (vector-get doc-out 0))
      (print (vector-get doc-out 1))
      (print (vector-get doc-out 2))
      (print (vector-get doc-out 3))
      (print (vector-get doc-out 4))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    // doc-output 構造は 5 スロット
    assert_eq!(lines[lines.len() - 6], "5", "doc-output 構造は 5 スロットであるべき");
    // module-id = 300
    assert_eq!(lines[lines.len() - 5], "300", "module-id が正しいこと");
    // public_functions = 1 (defn main)
    assert_eq!(lines[lines.len() - 4], "1", "public_functions = 1");
    // types = 1 (type Doc Int)
    assert_eq!(lines[lines.len() - 3], "1", "types = 1");
    // html-title = 0 (placeholder)
    assert_eq!(lines[lines.len() - 2], "0", "html-title は placeholder (0)");
    // html-sections = 2 (functions セクション + types セクション)
    assert_eq!(lines[lines.len() - 1], "2", "html-sections = 2 (functions + types)");
}

/// DOC-01: ドキュメント出力にタイムスタンプ・ホスト名・絶対パスが含まれないこと
/// AC-409: 環境依存情報を一切含まない
#[test]
fn test_e2e_selfhost_doctools_no_timestamp() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42) (type Doc Int)")
        doc (generate program 0)
        html (generate-html program 0)
        kb (generate-knowledge program 0)
        rev (generate-review program 0)
        doc-out (generate-doc-output program 0)]
    (do
      ;; 全出力スロットを print して環境依存値がないことを検証
      ;; generate: [title=0, body=0, fn-count, type-count]
      (print (vector-get doc 0))
      (print (vector-get doc 1))
      ;; generate-html: [tag=1, title=0, body=0, fn-count, type-count]
      (print (vector-get html 0))
      (print (vector-get html 1))
      (print (vector-get html 2))
      ;; generate-knowledge: [module-id=0, fn-count, type-count]
      (print (vector-get kb 0))
      ;; generate-review: [source-id=0, diag-count=0]
      (print (vector-get rev 1))
      ;; generate-doc-output: [module-id=0, fn-count, type-count, html-title=0, sections]
      (print (vector-get doc-out 0))
      (print (vector-get doc-out 3))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    // 全ての固定スロットが決定的小整数であること (タイムスタンプ等の大きな数値がないこと)
    let fixed_slots = &lines[lines.len() - 9..lines.len()];
    for (i, slot) in fixed_slots.iter().enumerate() {
        let val: i64 = slot.parse().unwrap_or_else(|_| panic!("スロット {} が整数でない: {}", i, slot));
        assert!(
            val.abs() < 1000,
            "スロット {} の値 {} がタイムスタンプまたは環境依存値の可能性あり",
            i, val
        );
    }
}

/// DOC-01: 同一入力に対し全スキーマ出力が deterministic であること
/// AC-408: 同一入力→同一出力 (2回実行して完全一致)
#[test]
fn test_e2e_selfhost_doctools_deterministic() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (type Num Int)")
        kb1 (generate-knowledge program 50)
        kb2 (generate-knowledge program 50)
        doc1 (generate-doc-output program 50)
        doc2 (generate-doc-output program 50)
        rev1 (generate-review program 50)
        rev2 (generate-review program 50)]
    (do
      ;; knowledge: 全 3 スロットが一致
      (print (= (vector-get kb1 0) (vector-get kb2 0)))
      (print (= (vector-get kb1 1) (vector-get kb2 1)))
      (print (= (vector-get kb1 2) (vector-get kb2 2)))
      ;; doc-output: 全 5 スロットが一致
      (print (= (vector-get doc1 0) (vector-get doc2 0)))
      (print (= (vector-get doc1 1) (vector-get doc2 1)))
      (print (= (vector-get doc1 2) (vector-get doc2 2)))
      (print (= (vector-get doc1 3) (vector-get doc2 3)))
      (print (= (vector-get doc1 4) (vector-get doc2 4)))
      ;; review: 全 2 スロットが一致
      (print (= (vector-get rev1 0) (vector-get rev2 0)))
      (print (= (vector-get rev1 1) (vector-get rev2 1)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    // 全 10 スロット比較が true (= 1) であること
    let comparisons = &lines[lines.len() - 10..lines.len()];
    for (i, cmp) in comparisons.iter().enumerate() {
        assert_eq!(
            *cmp, "1",
            "スロット比較 {} が不一致: expected 1, got {}",
            i, cmp
        );
    }
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
