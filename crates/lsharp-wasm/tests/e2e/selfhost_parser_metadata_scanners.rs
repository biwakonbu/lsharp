use super::support::*;

fn run_parser_runtime(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

#[test]
fn test_e2e_selfhost_parser_metadata_scanners_use_bounded_rooted_chunks() {
    let source = selfhost_module("Parser.ls");
    for name in [
        "parse-defn-meta-case-step-64-loop-bounded",
        "parse-defn-meta-case-loop-rooted-v3",
        "parse-defn-meta-assert-step-64-loop-bounded",
        "parse-defn-meta-assert-loop-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "Parser の metadata scanner は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_parser_metadata_scanners_preserve_cross_chunk_results() {
    let item_count = 65;
    let case_items = std::iter::repeat_n("(expect 1 1)", item_count)
        .collect::<Vec<_>>()
        .join(" ");
    let assert_items = std::iter::repeat_n("(= 1 1)", item_count)
        .collect::<Vec<_>>()
        .join(" ");
    let case_source = format!("[{}]", case_items);
    let assert_source = format!("[{}]", assert_items);
    let harness = format!(
        r#"
(defn main []
  (let [case-src "{case_source}"
        case-spans (tokenize-with-spans case-src)
        case-pos (ref-new 1)
        case-values (parse-defn-meta-case-loop-v3
          case-spans case-pos case-src (vector-new 0))
        assert-src "{assert_source}"
        assert-spans (tokenize-with-spans assert-src)
        assert-pos (ref-new 1)
        assert-values (parse-defn-meta-assert-loop-v3
          assert-spans assert-pos assert-src (vector-new 0))]
    (do
      (print (vector-length case-values))
      (print (p-current case-spans case-pos))
      (print (vector-length assert-values))
      (print (p-current assert-spans assert-pos))
      0)))
"#,
        case_source = case_source,
        assert_source = assert_source,
    );
    let output = run_parser_runtime(&harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        ["65", "99", "65", "99"],
        "Parser の metadata scanner は64要素境界を跨いでも case/assert の件数と cursor を保持するべき"
    );
}
