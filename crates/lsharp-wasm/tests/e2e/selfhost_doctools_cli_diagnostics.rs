use super::support::*;

fn selfhost_doctools_runtime_bundle() -> String {
    [
        include_str!("../../../../selfhost/Token.ls"),
        include_str!("../../../../selfhost/AST.ls"),
        include_str!("../../../../selfhost/Lexer.ls"),
        include_str!("../../../../selfhost/Parser.ls"),
        include_str!("../../../../selfhost/DocTools.ls"),
    ]
    .join("\n")
}

fn selfhost_cli_html_runtime_bundle() -> String {
    [
        &selfhost_doctools_runtime_bundle(),
        include_str!("../../../../selfhost/HtmlTemplate.ls"),
        include_str!("../../../../selfhost/HtmlLayout.ls"),
        include_str!("../../../../selfhost/HtmlDoc.ls"),
    ]
    .join("\n")
}

/// D-3: DocTools.ls の generate-html が title/body と entry list を返すこと
#[test]
fn test_e2e_selfhost_doctools_generate_html_basic() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (type Num Int)")
        html (generate-html program 0)
        functions (vector-get html 3)
        types (vector-get html 4)
        fn0 (vector-get functions 0)
        type0 (vector-get types 0)]
    (do
      (print (vector-length html))
      (print (vector-get html 0))
      (print (if (string-eq (vector-get html 1) "module-global") 1 0))
      (print (if (> (string-length (vector-get html 2)) 0) 1 0))
      (print (vector-length functions))
      (print (vector-get fn0 1))
      (print (vector-length types))
      (print (if (string-eq (vector-get type0 1) "type") 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["5", "1", "1", "1", "1", "2", "1", "1"]);
}

/// D-3: DocTools.ls の generate-html が 2 回実行しても同一 payload を返すこと
#[test]
fn test_e2e_selfhost_doctools_generate_html_idempotent() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (type Num Int)")
        html1 (generate-html program 0)
        html2 (generate-html program 0)
        functions1 (vector-get html1 3)
        functions2 (vector-get html2 3)
        types1 (vector-get html1 4)
        types2 (vector-get html2 4)
        fn1 (vector-get functions1 0)
        fn2 (vector-get functions2 0)
        type1 (vector-get types1 0)
        type2 (vector-get types2 0)]
    (do
      (print (if (string-eq (vector-get html1 1) (vector-get html2 1)) 1 0))
      (print (if (string-eq (vector-get html1 2) (vector-get html2 2)) 1 0))
      (print (if (= (vector-length functions1) (vector-length functions2)) 1 0))
      (print (if (= (vector-get fn1 0) (vector-get fn2 0)) 1 0))
      (print (if (= (vector-get fn1 1) (vector-get fn2 1)) 1 0))
      (print (if (string-eq (vector-get type1 1) (vector-get type2 1)) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1", "1", "1"]);
}

/// DOC-01: generate が title/body/function/type payload を返すこと
#[test]
fn test_e2e_selfhost_doctools_generate_structured_doc_payload() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42) (type Doc Int)")
        doc (generate program 0)
        functions (vector-get doc 2)
        types (vector-get doc 3)]
    (do
      (print (vector-length doc))
      (print (if (string-eq (vector-get doc 0) "module-global") 1 0))
      (print (if (> (string-length (vector-get doc 1)) 0) 1 0))
      (print (vector-length functions))
      (print (vector-length types))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["4", "1", "1", "1", "1"]);
}

/// DOC-01: generate-knowledge の出力が module + function entries + type entries を返すこと
#[test]
fn test_e2e_selfhost_doctools_schema_knowledge() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (type Doc Int) (type-alias Alias Int)")
        kb (generate-knowledge program 100)
        functions (vector-get kb 1)
        types (vector-get kb 2)
        fn0 (vector-get functions 0)
        type0 (vector-get types 0)
        type1 (vector-get types 1)]
    (do
      (print (vector-length kb))
      (print (vector-get kb 0))
      (print (vector-length functions))
      (print (vector-get fn0 1))
      (print (vector-length types))
      (print (if (string-eq (vector-get type0 1) "type") 1 0))
      (print (if (string-eq (vector-get type1 1) "typealias") 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["3", "100", "1", "2", "2", "1", "1"]);
}

/// DOC-01: generate-review の diagnostics slot が vector であること
#[test]
fn test_e2e_selfhost_doctools_schema_review() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42)")
        rev (generate-review program 200)]
    (do
      (print (vector-length rev))
      (print (vector-get rev 0))
      (print (vector-length (vector-get rev 1)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["2", "200", "0"]);
}

/// DOC-01: generate-doc-output の出力が function/type entries と title を含むこと
#[test]
fn test_e2e_selfhost_doctools_schema_doc_output() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42) (type Doc Int)")
        doc-out (generate-doc-output program 300)]
    (do
      (print (vector-length doc-out))
      (print (vector-get doc-out 0))
      (print (vector-length (vector-get doc-out 1)))
      (print (vector-length (vector-get doc-out 2)))
      (print (if (string-eq (vector-get doc-out 3) "module-300") 1 0))
      (print (vector-get doc-out 4))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["5", "300", "1", "1", "1", "2"]);
}

/// DOC-01: ドキュメント文字列がタイムスタンプ・ホスト名・絶対パスを含まないこと
#[test]
fn test_e2e_selfhost_doctools_no_timestamp() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [program (parse-program "(defn main [] 42) (type Doc Int)")
        doc (generate program 0)
        html (generate-html program 0)
        doc-out (generate-doc-output program 0)]
    (do
      (print (if (string-eq (vector-get doc 0) "module-global") 1 0))
      (print (if (string-eq (vector-get doc-out 3) "module-0") 1 0))
      (print (if (= (string-contains (vector-get doc 1) "/Users/") 0) 1 0))
      (print (if (= (string-contains (vector-get html 2) "localhost") 0) 1 0))
      (print (if (= (string-contains (vector-get html 2) "2026-") 0) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1", "1"]);
}

/// DOC-01: 同一入力に対し doc/html/schema 出力が deterministic であること
#[test]
fn test_e2e_selfhost_doctools_deterministic() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (type Num Int)")
        html1 (generate-html program 0)
        html2 (generate-html program 0)
        kb1 (generate-knowledge program 50)
        kb2 (generate-knowledge program 50)
        doc1 (generate-doc-output program 50)
        doc2 (generate-doc-output program 50)
        rev1 (generate-review program 50)
        rev2 (generate-review program 50)
        kb-fn1 (vector-get (vector-get kb1 1) 0)
        kb-fn2 (vector-get (vector-get kb2 1) 0)
        kb-type1 (vector-get (vector-get kb1 2) 0)
        kb-type2 (vector-get (vector-get kb2 2) 0)]
    (do
      (print (if (string-eq (vector-get html1 1) (vector-get html2 1)) 1 0))
      (print (if (string-eq (vector-get html1 2) (vector-get html2 2)) 1 0))
      (print (if (= (vector-get kb1 0) (vector-get kb2 0)) 1 0))
      (print (if (= (vector-get kb-fn1 0) (vector-get kb-fn2 0)) 1 0))
      (print (if (= (vector-get kb-fn1 1) (vector-get kb-fn2 1)) 1 0))
      (print (if (string-eq (vector-get kb-type1 1) (vector-get kb-type2 1)) 1 0))
      (print (if (string-eq (vector-get doc1 3) (vector-get doc2 3)) 1 0))
      (print (if (= (vector-get doc1 4) (vector-get doc2 4)) 1 0))
      (print (if (= (vector-get rev1 0) (vector-get rev2 0)) 1 0))
      (print (if (= (vector-length (vector-get rev1 1)) (vector-length (vector-get rev2 1))) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1", "1", "1", "1", "1", "1", "1"]);
}

/// D-3: HtmlDoc.ls が supported subset の実 HTML を決定的に描画できること
#[test]
fn test_e2e_selfhost_doctools_html_doc_render_html_supported_subset() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (type Num Int)")
        doc (generate-html program 0)
        html1 (render-html doc 0)
        html2 (render-html doc 0)]
    (do
      (print (if (string-eq html1 html2) 1 0))
      (print (if (= (string-contains html1 "<!doctype html>") 1) 1 0))
      (print (if (= (string-contains html1 "<title>module-global</title>") 1) 1 0))
      (print (if (= (string-contains html1 "<section id=\"functions\">") 1) 1 0))
      (print (if (= (string-contains html1 "<section id=\"types\">") 1) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1", "1"]);
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

// === DOC-02 統合テスト ===

/// DOC-02: DocTools.generate-html → HtmlDoc.render-html パイプラインが実 HTML を返す
#[test]
fn test_e2e_selfhost_doctools_html_template_pipeline() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (type Num Int)")
        doc (generate-html program 0)
        html (render-html doc 0)]
    (do
      ;; 出力が非空
      (print (if (> (string-length html) 0) 1 0))
      ;; <!doctype html> で始まる
      (print (if (string-eq (substring html 0 15) "<!doctype html>") 1 0))
      ;; <section id="functions"> を含む
      (print (if (= (string-contains html "<section id=\"functions\">") 1) 1 0))
      ;; <section id="types"> を含む
      (print (if (= (string-contains html "<section id=\"types\">") 1) 1 0))
      ;; </body></html> で終わる (base-layout)
      (print (if (= (string-contains html "</body></html>") 1) 1 0))
      ;; CSS を含む (base-layout の css-inline)
      (print (if (= (string-contains html "<style>") 1) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1", "1", "1"]);
}

/// DOC-02: render-html の出力が deterministic
#[test]
fn test_e2e_selfhost_doctools_html_template_deterministic() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn foo [x] x) (type Bar Int)")
        doc (generate-html program 0)
        html1 (render-html doc 0)
        html2 (render-html doc 0)]
    (do
      (print (if (string-eq html1 html2) 1 0))
      (print (if (> (string-length html1) 100) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1"]);
}

/// DOC-02: render-html の出力にタイムスタンプ・ホスト名・絶対パスが含まれない
#[test]
fn test_e2e_selfhost_doctools_html_template_no_timestamp() {
    let harness = r#"
(defn string-contains-loop [haystack needle i hlen nlen]
  (if (> (+ i nlen) hlen)
    0
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      1
      (string-contains-loop haystack needle (+ i 1) hlen nlen))))

(defn string-contains [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (= nlen 0)
      1
      (if (> nlen hlen)
        0
        (string-contains-loop haystack needle 0 hlen nlen)))))

(defn main []
  (let [program (parse-program "(defn main [] 42)")
        doc (generate-html program 0)
        html (render-html doc 0)]
    (do
      ;; タイムスタンプが含まれない
      (print (if (= (string-contains html "2026") 0) 1 0))
      ;; ホスト名パターンが含まれない
      (print (if (= (string-contains html "hostname") 0) 1 0))
      ;; 絶対パスが含まれない
      (print (if (= (string-contains html "/Users/") 0) 1 0))
      (print (if (= (string-contains html "/home/") 0) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1"]);
}
