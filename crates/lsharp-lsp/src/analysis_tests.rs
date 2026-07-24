use super::*;

#[test]
fn test_hover_returns_type_and_doc_for_toplevel_function() {
    let source = r#"
(defn add
  [x y]
  :doc "整数加算"
  (+ x y))

(defn main []
  (add 1 2))
"#;

    let hover = hover(source, Position::new(7, 3)).expect("hover が必要");
    let HoverContents::Scalar(MarkedString::String(text)) = hover.contents else {
        panic!("hover contents は string markdown を返すべき");
    };

    assert!(text.contains("add"));
    assert!(text.contains("Int -> Int -> Int"));
    assert!(text.contains("整数加算"));
}
