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
      (print-string (vector-get fn0 1))
      (print-string "\n")
      (print (vector-get fn0 2))
      (print (vector-length types))
      (print-string (vector-get type0 1))
      (print-string "\n")
      (print (if (string-eq (vector-get type0 2) "type") 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["5", "1", "1", "1", "1", "add", "2", "1", "Num", "1"]);
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
      (print (if (string-eq (vector-get fn1 1) (vector-get fn2 1)) 1 0))
      (print (if (= (vector-get fn1 2) (vector-get fn2 2)) 1 0))
      (print (if (string-eq (vector-get type1 1) (vector-get type2 1)) 1 0))
      (print (if (string-eq (vector-get type1 2) (vector-get type2 2)) 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1", "1", "1", "1", "1", "1", "1"]);
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
      (print (if (string-eq (vector-get doc 1) "functions:1,types:1,first-fn:main,first-type:Doc") 1 0))
      (print (vector-length functions))
      (print (vector-length types))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["4", "1", "1", "1", "1"]);
}

/// DOC-01: module decl がある場合は title に module 名を反映すること
#[test]
fn test_e2e_selfhost_doctools_module_title_uses_name() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn main [] 42))")
        doc (generate program 0)]
    (do
      (print (if (string-eq (vector-get doc 0) "module-Demo") 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1"]);
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
      (print-string (vector-get fn0 1))
      (print-string "\n")
      (print (vector-get fn0 2))
      (print (vector-length types))
      (print (if (string-eq (vector-get type0 1) "Doc") 1 0))
      (print (if (string-eq (vector-get type0 2) "type") 1 0))
      (print (if (string-eq (vector-get type1 1) "Alias") 1 0))
      (print (if (string-eq (vector-get type1 2) "typealias") 1 0))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["3", "100", "1", "add", "2", "2", "1", "1", "1", "1"]);
}

/// DOC-01: DocTools entry list が source 順ではなく deterministic な hash 昇順で並ぶこと
#[test]
fn test_e2e_selfhost_doctools_sorts_entries_by_hash() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn zebra [] 0) (defn add [] 1) (type Zebra Int) (type Alias Int)")
        kb (generate-knowledge program 100)
        fns (vector-get kb 1)
        tys (vector-get kb 2)
        fn0 (vector-get fns 0)
        fn1 (vector-get fns 1)
        ty0 (vector-get tys 0)
        ty1 (vector-get tys 1)
        zebra-hash (name-hash "zebra" 0 5)
        add-hash (name-hash "add" 0 3)
        zebra-type-hash (name-hash "Zebra" 0 5)
        alias-hash (name-hash "Alias" 0 5)]
    (do
      (print (vector-length fns))
      (print (if (< add-hash zebra-hash)
               (= (vector-get fn0 0) add-hash)
               (= (vector-get fn0 0) zebra-hash)))
      (print (if (< add-hash zebra-hash)
               (= (vector-get fn1 0) zebra-hash)
               (= (vector-get fn1 0) add-hash)))
      (print (vector-length tys))
      (print (if (< alias-hash zebra-type-hash)
               (= (vector-get ty0 0) alias-hash)
               (= (vector-get ty0 0) zebra-type-hash)))
      (print (if (< alias-hash zebra-type-hash)
               (= (vector-get ty1 0) zebra-type-hash)
               (= (vector-get ty1 0) alias-hash)))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["2", "1", "1", "2", "1", "1"]);
}

/// DOC-01: generate-review が unused-let を deterministic diagnostic として返すこと
#[test]
fn test_e2e_selfhost_doctools_schema_review() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (let [x 42] 0))")
        rev (generate-review program 200)
        diags (vector-get rev 1)
        diag0 (vector-get diags 0)]
    (do
      (print (vector-length rev))
      (print (vector-get rev 0))
      (print (vector-length diags))
      (print (vector-length diag0))
      (print (vector-get diag0 0))
      (print-string (vector-get diag0 1))
      (print-string "\n")
      (print-string (vector-get diag0 2))
      (print-string "\n")
      (print-string (vector-get diag0 3))
      (print-string "\n")
      (print (vector-get diag0 4))
      (print (vector-get diag0 5))
      (print-string (vector-get diag0 6))
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "2",
            "200",
            "1",
            "7",
            "100",
            "unused-let",
            "let binding x is not used",
            "warning",
            "1",
            "1",
            "L0001",
        ]
    );
}

/// DOC-01: generate-review が empty-do を deterministic diagnostic として返すこと
#[test]
fn test_e2e_selfhost_doctools_schema_review_empty_do() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (do))")
        rev (generate-review program 300)
        diags (vector-get rev 1)
        diag0 (vector-get diags 0)]
    (do
      (print (vector-get rev 0))
      (print (vector-length diags))
      (print (vector-length diag0))
      (print (vector-get diag0 0))
      (print-string (vector-get diag0 1))
      (print-string "\n")
      (print-string (vector-get diag0 2))
      (print-string "\n")
      (print-string (vector-get diag0 3))
      (print-string "\n")
      (print (vector-get diag0 4))
      (print (vector-get diag0 5))
      (print-string (vector-get diag0 6))
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "300",
            "1",
            "7",
            "104",
            "empty-do",
            "do block has no expressions",
            "warning",
            "1",
            "1",
            "L0002",
        ]
    );
}

/// DOC-01: generate-doc-ack が status/title/body と trailer を返すこと
#[test]
fn test_e2e_selfhost_doctools_doc_ack_trailer_payload() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn main [] 42))")
        ack (generate-doc-ack program "alice")
        trailers (vector-get ack 3)]
    (do
      (print (vector-length ack))
      (print-string (vector-get ack 0))
      (print-string "\n")
      (print-string (vector-get ack 1))
      (print-string "\n")
      (print-string (vector-get ack 2))
      (print-string "\n")
      (print (vector-length trailers))
      (print-string (vector-get trailers 0))
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "4",
            "ack:recorded",
            "module-Demo",
            "functions:1,types:0,first-fn:main",
            "1",
            "Doc-Reviewed-By: alice",
        ]
    );
}

/// DOC-01: generate-doc-check が status/title/body と trailer を返すこと
#[test]
fn test_e2e_selfhost_doctools_doc_check_trailer_payload() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn main [] 42))")
        check (generate-doc-check program "alice")
        trailers (vector-get check 3)]
    (do
      (print (vector-length check))
      (print-string (vector-get check 0))
      (print-string "\n")
      (print-string (vector-get check 1))
      (print-string "\n")
      (print-string (vector-get check 2))
      (print-string "\n")
      (print (vector-length trailers))
      (print-string (vector-get trailers 0))
      (print-string "\n")
      (print-string (vector-get trailers 1))
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec![
            "4",
            "status:ok",
            "module-Demo",
            "functions:1,types:0,first-fn:main",
            "2",
            "Doc-Review-Status: Passed",
            "Doc-Reviewed-By: alice",
        ]
    );
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

/// DOC-01: generate-doc-output も module decl がある場合は module 名 title を使うこと
#[test]
fn test_e2e_selfhost_doctools_schema_doc_output_module_title_name() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn main [] 42))")
        doc-out (generate-doc-output program 300)]
    (do
      (print (if (string-eq (vector-get doc-out 3) "module-Demo") 1 0))
      (print (vector-get doc-out 4))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1", "1"]);
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
      (print (if (string-eq (vector-get kb-fn1 1) (vector-get kb-fn2 1)) 1 0))
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

/// D-4: Cli.ls の parse-diagnostics-count が recovery 対象入力で 1 を返すこと
#[test]
fn test_e2e_selfhost_cli_parse_diagnostics_recovery_error() {
    let harness = r#"
(defn main []
  (let [diag-count (parse-diagnostics-count ")")]
    (do
      (print diag-count)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1"], "unexpected ')' の parse diagnostics は 1 件であるべき");
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

/// D-4: Cli.ls の check-diagnostics-count が型エラー入力で 1 を返すこと
#[test]
fn test_e2e_selfhost_cli_check_diagnostics_type_error() {
    let harness = r#"
(defn main []
  (let [diag-count (check-diagnostics-count "(defn main [] (if 42 1 0))")]
    (do
      (print diag-count)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1"], "if 条件の型エラーは check diagnostics 1 件であるべき");
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

// === HtmlDoc 単体テスト ===

/// HtmlDoc.render-function-signature が "<li>fn-{id}/{arity}</li>" 形式を返す
#[test]
fn test_e2e_selfhost_htmldoc_render_function_signature() {
    let harness = r#"
(defn main []
  (let [func-doc (vector-push (vector-push (vector-push (vector-new 3) 42) "add") 3)
        result (render-function-signature func-doc)]
    (do
      (print (if (string-eq result "<li>add/3</li>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// HtmlDoc.render-type-definition が "<li>{kind} {name}</li>" 形式を返す
#[test]
fn test_e2e_selfhost_htmldoc_render_type_definition() {
    let harness = r#"
(defn main []
  (let [type-doc (vector-push (vector-push (vector-push (vector-new 3) 99) "Pair") "recorddef")
        result (render-type-definition type-doc)]
    (do
      (print (if (string-eq result "<li>recorddef Pair</li>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// HtmlDoc.render-module-page が <main><h1>...</h1>... 構造を持つ
#[test]
fn test_e2e_selfhost_htmldoc_render_module_page_structure() {
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
  (let [program (parse-program "(defn add [x y] (+ x y)) (defn sub [a b] (- a b)) (type Pair Int)")
        doc (generate-html program 0)
        page (render-module-page doc)]
    (do
      ;; <main><h1> で始まる
      (print (if (string-eq (substring page 0 10) "<main><h1>") 1 0))
      ;; </main> で終わる
      (let [len (string-length page)]
        (print (if (string-eq (substring page (- len 7) len) "</main>") 1 0)))
      ;; 関数セクションが存在する
      (print (if (= (string-contains page "<section id=\"functions\">") 1) 1 0))
      ;; 型セクションが存在する
      (print (if (= (string-contains page "<section id=\"types\">") 1) 1 0))
      ;; 関数エントリが名前つきで <li> に含まれる
      (print (if (= (string-contains page "<li>add/2</li>") 1) 1 0))
      ;; 型エントリが kind + name で <li> に含まれる
      (print (if (= (string-contains page "<li>type Pair</li>") 1) 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1", "1", "1", "1"]);
}

/// HtmlDoc.render-html が完全な HTML ドキュメントを返し、title がエスケープされる
#[test]
fn test_e2e_selfhost_htmldoc_render_html_full_document() {
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
      ;; <!doctype html> で始まる
      (print (if (string-eq (substring html 0 15) "<!doctype html>") 1 0))
      ;; <html> を含む
      (print (if (= (string-contains html "<html>") 1) 1 0))
      ;; <head> を含む
      (print (if (= (string-contains html "<head>") 1) 1 0))
      ;; <body> を含む
      (print (if (= (string-contains html "<body>") 1) 1 0))
      ;; </body></html> で終わる
      (let [len (string-length html)]
        (print (if (string-eq (substring html (- len 14) len) "</body></html>") 1 0)))
      ;; <title> を含む
      (print (if (= (string-contains html "<title>") 1) 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1", "1", "1", "1"]);
}

/// HtmlDoc.render-index がモジュール一覧ページを生成する
#[test]
fn test_e2e_selfhost_htmldoc_render_index() {
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
  (let [modules (vector-push (vector-push (vector-push (vector-new 3) "Parser") "Lexer") "DocTools")
        html (render-index modules)]
    (do
      ;; <!doctype html> で始まる
      (print (if (string-eq (substring html 0 15) "<!doctype html>") 1 0))
      ;; <h1>modules</h1> を含む
      (print (if (= (string-contains html "<h1>modules</h1>") 1) 1 0))
      ;; 各モジュール名が <li> に含まれる
      (print (if (= (string-contains html "<li>Parser</li>") 1) 1 0))
      (print (if (= (string-contains html "<li>Lexer</li>") 1) 1 0))
      (print (if (= (string-contains html "<li>DocTools</li>") 1) 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1", "1", "1"]);
}

/// HtmlDoc: 関数も型もない場合の render-module-page
#[test]
fn test_e2e_selfhost_htmldoc_render_module_page_empty() {
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
        page (render-module-page doc)]
    (do
      ;; <main><h1> で始まる
      (print (if (string-eq (substring page 0 10) "<main><h1>") 1 0))
      ;; 関数セクションは存在する (main が抽出される)
      (print (if (= (string-contains page "<section id=\"functions\">") 1) 1 0))
      ;; 型セクションは存在しない
      (print (if (= (string-contains page "<section id=\"types\">") 0) 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_html_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1"]);
}
