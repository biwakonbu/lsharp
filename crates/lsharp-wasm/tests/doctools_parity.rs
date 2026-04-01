#[path = "e2e/support.rs"]
mod support;

use support::*;

fn selfhost_doctools_bundle() -> String {
    [
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("DocTools.ls"),
    ]
    .join("\n")
}

fn selfhost_html_bundle() -> String {
    [
        &selfhost_doctools_bundle(),
        selfhost_module("HtmlTemplate.ls"),
        selfhost_module("HtmlLayout.ls"),
        selfhost_module("HtmlDoc.ls"),
    ]
    .join("\n")
}



fn run_doctools(harness: &str) -> Vec<String> {
    let output = compile_and_run(&format!("{}\n{}", selfhost_doctools_bundle(), harness));
    output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect()
}

fn run_html(harness: &str) -> Vec<String> {
    let output = compile_and_run(&format!("{}\n{}", selfhost_html_bundle(), harness));
    output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect()
}

// ============================================================
// CP-04: extract-public-functions の名前検証
// ============================================================

/// 抽出した関数エントリが実際の関数名を保持していること
#[test]
fn test_e2e_doctools_extract_function_entry_preserves_name() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (defn mul [a b] (* a b))")
        entries (extract-function-entries program)
        e0 (vector-get entries 0)
        e1 (vector-get entries 1)]
    (do
      (print (vector-length entries))
      (print-string (vector-get e0 1))
      (print-string "\n")
      (print-string (vector-get e1 1))
      (print-string "\n")
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "2", "2 件の関数エントリを抽出すべき");
    let all = format!("{},{}", lines[1], lines[2]);
    assert!(all.contains("add"), "add が含まれるべき: {}", all);
    assert!(all.contains("mul"), "mul が含まれるべき: {}", all);
}

/// 抽出した関数エントリの arity が正しいこと
#[test]
fn test_e2e_doctools_extract_function_entry_preserves_arity() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn foo [] 0) (defn bar [a b] 1)")
        entries (extract-function-entries program)
        e0 (vector-get entries 0)
        e1 (vector-get entries 1)]
    (do
      (print (vector-length entries))
      (print-string (vector-get e0 1))
      (print-string ":")
      (print (vector-get e0 2))
      (print-string (vector-get e1 1))
      (print-string ":")
      (print (vector-get e1 2))
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "2");
    // name:arity のペアを確認 (print-string はnewlineなし、printはnewlineあり)
    let all = lines[1..].join(",");
    assert!(
        all.contains("foo") && all.contains("bar"),
        "foo と bar が含まれるべき: {}",
        all
    );
}

// ============================================================
// CP-04: extract-type-definitions の名前・種類検証
// ============================================================

/// 抽出した型エントリが名前と kind を保持していること
#[test]
fn test_e2e_doctools_extract_type_entry_preserves_name_and_kind() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(type Foo Int) (type-alias Bar Int)")
        entries (extract-type-entries program)
        e0 (vector-get entries 0)
        e1 (vector-get entries 1)]
    (do
      (print (vector-length entries))
      (print-string (vector-get e0 1))
      (print-string (vector-get e0 2))
      (print-string (vector-get e1 1))
      (print-string (vector-get e1 2))
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "2", "2 件の型エントリを抽出すべき");
    let all = lines[1..].join("|");
    assert!(all.contains("Foo"), "Foo が含まれるべき: {}", all);
    assert!(all.contains("Bar"), "Bar が含まれるべき: {}", all);
    assert!(all.contains("type"), "type kind が含まれるべき: {}", all);
    assert!(
        all.contains("typealias"),
        "typealias kind が含まれるべき: {}",
        all
    );
}

// ============================================================
// CP-04: type-kind-string の全パターン網羅
// ============================================================

/// type-kind-string が 4 種の AST タグに正しい文字列を返すこと
#[test]
fn test_e2e_doctools_type_kind_string_all_variants() {
    let harness = r#"
(defn main []
  (do
    (print-string (type-kind-string (ast-type-decl)))
    (print-string "\n")
    (print-string (type-kind-string (ast-recorddef)))
    (print-string "\n")
    (print-string (type-kind-string (ast-typealias)))
    (print-string "\n")
    (print-string (type-kind-string (ast-typeconstrained)))
    (print-string "\n")
    0))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "type");
    assert_eq!(lines[1], "recorddef");
    assert_eq!(lines[2], "typealias");
    assert_eq!(lines[3], "typeconstrained");
}

// ============================================================
// CP-04: generate-review の diagnostic 構造検証
// ============================================================

/// generate-review が unused-let に対して diagnostic を生成すること
#[test]
fn test_e2e_doctools_generate_review_unused_let() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn foo [] (let [unused 42] 0))")
        review (generate-review program 1)
        source-id (vector-get review 0)
        diagnostics (vector-get review 1)]
    (do
      (print source-id)
      (print (vector-length diagnostics))
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "1", "source-id が 1 であるべき");
    let diag_count: usize = lines[1].parse().expect("diagnostics count");
    assert!(
        diag_count >= 1,
        "unused-let に対して 1 件以上の diagnostic が生成されるべき: {}",
        diag_count
    );
}

/// generate-review が clean code に対して空 diagnostics を返すこと
#[test]
fn test_e2e_doctools_generate_review_clean_code() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y))")
        review (generate-review program 1)
        diagnostics (vector-get review 1)]
    (do
      (print (vector-length diagnostics))
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(
        lines[0], "0",
        "clean code に対しては 0 件の diagnostic を返すべき"
    );
}

/// review-summary-title が診断ありの場合にタイトルを返すこと
#[test]
fn test_e2e_doctools_review_summary_title_with_diagnostics() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn foo [] (let [unused 42] 0))")
        review (generate-review program 1)
        diagnostics (vector-get review 1)
        title (review-summary-title diagnostics)]
    (do
      (print-string title)
      0)))
"#;
    let lines = run_doctools(harness);
    assert!(
        !lines.is_empty() && !lines[0].is_empty(),
        "diagnostics ありの場合は非空のタイトルを返すべき"
    );
}

/// review-summary-title が clean code で "clean" を返すこと
#[test]
fn test_e2e_doctools_review_summary_title_clean() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y))")
        review (generate-review program 1)
        diagnostics (vector-get review 1)
        title (review-summary-title diagnostics)]
    (do
      (print-string title)
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "clean", "clean code では 'clean' を返すべき");
}

// ============================================================
// CP-04: generate-doc-output の構造検証
// ============================================================

/// generate-doc-output が 5 要素の doc vector を返すこと
#[test]
fn test_e2e_doctools_generate_doc_output_structure() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn add [x y] (+ x y)) (type Num Int))")
        doc (generate-doc-output program 42)
        sections (vector-get doc 4)
        section0 (vector-get sections 0)
        section1 (vector-get sections 1)]
    (do
      (print (vector-get doc 0))
      (print (vector-length (vector-get doc 1)))
      (print (vector-length (vector-get doc 2)))
      (print-string (vector-get doc 3))
      (print-string "\n")
      (print (vector-length sections))
      (print-string (vector-get section0 0))
      (print-string "\n")
      (print (vector-get section0 1))
      (print-string (vector-get section1 0))
      (print-string "\n")
      (print (vector-get section1 1))
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "42", "module-id が 42 であるべき");
    assert_eq!(lines[1], "1", "functions が 1 件であるべき");
    assert_eq!(lines[2], "1", "types が 1 件であるべき");
    assert!(
        lines[3].contains("Demo"),
        "title に module 名 Demo が含まれるべき: {}",
        lines[3]
    );
    assert_eq!(lines[4], "2", "sections が 2 件であるべき");
    assert_eq!(lines[5], "functions", "先頭 section は functions であるべき");
    assert_eq!(lines[6], "1", "functions section count は 1 であるべき");
    assert_eq!(lines[7], "types", "2 件目 section は types であるべき");
    assert_eq!(lines[8], "1", "types section count は 1 であるべき");
}

// ============================================================
// CP-04: generate-knowledge の構造検証
// ============================================================

/// generate-knowledge が [module-id, functions, types] を返すこと
#[test]
fn test_e2e_doctools_generate_knowledge_structure() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] (+ x y)) (defn sub [a b] (- a b))")
        knowledge (generate-knowledge program 99)]
    (do
      (print (vector-get knowledge 0))
      (print (vector-length (vector-get knowledge 1)))
      (print (vector-length (vector-get knowledge 2)))
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "99", "module-id が 99 であるべき");
    assert_eq!(lines[1], "2", "functions が 2 件であるべき");
    assert_eq!(lines[2], "0", "types が 0 件であるべき");
}

// ============================================================
// CP-04: HtmlDoc XSS safety テスト
// ============================================================

/// module title に <script> を含む場合も HTML エスケープされること
#[test]
fn test_e2e_htmldoc_module_page_title_xss_safety() {
    let harness = r#"
(defn main []
  (let [tag 1
        title "<script>alert(1)</script>"
        body "safe"
        functions (vector-new 0)
        types (vector-new 0)
        html-data (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) tag) title) body) functions) types)
        result (render-module-page html-data)]
    (do
      (print-string result)
      0)))
"#;
    let lines = run_html(harness);
    let output = lines.join("\n");
    assert!(
        !output.contains("<script>"),
        "title の <script> は raw で出力されるべきではない: {}",
        output
    );
    assert!(
        output.contains("&lt;script&gt;"),
        "title は &lt;script&gt; にエスケープされるべき: {}",
        output
    );
}

/// 関数名に & を含む場合も HTML エスケープされること
#[test]
fn test_e2e_htmldoc_function_signature_ampersand_escape() {
    let harness = r#"
(defn main []
  (let [func-doc (vector-push (vector-push (vector-push (vector-new 3) 42) "a&b") 1)
        result (render-function-signature func-doc)]
    (do
      (print-string result)
      0)))
"#;
    let lines = run_html(harness);
    let output = lines.join("\n");
    assert!(
        output.contains("&amp;"),
        "& は &amp; にエスケープされるべき: {}",
        output
    );
}

// ============================================================
// CP-04: sort-doc-entries の安定性テスト
// ============================================================

/// sort-doc-entries が 5 件以上のエントリを安定ソートできること
#[test]
fn test_e2e_doctools_sort_entries_stability() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn alpha [] 1) (defn beta [] 2) (defn gamma [] 3) (defn delta [] 4) (defn epsilon [] 5)")
        entries (extract-function-entries program)
        sorted (sort-doc-entries entries)
        sorted2 (sort-doc-entries entries)]
    (do
      (print (vector-length sorted))
      (print (if (= (vector-get (vector-get sorted 0) 0) (vector-get (vector-get sorted2 0) 0)) 1 0))
      (print (if (= (vector-get (vector-get sorted 4) 0) (vector-get (vector-get sorted2 4) 0)) 1 0))
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "5", "5 件のソート済みエントリを返すべき");
    assert_eq!(lines[1], "1", "1回目と2回目のソートで先頭が一致すべき");
    assert_eq!(lines[2], "1", "1回目と2回目のソートで末尾が一致すべき");
}
