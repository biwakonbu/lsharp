use super::support::*;

fn selfhost_html_layout_runtime_bundle() -> String {
    format!(
        "{}\n{}",
        selfhost_module("HtmlTemplate.ls"),
        selfhost_module("HtmlLayout.ls")
    )
}

/// base-layout が <!doctype html> で始まる
#[test]
fn test_e2e_selfhost_html_layout_base_has_doctype() {
    let harness = r#"
(defn main []
  (let [html (base-layout "Test" "<p>hi</p>")
        prefix (substring html 0 15)]
    (do
      (print (if (string-eq prefix "<!doctype html>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_layout_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// base-layout が <meta charset="utf-8"> を含む
#[test]
fn test_e2e_selfhost_html_layout_base_has_charset() {
    // charset 文字列が含まれるか string-length の差分で検証
    let harness = r#"
(defn contains-check [html target idx len tlen]
  (if (> idx (- len tlen))
    0
    (if (string-eq (substring html idx (+ idx tlen)) target)
      1
      (contains-check html target (+ idx 1) len tlen))))

(defn main []
  (let [html (base-layout "Test" "<p>hi</p>")
        len (string-length html)
        target "charset=\"utf-8\""
        tlen (string-length target)]
    (do
      (print (contains-check html target 0 len tlen))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_layout_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// base-layout のタイトルが HTML エスケープされる
#[test]
fn test_e2e_selfhost_html_layout_title_escaped() {
    let harness = r#"
(defn contains-check [html target idx len tlen]
  (if (> idx (- len tlen))
    0
    (if (string-eq (substring html idx (+ idx tlen)) target)
      1
      (contains-check html target (+ idx 1) len tlen))))

(defn main []
  (let [html (base-layout "<script>" "<p>hi</p>")
        len (string-length html)
        ;; タイトルがエスケープされているので "&lt;script&gt;" が含まれる
        target "&lt;script&gt;"
        tlen (string-length target)]
    (do
      (print (contains-check html target 0 len tlen))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_layout_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// doc-page-layout がモジュールページ構造を持つ
#[test]
fn test_e2e_selfhost_html_layout_doc_page() {
    let harness = r#"
(defn contains-check [html target idx len tlen]
  (if (> idx (- len tlen))
    0
    (if (string-eq (substring html idx (+ idx tlen)) target)
      1
      (contains-check html target (+ idx 1) len tlen))))

(defn main []
  (let [html (doc-page-layout "MyModule" "<section>fn</section>" "<section>ty</section>")
        len (string-length html)]
    (do
      ;; <main><h1> が含まれる
      (print (contains-check html "<main><h1>" 0 len 10))
      ;; MyModule が含まれる
      (print (contains-check html "MyModule" 0 len 8))
      ;; </main> が含まれる
      (print (contains-check html "</main>" 0 len 7))
      ;; <!doctype html> で始まる
      (print (if (string-eq (substring html 0 15) "<!doctype html>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_layout_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1", "1"]);
}

/// index-page-layout がインデックスページ構造を持つ
#[test]
fn test_e2e_selfhost_html_layout_index_page() {
    let harness = r#"
(defn contains-check [html target idx len tlen]
  (if (> idx (- len tlen))
    0
    (if (string-eq (substring html idx (+ idx tlen)) target)
      1
      (contains-check html target (+ idx 1) len tlen))))

(defn main []
  (let [html (index-page-layout "<li>Mod1</li><li>Mod2</li>")
        len (string-length html)]
    (do
      ;; <h1>modules</h1> が含まれる
      (print (contains-check html "<h1>modules</h1>" 0 len 16))
      ;; <ul> が含まれる
      (print (contains-check html "<ul>" 0 len 4))
      ;; <li>Mod1</li> が含まれる
      (print (contains-check html "<li>Mod1</li>" 0 len 13))
      ;; <!doctype html> で始まる
      (print (if (string-eq (substring html 0 15) "<!doctype html>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_layout_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1", "1"]);
}

// === HtmlLayout エッジケーステスト ===

/// base-layout に空 content を渡した場合
#[test]
fn test_e2e_selfhost_html_layout_empty_content() {
    let harness = r#"
(defn contains-check [html target idx len tlen]
  (if (> idx (- len tlen))
    0
    (if (string-eq (substring html idx (+ idx tlen)) target)
      1
      (contains-check html target (+ idx 1) len tlen))))

(defn main []
  (let [html (base-layout "Empty" "")
        len (string-length html)]
    (do
      ;; 空 content でも完全な HTML 構造を持つ
      (print (if (string-eq (substring html 0 15) "<!doctype html>") 1 0))
      (print (contains-check html "</body></html>" 0 len 14))
      ;; body 直後に閉じタグ (content が空なので)
      (print (contains-check html "<body></body>" 0 len 13))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_layout_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1"]);
}

/// doc-page-layout に空 functions/types を渡した場合
#[test]
fn test_e2e_selfhost_html_layout_doc_page_empty_sections() {
    let harness = r#"
(defn contains-check [html target idx len tlen]
  (if (> idx (- len tlen))
    0
    (if (string-eq (substring html idx (+ idx tlen)) target)
      1
      (contains-check html target (+ idx 1) len tlen))))

(defn main []
  (let [html (doc-page-layout "EmptyModule" "" "")
        len (string-length html)]
    (do
      ;; タイトルは存在する
      (print (contains-check html "<h1>EmptyModule</h1>" 0 len 20))
      ;; <main> と </main> は存在する
      (print (contains-check html "<main>" 0 len 6))
      (print (contains-check html "</main>" 0 len 7))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_layout_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1"]);
}

/// base-layout の CSS 存在検証
#[test]
fn test_e2e_selfhost_html_layout_css_presence() {
    let harness = r#"
(defn contains-check [html target idx len tlen]
  (if (> idx (- len tlen))
    0
    (if (string-eq (substring html idx (+ idx tlen)) target)
      1
      (contains-check html target (+ idx 1) len tlen))))

(defn main []
  (let [html (base-layout "Test" "<p>hi</p>")
        len (string-length html)]
    (do
      ;; <style> タグが存在する
      (print (contains-check html "<style>" 0 len 7))
      ;; CSS に font-family が含まれる
      (print (contains-check html "font-family" 0 len 11))
      ;; </style> が存在する
      (print (contains-check html "</style>" 0 len 8))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_layout_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1", "1"]);
}
