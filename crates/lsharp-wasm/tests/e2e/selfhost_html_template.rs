use super::support::*;

fn selfhost_html_template_runtime_bundle() -> String {
    include_str!("../../../../selfhost/HtmlTemplate.ls").to_string()
}

// === Step 1: html-escape テスト ===

/// html-escape が < > & を正しくエスケープする
#[test]
fn test_e2e_selfhost_html_template_escape_lt_gt_amp() {
    let harness = r#"
(defn main []
  (let [result (html-escape "<>&")]
    (do
      (print (if (string-eq result "&lt;&gt;&amp;") 1 0))
      (print (string-length result))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    // string-eq で一致 = 1, "&lt;&gt;&amp;" は 13 文字 (4+4+5)
    assert_eq!(lines, vec!["1", "13"]);
}

/// html-escape が " ' を正しくエスケープする
#[test]
fn test_e2e_selfhost_html_template_escape_quotes() {
    let harness = r#"
(defn main []
  (let [result (html-escape "a\"b'c")]
    (do
      (print (if (string-eq result "a&quot;b&#39;c") 1 0))
      (print (string-length result))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    // "a&quot;b&#39;c" は 14 文字
    assert_eq!(lines, vec!["1", "14"]);
}

/// html-escape が特殊文字を含まない文字列をそのまま返す
#[test]
fn test_e2e_selfhost_html_template_escape_passthrough() {
    let harness = r#"
(defn main []
  (let [result (html-escape "hello")]
    (do
      (print (if (string-eq result "hello") 1 0))
      (print (string-length result))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "5"]);
}

// === Step 1: elem/text/raw + render-node テスト ===

/// elem + text で基本的な <div>hello</div> を生成する
#[test]
fn test_e2e_selfhost_html_template_elem_basic() {
    let harness = r#"
(defn main []
  (let [child-text (text "hello")
        children (vector-push (vector-new 1) child-text)
        node (elem "div" (vector-new 0) children)
        result (render-template node)]
    (do
      (print (if (string-eq result "<div>hello</div>") 1 0))
      (print (string-length result))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    // "<div>hello</div>" は 16 文字
    assert_eq!(lines, vec!["1", "16"]);
}

/// elem + 属性付きで <a href="...">link</a> を生成する
#[test]
fn test_e2e_selfhost_html_template_elem_attrs() {
    let harness = r#"
(defn main []
  (let [attr (vector-push (vector-push (vector-new 2) "href") "https://example.com")
        attrs (vector-push (vector-new 1) attr)
        child-text (text "link")
        children (vector-push (vector-new 1) child-text)
        node (elem "a" attrs children)
        result (render-template node)
        expected "<a href=\"https://example.com\">link</a>"]
    (do
      (print (if (string-eq result expected) 1 0))
      (print (string-length result))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines[0], "1");
}

/// void element (br, meta) は閉じタグなしで生成する
#[test]
fn test_e2e_selfhost_html_template_void_element() {
    let harness = r#"
(defn main []
  (let [br-node (elem "br" (vector-new 0) (vector-new 0))
        br-html (render-template br-node)
        meta-attr (vector-push (vector-push (vector-new 2) "charset") "utf-8")
        meta-attrs (vector-push (vector-new 1) meta-attr)
        meta-node (elem "meta" meta-attrs (vector-new 0))
        meta-html (render-template meta-node)]
    (do
      (print (if (string-eq br-html "<br>") 1 0))
      (print (if (string-eq meta-html "<meta charset=\"utf-8\">") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1"]);
}

/// ネストした要素構造 <div><p>text</p></div> を生成する
#[test]
fn test_e2e_selfhost_html_template_nested() {
    let harness = r#"
(defn main []
  (let [inner-text (text "nested")
        inner-children (vector-push (vector-new 1) inner-text)
        p-node (elem "p" (vector-new 0) inner-children)
        outer-children (vector-push (vector-new 1) p-node)
        div-node (elem "div" (vector-new 0) outer-children)
        result (render-template div-node)]
    (do
      (print (if (string-eq result "<div><p>nested</p></div>") 1 0))
      (print (string-length result))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    // "<div><p>nested</p></div>" は 24 文字
    assert_eq!(lines, vec!["1", "24"]);
}

/// raw-node はエスケープせずそのまま出力する
#[test]
fn test_e2e_selfhost_html_template_raw_passthrough() {
    let harness = r#"
(defn main []
  (let [node (raw-node "<b>bold</b>")
        result (render-node node)]
    (do
      (print (if (string-eq result "<b>bold</b>") 1 0))
      (print (string-length result))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "11"]);
}

// === Step 2: each-nodes/when-node/doctype テスト ===

/// each-nodes が複数の li ノードを連結する
#[test]
fn test_e2e_selfhost_html_template_each_nodes() {
    let harness = r#"
(defn main []
  (let [li1 (elem "li" (vector-new 0) (vector-push (vector-new 1) (text "a")))
        li2 (elem "li" (vector-new 0) (vector-push (vector-new 1) (text "b")))
        li3 (elem "li" (vector-new 0) (vector-push (vector-new 1) (text "c")))
        items (vector-push (vector-push (vector-push (vector-new 3) li1) li2) li3)
        result (each-nodes items)]
    (do
      (print (if (string-eq result "<li>a</li><li>b</li><li>c</li>") 1 0))
      (print (string-length result))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    // "<li>a</li><li>b</li><li>c</li>" は 30 文字
    assert_eq!(lines, vec!["1", "30"]);
}

/// when-node が cond=1 の場合にノードを出力する
#[test]
fn test_e2e_selfhost_html_template_when_true() {
    let harness = r#"
(defn main []
  (let [node (elem "span" (vector-new 0) (vector-push (vector-new 1) (text "visible")))
        result (when-node 1 node)]
    (do
      (print (if (string-eq result "<span>visible</span>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// when-node が cond=0 の場合に空文字列を返す
#[test]
fn test_e2e_selfhost_html_template_when_false() {
    let harness = r#"
(defn main []
  (let [node (elem "span" (vector-new 0) (vector-push (vector-new 1) (text "hidden")))
        result (when-node 0 node)]
    (do
      (print (string-length result))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "0");
}

/// 同一入力で 2 回レンダリングして同一出力 (deterministic)
#[test]
fn test_e2e_selfhost_html_template_deterministic() {
    let harness = r#"
(defn main []
  (let [child (text "test")
        children (vector-push (vector-new 1) child)
        node (elem "div" (vector-new 0) children)
        r1 (render-template node)
        r2 (render-template node)]
    (do
      (print (if (string-eq r1 r2) 1 0))
      (print (if (string-eq r1 "<div>test</div>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "1"]);
}
