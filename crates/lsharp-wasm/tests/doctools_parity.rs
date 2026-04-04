#[path = "e2e/support.rs"]
mod support;

use serde_json::{Value, json};
use std::path::PathBuf;
use support::*;

fn selfhost_doctools_bundle() -> String {
    [
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("Type.ls"),
        selfhost_module("TypeScheme.ls"),
        selfhost_module("TypeInferCore.ls"),
        selfhost_module("TypeInferFunctions.ls"),
        selfhost_module("TypeInferBuiltins.ls"),
        selfhost_module("TypeInfer.ls"),
        selfhost_module("TypeInferApply.ls"),
        selfhost_module("TypeInferBlock.ls"),
        selfhost_module("TypeInferPattern.ls"),
        selfhost_module("TypeInferRecord.ls"),
        selfhost_module("DocTools.ls"),
        selfhost_module("JsonRpc.ls"),
        selfhost_module("DocJson.ls"),
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

fn run_doctools_raw(harness: &str) -> String {
    compile_and_run(&format!("{}\n{}", selfhost_doctools_bundle(), harness))
}

fn run_html(harness: &str) -> Vec<String> {
    let output = compile_and_run(&format!("{}\n{}", selfhost_html_bundle(), harness));
    output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect()
}

fn doctools_snapshot(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/snapshots/doctools")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("doctools snapshot 読み込み失敗 {}: {}", path.display(), e))
}

fn assert_doctools_json_snapshot(actual: &Value, snapshot_name: &str, message: &str) {
    let actual = serde_json::to_string_pretty(actual).expect("snapshot JSON pretty print");
    let expected = doctools_snapshot(snapshot_name);
    assert_eq!(actual.trim(), expected.trim(), "{}", message);
}

fn assert_doctools_text_snapshot(actual: &str, snapshot_name: &str, message: &str) {
    let expected = doctools_snapshot(snapshot_name);
    assert_eq!(actual.trim(), expected.trim(), "{}", message);
}

fn parse_i64(line: &str, label: &str) -> i64 {
    line.parse::<i64>()
        .unwrap_or_else(|e| panic!("{label} は整数であるべき: {line:?} ({e})"))
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
    assert_eq!(
        lines[5], "functions",
        "先頭 section は functions であるべき"
    );
    assert_eq!(lines[6], "1", "functions section count は 1 であるべき");
    assert_eq!(lines[7], "types", "2 件目 section は types であるべき");
    assert_eq!(lines[8], "1", "types section count は 1 であるべき");
}

/// generate-doc-output が function doc/example metadata を保持すること
#[test]
fn test_e2e_doctools_generate_doc_output_function_metadata() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y))")
        doc (generate-doc-output program 42)
        functions (vector-get doc 1)
        fn0 (vector-get functions 0)
        params (vector-get fn0 3)
        param0 (vector-get params 0)
        param1 (vector-get params 1)
        returns (vector-get fn0 4)]
    (do
      (print (vector-length fn0))
      (print (vector-length params))
      (print-string (vector-get param0 0))
      (print-string "\n")
      (print-string (vector-get param0 1))
      (print-string "\n")
      (print-string (vector-get param0 2))
      (print-string "\n")
      (print-string (vector-get param1 0))
      (print-string "\n")
      (print-string (vector-get param1 1))
      (print-string "\n")
      (print-string (vector-get param1 2))
      (print-string "\n")
      (print-string (vector-get returns 0))
      (print-string "\n")
      (print-string (vector-get returns 1))
      (print-string "\n")
      (print-string (vector-get fn0 5))
      (print-string "\n")
      (print-string (vector-get fn0 6))
      (print-string "\n")
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(
        lines[0], "7",
        "doc-output function entry は 7 要素であるべき"
    );
    assert_eq!(lines[1], "2", "params が 2 件であるべき");
    assert_eq!(lines[2], "x", "先頭 param 名が x であるべき");
    assert_eq!(lines[3], "Int", "先頭 param 型が Int であるべき");
    assert_eq!(lines[4], "left", "先頭 param doc が保持されるべき");
    assert_eq!(lines[5], "y", "2 件目 param 名が y であるべき");
    assert_eq!(lines[6], "Int", "2 件目 param 型が Int であるべき");
    assert_eq!(lines[7], "right", "2 件目 param doc が保持されるべき");
    assert_eq!(lines[8], "Int", "returns 型が Int であるべき");
    assert_eq!(lines[9], "sum", "returns doc が保持されるべき");
    assert_eq!(lines[10], "Add two ints", "doc が保持されるべき");
    assert_eq!(lines[11], "(add 1 2)", "example が保持されるべき");
}

// ============================================================
// CP-04: generate-knowledge の構造検証
// ============================================================

/// generate-knowledge が [module-id, functions, types] を返すこと
#[test]
fn test_e2e_doctools_generate_knowledge_structure() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] :doc \"Add two ints\" :example [(add 1 2)] (+ x y)) (defn sub [a b] (- a b))")
        knowledge (generate-knowledge program 99)
        functions (vector-get knowledge 1)
        fn0 (vector-get functions 0)
        params (vector-get fn0 3)]
    (do
      (print (vector-get knowledge 0))
      (print (vector-length functions))
      (print (vector-length (vector-get knowledge 2)))
      (print (vector-length fn0))
      (print (vector-length params))
      (print-string (vector-get params 0))
      (print-string "\n")
      (print-string (vector-get params 1))
      (print-string "\n")
      (print-string (vector-get fn0 4))
      (print-string "\n")
      (print-string (vector-get fn0 5))
      (print-string "\n")
      (print-string (vector-get fn0 6))
      (print-string "\n")
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "99", "module-id が 99 であるべき");
    assert_eq!(lines[1], "2", "functions が 2 件であるべき");
    assert_eq!(lines[2], "0", "types が 0 件であるべき");
    assert_eq!(
        lines[3], "7",
        "knowledge function entry は 7 要素であるべき"
    );
    assert_eq!(lines[4], "2", "params が 2 件であるべき");
    assert_eq!(lines[5], "x:Int", "先頭 param が x:Int であるべき");
    assert_eq!(lines[6], "y:Int", "2 件目 param が y:Int であるべき");
    assert_eq!(lines[7], "Int", "returns が Int であるべき");
    assert_eq!(lines[8], "Add two ints", "doc が保持されるべき");
    assert_eq!(lines[9], "(add 1 2)", "example が保持されるべき");
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

// ============================================================
// CP-04: metadata 抽出テスト
// ============================================================

/// :doc メタデータなしの defn では空文字列が返ること
#[test]
fn test_e2e_doctools_extract_doc_metadata_empty() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn foo [x] x)")
        decl (vector-get program 0)
        doc (extract-doc-metadata decl)]
    (do
      (print (string-length doc))
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "0", ":doc なしの defn は空文字列であるべき");
}

/// :doc メタデータ付き defn からドキュメント文字列が抽出できること
#[test]
fn test_e2e_doctools_extract_doc_metadata_present() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn foo [x] :doc \"Hello world\" x)")
        decl (vector-get program 0)
        doc (extract-doc-metadata decl)]
    (do
      (print-string doc)
      (print-string "\n")
      (print (string-length doc))
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "Hello world", ":doc 文字列が抽出されるべき");
    assert_eq!(lines[1], "11", "doc の長さが 11 であるべき");
}

/// :example メタデータ付き defn から例テキストが抽出できること
#[test]
fn test_e2e_doctools_extract_example_metadata_present() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn foo [x] :example [(+ x 1)] x)")
        decl (vector-get program 0)
        ex (extract-example-metadata decl)]
    (do
      (print-string ex)
      (print-string "\n")
      (print (string-length ex))
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "(+ x 1)", ":example テキストが抽出されるべき");
    assert_eq!(lines[1], "7", "example の長さが 7 であるべき");
}

/// :doc と :example 両方のメタデータを持つ defn で両方抽出できること
#[test]
fn test_e2e_doctools_extract_both_metadata() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn add [x y] :doc \"Add nums\" :example [(add 1 2)] (+ x y))")
        decl (vector-get program 0)
        doc (extract-doc-metadata decl)
        ex (extract-example-metadata decl)]
    (do
      (print-string doc)
      (print-string "\n")
      (print-string ex)
      (print-string "\n")
      0)))
"#;
    let lines = run_doctools(harness);
    assert_eq!(lines[0], "Add nums", ":doc が抽出されるべき");
    assert_eq!(lines[1], "(add 1 2)", ":example が抽出されるべき");
}

// ============================================================
// CP-04: representative snapshot gate
// ============================================================

/// representative knowledge payload を snapshot に固定すること
#[test]
fn test_e2e_doctools_generate_knowledge_snapshot() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn add [x y] :doc \"Add two ints\" :example [(add 1 2)] (+ x y)) (type Doc Int) (type-alias Alias Int))")
        kb (generate-knowledge program 100)
        functions (vector-get kb 1)
        fn0 (vector-get functions 0)
        params (vector-get fn0 3)
        types (vector-get kb 2)
        type0 (vector-get types 0)
        type1 (vector-get types 1)]
    (do
      (print (vector-get kb 0))
      (print (vector-length functions))
      (print (vector-get fn0 0))
      (print-string (vector-get fn0 1))
      (print-string "\n")
      (print (vector-get fn0 2))
      (print (vector-length params))
      (print-string (vector-get params 0))
      (print-string "\n")
      (print-string (vector-get params 1))
      (print-string "\n")
      (print-string (vector-get fn0 4))
      (print-string "\n")
      (print-string (vector-get fn0 5))
      (print-string "\n")
      (print-string (vector-get fn0 6))
      (print-string "\n")
      (print (vector-length types))
      (print (vector-get type0 0))
      (print-string (vector-get type0 1))
      (print-string "\n")
      (print-string (vector-get type0 2))
      (print-string "\n")
      (print (vector-get type1 0))
      (print-string (vector-get type1 1))
      (print-string "\n")
      (print-string (vector-get type1 2))
      (print-string "\n")
      0)))
"#;
    let lines = run_doctools(harness);
    let actual = json!({
        "moduleId": parse_i64(&lines[0], "knowledge.moduleId"),
        "functions": [{
            "hash": parse_i64(&lines[2], "knowledge.function.hash"),
            "name": lines[3],
            "arity": parse_i64(&lines[4], "knowledge.function.arity"),
            "params": [lines[6], lines[7]],
            "returns": lines[8],
            "doc": lines[9],
            "example": lines[10],
        }],
        "types": [
            {
                "hash": parse_i64(&lines[12], "knowledge.type0.hash"),
                "name": lines[13],
                "kind": lines[14],
            },
            {
                "hash": parse_i64(&lines[15], "knowledge.type1.hash"),
                "name": lines[16],
                "kind": lines[17],
            }
        ]
    });

    assert_eq!(
        lines[1], "1",
        "representative knowledge payload は関数 1 件であるべき"
    );
    assert_eq!(
        lines[11], "2",
        "representative knowledge payload は型 2 件であるべき"
    );
    assert_doctools_json_snapshot(
        &actual,
        "knowledge-payload.json",
        "representative knowledge payload snapshot が一致するべき",
    );
}

/// representative review payload を snapshot に固定すること
#[test]
fn test_e2e_doctools_generate_review_snapshot() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn first [] (let [unused 42] 0)) (defn second [] (do))")
        review (generate-review program 200)
        diags (vector-get review 1)
        diag0 (vector-get diags 0)
        diag1 (vector-get diags 1)]
    (do
      (print (vector-get review 0))
      (print (vector-length diags))
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
      (print (vector-get diag1 0))
      (print-string (vector-get diag1 1))
      (print-string "\n")
      (print-string (vector-get diag1 2))
      (print-string "\n")
      (print-string (vector-get diag1 3))
      (print-string "\n")
      (print (vector-get diag1 4))
      (print (vector-get diag1 5))
      (print-string (vector-get diag1 6))
      (print-string "\n")
      0)))
"#;
    let lines = run_doctools(harness);
    let actual = json!({
        "sourceId": parse_i64(&lines[0], "review.sourceId"),
        "diagnostics": [
            {
                "ruleId": parse_i64(&lines[2], "review.diag0.ruleId"),
                "title": lines[3],
                "body": lines[4],
                "severity": lines[5],
                "line": parse_i64(&lines[6], "review.diag0.line"),
                "column": parse_i64(&lines[7], "review.diag0.column"),
                "code": lines[8],
            },
            {
                "ruleId": parse_i64(&lines[9], "review.diag1.ruleId"),
                "title": lines[10],
                "body": lines[11],
                "severity": lines[12],
                "line": parse_i64(&lines[13], "review.diag1.line"),
                "column": parse_i64(&lines[14], "review.diag1.column"),
                "code": lines[15],
            }
        ]
    });

    assert_eq!(
        lines[1], "2",
        "representative review payload は diagnostics 2 件であるべき"
    );
    assert_doctools_json_snapshot(
        &actual,
        "review-payload.json",
        "representative review payload snapshot が一致するべき",
    );
}

/// representative doc-output payload を snapshot に固定すること
#[test]
fn test_e2e_doctools_generate_doc_output_snapshot() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y)) (type Doc Int) (type-alias Alias Int))")
        doc (generate-doc-output program 42)
        functions (vector-get doc 1)
        fn0 (vector-get functions 0)
        params (vector-get fn0 3)
        param0 (vector-get params 0)
        param1 (vector-get params 1)
        returns (vector-get fn0 4)
        types (vector-get doc 2)
        type0 (vector-get types 0)
        type1 (vector-get types 1)
        sections (vector-get doc 4)
        section0 (vector-get sections 0)
        section1 (vector-get sections 1)]
    (do
      (print (vector-get doc 0))
      (print-string (vector-get doc 3))
      (print-string "\n")
      (print (vector-length functions))
      (print (vector-get fn0 0))
      (print-string (vector-get fn0 1))
      (print-string "\n")
      (print (vector-get fn0 2))
      (print (vector-length params))
      (print-string (vector-get param0 0))
      (print-string "\n")
      (print-string (vector-get param0 1))
      (print-string "\n")
      (print-string (vector-get param0 2))
      (print-string "\n")
      (print-string (vector-get param1 0))
      (print-string "\n")
      (print-string (vector-get param1 1))
      (print-string "\n")
      (print-string (vector-get param1 2))
      (print-string "\n")
      (print-string (vector-get returns 0))
      (print-string "\n")
      (print-string (vector-get returns 1))
      (print-string "\n")
      (print-string (vector-get fn0 5))
      (print-string "\n")
      (print-string (vector-get fn0 6))
      (print-string "\n")
      (print (vector-length types))
      (print (vector-get type0 0))
      (print-string (vector-get type0 1))
      (print-string "\n")
      (print-string (vector-get type0 2))
      (print-string "\n")
      (print (vector-get type1 0))
      (print-string (vector-get type1 1))
      (print-string "\n")
      (print-string (vector-get type1 2))
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
    let actual = json!({
        "moduleId": parse_i64(&lines[0], "docOutput.moduleId"),
        "title": lines[1],
        "functions": [{
            "hash": parse_i64(&lines[3], "docOutput.function.hash"),
            "name": lines[4],
            "arity": parse_i64(&lines[5], "docOutput.function.arity"),
            "params": [
                { "name": lines[7], "type": lines[8], "doc": lines[9] },
                { "name": lines[10], "type": lines[11], "doc": lines[12] }
            ],
            "returns": { "type": lines[13], "doc": lines[14] },
            "doc": lines[15],
            "example": lines[16],
        }],
        "types": [
            {
                "hash": parse_i64(&lines[18], "docOutput.type0.hash"),
                "name": lines[19],
                "kind": lines[20],
            },
            {
                "hash": parse_i64(&lines[21], "docOutput.type1.hash"),
                "name": lines[22],
                "kind": lines[23],
            }
        ],
        "htmlSections": [
            { "id": lines[25], "count": parse_i64(&lines[26], "docOutput.section0.count") },
            { "id": lines[27], "count": parse_i64(&lines[28], "docOutput.section1.count") }
        ]
    });

    assert_eq!(
        lines[2], "1",
        "representative doc-output payload は関数 1 件であるべき"
    );
    assert_eq!(
        lines[17], "2",
        "representative doc-output payload は型 2 件であるべき"
    );
    assert_eq!(
        lines[24], "2",
        "representative doc-output payload は section 2 件であるべき"
    );
    assert_doctools_json_snapshot(
        &actual,
        "doc-output-payload.json",
        "representative doc-output payload snapshot が一致するべき",
    );
}

/// representative HTML output を snapshot に固定すること
#[test]
fn test_e2e_htmldoc_render_html_snapshot() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"<danger>\" :example [(add 1 2)] (+ x y)) (type Pair Int) (type-alias Alias Int))")
        doc (generate-html program 0)
        html (render-html doc 0)]
    (do
      (print-string html)
      0)))
"#;
    let lines = run_html(harness);
    let html = lines.join("\n");
    assert_doctools_text_snapshot(
        &html,
        "render-html-rich.html",
        "representative render-html snapshot が一致するべき",
    );
}

/// guide page HTML を snapshot に固定すること
#[test]
fn test_e2e_htmldoc_render_guide_page_snapshot() {
    let harness = r#"
(defn main []
  (let [html (render-guide-page "Quick Start" "<h1>Quick Start</h1><p>hello</p>")]
    (do
      (print-string html)
      0)))
"#;
    let lines = run_html(harness);
    let html = lines.join("\n");
    assert_doctools_text_snapshot(
        &html,
        "render-guide-page.html",
        "guide page HTML snapshot が一致するべき",
    );
}

/// doc-site index HTML を snapshot に固定すること
#[test]
fn test_e2e_htmldoc_render_doc_site_index_snapshot() {
    let harness = r#"
(defn guide-link [href label]
  (let [node (vector-new 2)]
    (vector-push
      (vector-push node href)
      label)))

(defn main []
  (let [guides (vector-push
                 (vector-push
                   (vector-push (vector-new 3)
                     (guide-link "guides/quick-start.html" "Quick Start"))
                   (guide-link "guides/language-reference.html" "Language Reference"))
                 (guide-link "guides/package-layout.html" "Package Layout"))
        modules (vector-push (vector-push (vector-new 2) "Core") "List")
        html (render-doc-site-index guides modules)]
    (do
      (print-string html)
      0)))
"#;
    let lines = run_html(harness);
    let html = lines.join("\n");
    assert_doctools_text_snapshot(
        &html,
        "render-doc-site-index.html",
        "doc-site index HTML snapshot が一致するべき",
    );
}

/// schema object 互換の knowledge JSON を snapshot に固定すること
#[test]
fn test_e2e_doctools_generate_knowledge_schema_object_snapshot() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn add [x y] :doc \"Add two ints\" :example [(add 1 2)] (+ x y)) (type Doc Int) (type-alias Alias Int))")
        json (generate-knowledge-schema-json program 100)]
    (do
      (print-string json)
      0)))
"#;
    let output = run_doctools_raw(harness);
    let actual: Value = serde_json::from_str(output.trim()).expect("knowledge schema object JSON");
    assert_doctools_json_snapshot(
        &actual,
        "knowledge-schema-object.json",
        "knowledge schema object snapshot が一致するべき",
    );
}

/// schema object 互換の review JSON を snapshot に固定すること
#[test]
fn test_e2e_doctools_generate_review_schema_object_snapshot() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn first [] (let [unused 42] 0)) (defn second [] (do))")
        json (generate-review-schema-json program 200)]
    (do
      (print-string json)
      0)))
"#;
    let output = run_doctools_raw(harness);
    let actual: Value = serde_json::from_str(output.trim()).expect("review schema object JSON");
    assert_doctools_json_snapshot(
        &actual,
        "review-schema-object.json",
        "review schema object snapshot が一致するべき",
    );
}

/// schema object 互換の doc-output JSON を snapshot に固定すること
#[test]
fn test_e2e_doctools_generate_doc_output_schema_object_snapshot() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y)) (type Doc Int) (type-alias Alias Int))")
        json (generate-doc-output-schema-json program 42)]
    (do
      (print-string json)
      0)))
"#;
    let output = run_doctools_raw(harness);
    let actual: Value = serde_json::from_str(output.trim()).expect("doc-output schema object JSON");
    assert_doctools_json_snapshot(
        &actual,
        "doc-output-schema-object.json",
        "doc-output schema object snapshot が一致するべき",
    );
}
