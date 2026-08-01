use super::support::*;

fn run_parser_runtime(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

#[test]
fn test_e2e_selfhost_parser_metadata_outer_scanners_use_bounded_rooted_chunks() {
    let source = selfhost_module("Parser.ls");
    for name in [
        "parse-defn-metadata-step-64-loop-bounded",
        "parse-defn-metadata-loop-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "Parser の outer metadata scanner は {} を持つべき",
            name
        );
    }
    assert_eq!(
        source
            .matches("(parse-defn-metadata-loop-v3 spans pos-ref src")
            .count(),
        1,
        "metadata handler は旧 outer loop へ直接再帰してはいけない"
    );
}

#[test]
fn test_e2e_selfhost_parser_metadata_outer_scanners_preserve_cross_chunk_results() {
    let source = (0..65)
        .map(|_| ":property []")
        .collect::<Vec<_>>()
        .join(" ");
    let harness = format!(
        r#"
(defn main []
  (let [src "{source}"
        spans (tokenize-with-spans src)
        pos-ref (ref-new 0)
        meta (parse-defn-metadata-v3 spans pos-ref src)]
    (do
      (print (vector-length (vector-get meta 5)))
      (print (p-current spans pos-ref))
      0)))
"#,
        source = source.replace('"', "\\\""),
    );
    let output = run_parser_runtime(&harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        ["65", "99"],
        "Parser の outer metadata scanner は64 directive境界を跨いでも件数と cursor を保持するべき"
    );
}
