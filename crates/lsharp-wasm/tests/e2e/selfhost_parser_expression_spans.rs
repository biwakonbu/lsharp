use super::support::*;

fn run_parser_runtime(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

#[test]
fn test_e2e_selfhost_parser_expression_spans_use_bounded_rooted_chunks() {
    let source = selfhost_module("Parser.ls");
    for name in [
        "collect-example-expression-spans-v3-step-64-loop-bounded",
        "collect-example-expression-spans-v3-rooted",
    ] {
        assert!(
            source.contains(name),
            "Parser の expression span collector は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_parser_expression_spans_preserve_cross_chunk_results() {
    let expression_count = 129;
    let forms = std::iter::repeat_n("(a)", expression_count)
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("[{}]", forms);
    let first_start = 1;
    let first_end = first_start + 3;
    let last_start = first_start + (expression_count - 1) * 4;
    let last_end = last_start + 3;
    let harness = format!(
        r#"
(defn main []
  (let [spans (tokenize-with-spans "{source}")
        end (- (/ (vector-length spans) 3) 1)
        collected (collect-example-expression-spans-v3 spans 1 end)]
    (do
      (print (vector-length collected))
      (print (vector-get collected 0))
      (print (vector-get collected 1))
      (print (vector-get collected 256))
      (print (vector-get collected 257))
      0)))
"#,
        source = source,
    );
    let output = run_parser_runtime(&harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec![
            (expression_count * 2).to_string(),
            first_start.to_string(),
            first_end.to_string(),
            last_start.to_string(),
            last_end.to_string(),
        ],
        "Parser の expression span collector は64要素境界を跨いでも式の順序と境界を保持するべき"
    );
}
