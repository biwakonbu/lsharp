use super::support::*;

#[test]
fn test_e2e_selfhost_parser_collection_scanners_use_bounded_rooted_chunks() {
    let source = selfhost_module("Parser.ls");
    for name in [
        "vector-set-at-step-64-loop-bounded",
        "vector-set-at-rooted-v3",
        "defn-signature-param-present-step-64-loop-bounded",
        "defn-signature-param-present-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "Parser の collection scanner は {} を持つべき",
            name
        );
    }
}

fn run_parser_collection_harness(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

#[test]
fn test_e2e_selfhost_parser_vector_set_preserves_cross_chunk_results() {
    let harness = format!(
        r#"
(defn main []
  (let [values (tokenize-with-spans "(a a a a a a a a a a a a a a a a a a a a a a a a a a a a a a)")
        updated (vector-set-at values 64 999)]
    (do
      (print (vector-length values))
      (print (vector-length updated))
      (print (vector-get updated 64))
      0)))
"#
    );
    let output = run_parser_collection_harness(&harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], lines[1]);
    assert_eq!(lines[2], "999");
}

#[test]
fn test_e2e_selfhost_signature_scan_preserves_cross_chunk_results() {
    let harness = format!(
        r#"
(defn main []
  (let [signature0 (tokenize-with-spans "(a a a a a a a a a a a a a a a a a a a a a a a a a a a a a a)")
        signature1 (vector-set-at signature0 1 65)
        signature (vector-set-at signature1 66 1)]
    (do
      (print (defn-signature-param-present-v3 signature 0 65))
      (print (defn-signature-present-v3 signature))
      0)))
"#
    );
    let output = run_parser_collection_harness(&harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, ["1", "1"]);
}
