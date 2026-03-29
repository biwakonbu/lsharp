use super::support::*;

fn selfhost_html_template_runtime_bundle() -> String {
    selfhost_module("HtmlTemplate.ls").to_string()
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

// === XSS 安全性テスト ===

/// text ノードに <script>alert(1)</script> を入れた場合にエスケープされる
#[test]
fn test_e2e_selfhost_html_template_xss_text_script_tag() {
    let harness = r#"
(defn main []
  (let [node (elem "div" (vector-new 0)
              (vector-push (vector-new 1) (text "<script>alert(1)</script>")))
        result (render-template node)]
    (do
      ;; <script> がエスケープされて &lt;script&gt; になるはず
      (print (if (string-eq result "<div>&lt;script&gt;alert(1)&lt;/script&gt;</div>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// 属性値に " を含む場合にエスケープされる (属性値 XSS)
#[test]
fn test_e2e_selfhost_html_template_xss_attr_value_quote() {
    let harness = r#"
(defn main []
  (let [attr (vector-push (vector-push (vector-new 2) "title") "a\"b")
        attrs (vector-push (vector-new 1) attr)
        node (elem "span" attrs (vector-push (vector-new 1) (text "x")))
        result (render-template node)]
    (do
      ;; " が &quot; にエスケープされるはず
      (print (if (string-eq result "<span title=\"a&quot;b\">x</span>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// 属性値に < を含む場合にエスケープされる (属性値 XSS)
#[test]
fn test_e2e_selfhost_html_template_xss_attr_value_lt() {
    let harness = r#"
(defn main []
  (let [attr (vector-push (vector-push (vector-new 2) "data") "<img onerror=alert(1)>")
        attrs (vector-push (vector-new 1) attr)
        node (elem "div" attrs (vector-push (vector-new 1) (text "safe")))
        result (render-template node)]
    (do
      ;; < > がエスケープされるはず
      (print (if (string-eq result "<div data=\"&lt;img onerror=alert(1)&gt;\">safe</div>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// text ノードに全特殊文字 <>&"' を混合した場合にすべてエスケープされる
#[test]
fn test_e2e_selfhost_html_template_xss_all_special_chars() {
    let harness = r#"
(defn main []
  (let [node (elem "p" (vector-new 0)
              (vector-push (vector-new 1) (text "<b>\"hello\"&'world'</b>")))
        result (render-template node)]
    (do
      (print (if (string-eq result "<p>&lt;b&gt;&quot;hello&quot;&amp;&#39;world&#39;&lt;/b&gt;</p>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

// === 境界値・エッジケーステスト ===

/// 空文字列のエスケープ
#[test]
fn test_e2e_selfhost_html_template_escape_empty_string() {
    let harness = r#"
(defn main []
  (let [result (html-escape "")]
    (do
      (print (if (string-eq result "") 1 0))
      (print (string-length result))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "0"]);
}

/// 空 children の非 void 要素 → <div></div>
#[test]
fn test_e2e_selfhost_html_template_elem_empty_children() {
    let harness = r#"
(defn main []
  (let [node (elem "div" (vector-new 0) (vector-new 0))
        result (render-template node)]
    (do
      (print (if (string-eq result "<div></div>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// 複数属性の要素
#[test]
fn test_e2e_selfhost_html_template_elem_multiple_attrs() {
    let harness = r#"
(defn main []
  (let [a1 (vector-push (vector-push (vector-new 2) "id") "main")
        a2 (vector-push (vector-push (vector-new 2) "class") "container")
        attrs (vector-push (vector-push (vector-new 2) a1) a2)
        node (elem "div" attrs (vector-push (vector-new 1) (text "hi")))
        result (render-template node)]
    (do
      (print (if (string-eq result "<div id=\"main\" class=\"container\">hi</div>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// 3 段ネスト <div><ul><li>deep</li></ul></div>
#[test]
fn test_e2e_selfhost_html_template_triple_nested() {
    let harness = r#"
(defn main []
  (let [li (elem "li" (vector-new 0) (vector-push (vector-new 1) (text "deep")))
        ul (elem "ul" (vector-new 0) (vector-push (vector-new 1) li))
        div (elem "div" (vector-new 0) (vector-push (vector-new 1) ul))
        result (render-template div)]
    (do
      (print (if (string-eq result "<div><ul><li>deep</li></ul></div>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// text と element の混合 children
#[test]
fn test_e2e_selfhost_html_template_mixed_children() {
    let harness = r#"
(defn main []
  (let [t1 (text "hello ")
        em (elem "em" (vector-new 0) (vector-push (vector-new 1) (text "world")))
        t2 (text "!")
        children (vector-push (vector-push (vector-push (vector-new 3) t1) em) t2)
        node (elem "p" (vector-new 0) children)
        result (render-template node)]
    (do
      (print (if (string-eq result "<p>hello <em>world</em>!</p>") 1 0))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "1");
}

/// 連続特殊文字のエスケープ
#[test]
fn test_e2e_selfhost_html_template_escape_consecutive_special() {
    let harness = r#"
(defn main []
  (let [result (html-escape "<<<>>>")]
    (do
      (print (if (string-eq result "&lt;&lt;&lt;&gt;&gt;&gt;") 1 0))
      (print (string-length result))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_html_template_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    // "&lt;&lt;&lt;&gt;&gt;&gt;" = 4*3 + 4*3 = 24 文字
    assert_eq!(lines, vec!["1", "24"]);
}
