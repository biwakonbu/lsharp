use super::support::*;

fn run_parser_runtime(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

#[test]
fn test_e2e_selfhost_parser_signature_scanners_use_bounded_rooted_chunks() {
    let source = selfhost_module("Parser.ls");
    for name in [
        "parse-defn-param-signature-step-64-loop-bounded",
        "parse-defn-param-signature-loop-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "Parser の signature scanner は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_parser_signature_scanners_preserve_cross_chunk_results() {
    let param_count = 65;
    let params = (0..param_count)
        .map(|index| {
            let type_name = if index + 1 == param_count {
                "String"
            } else {
                "Int"
            };
            format!("(: p{} {})", index, type_name)
        })
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(defn many [{}] : Bool 0)", params);
    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{source}") 0)
        count (vector-get node 2)
        signature (vector-get node (+ count 4))
        first-type (vector-get signature 2)
        last-type (vector-get signature (+ count 1))
        return-type (vector-get signature (+ count 2))]
    (do
      (print count)
      (print (vector-get signature 0))
      (print (vector-get signature 1))
      (print (if (= (vector-get first-type 0) (ast-type-named)) 1 0))
      (print (if (= (vector-get first-type 1) (name-hash "Int" 0 3)) 1 0))
      (print (if (= (vector-get last-type 0) (ast-type-named)) 1 0))
      (print (if (= (vector-get last-type 1) (name-hash "String" 0 6)) 1 0))
      (print (if (= (vector-get return-type 0) (ast-type-named)) 1 0))
      (print (if (= (vector-get return-type 1) (name-hash "Bool" 0 4)) 1 0))
      0)))
"#,
        source = source.replace('"', "\\\""),
    );
    let output = run_parser_runtime(&harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        ["65", "65", "65", "1", "1", "1", "1", "1", "1"],
        "Parser の typed signature scanner は64要素境界を跨いでも型の順序と末尾を保持するべき"
    );
}
