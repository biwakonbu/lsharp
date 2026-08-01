use super::support::*;

fn run_parser_runtime(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

#[test]
fn test_e2e_selfhost_parser_metadata_sequences_use_bounded_rooted_chunks() {
    let source = selfhost_module("Parser.ls");
    for name in [
        "parse-source-evidence-shrinks-step-64-loop-bounded",
        "parse-source-evidence-shrinks-loop-rooted-v3",
        "parse-source-evidence-coverage-step-64-loop-bounded",
        "parse-source-evidence-coverage-loop-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "Parser の metadata sequence scanner は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_parser_metadata_sequences_preserve_cross_chunk_results() {
    let item_count = 65;
    let shrink_source = (0..item_count)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let shrink_source = format!("[{}]", shrink_source);
    let coverage_source = (0..item_count)
        .map(|value| format!("(\"bucket-{}\" {})", value, value))
        .collect::<Vec<_>>()
        .join(" ");
    let coverage_source = format!("[{}]", coverage_source);
    let escape_lsharp_string = |source: &str| source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [shrink-src "{shrink_source}"
        shrink-spans (tokenize-with-spans shrink-src)
        shrink-pos (ref-new 0)
        shrinks (parse-source-evidence-shrinks-v3 shrink-spans shrink-pos shrink-src)
        coverage-src "{coverage_source}"
        coverage-spans (tokenize-with-spans coverage-src)
        coverage-pos (ref-new 0)
        coverage (parse-source-evidence-coverage-v3 coverage-spans coverage-pos coverage-src)]
    (do
      (print (vector-length shrinks))
      (print (p-current shrink-spans shrink-pos))
      (print (vector-length coverage))
      (print (p-current coverage-spans coverage-pos))
      0)))
"#,
        shrink_source = escape_lsharp_string(&shrink_source),
        coverage_source = escape_lsharp_string(&coverage_source),
    );
    let output = run_parser_runtime(&harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        ["65", "99", "65", "99"],
        "Parser の metadata sequence scanner は64要素境界を跨いでも cursor と件数を保持するべき"
    );
}
