use super::support::*;

#[test]
fn test_e2e_selfhost_typeinfer_assertion_scanners_use_bounded_rooted_chunks() {
    let source = selfhost_module("TypeInferAssertions.ls");
    for name in [
        "property-skip-space-step-64-loop-bounded",
        "property-find-substring-step-64-loop-bounded",
        "property-balanced-expression-end-step-64-loop-bounded",
        "property-atom-expression-end-step-64-loop-bounded",
        "property-skip-space-rooted-v3",
        "property-find-substring-rooted-v3",
        "property-balanced-expression-end-rooted-v3",
        "property-atom-expression-end-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "TypeInferAssertions の scanner は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_typeinfer_assertion_scanners_preserve_cross_chunk_indexes() {
    let width = 65;
    let spaces = " ".repeat(width);
    let haystack = format!("{}needle", "x".repeat(width));
    let balanced = format!("{}x{}", "(".repeat(width), ")".repeat(width));
    let atom = format!("{} ", "a".repeat(width));
    let harness = format!(
        r#"
(defn main []
  (let [skip-src "{spaces}x"
        find-src "{haystack}"
        balanced-src "{balanced}"
        atom-src "{atom}"]
    (do
      (print (property-skip-space skip-src 0 {skip_len}))
      (print (property-find-substring find-src "needle"))
      (print (property-balanced-expression-end balanced-src 0 {balanced_len} 0))
      (print (property-atom-expression-end atom-src 0 {atom_len}))
      0)))
"#,
        spaces = spaces,
        haystack = haystack,
        balanced = balanced,
        atom = atom,
        skip_len = width + 1,
        balanced_len = width * 2 + 1,
        atom_len = width + 1,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, ["65", "65", "131", "65"]);
}
