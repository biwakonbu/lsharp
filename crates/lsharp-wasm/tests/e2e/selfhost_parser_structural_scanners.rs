use super::support::*;

fn flat_atoms(count: usize) -> String {
    std::iter::repeat_n("a", count)
        .collect::<Vec<_>>()
        .join(" ")
}

fn run_parser_runtime(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

#[test]
fn test_e2e_selfhost_parser_structural_scanners_use_bounded_rooted_chunks() {
    let source = selfhost_module("Parser.ls");
    for name in [
        "parse-skip-bracket-step-64-loop-bounded",
        "parse-skip-bracket-rooted-v3",
        "parse-skip-brace-step-64-loop-bounded",
        "parse-skip-brace-rooted-v3",
        "scan-defn-param-form-end-step-64-loop-bounded",
        "scan-defn-param-form-end-rooted-v3",
        "parse-delimiter-balance-step-64-loop-bounded",
        "parse-delimiter-balance-rooted-v3",
        "recover-to-next-step-64-loop-bounded",
        "recover-to-next-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "Parser の structural scanner は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_parser_structural_scanners_preserve_cross_chunk_results() {
    let atoms = flat_atoms(129);
    let bracket_src = format!("[{}]", atoms);
    let brace_src = format!("{{{}}}", atoms);
    let paren_src = format!("({})", atoms);
    let unclosed_paren_src = format!("({}", atoms);
    let unclosed_bracket_src = format!("[{}", atoms);
    let harness = format!(
        r#"
(defn main []
  (let [bracket-spans (tokenize-with-spans "{bracket_src}")
        bracket-pos (ref-new 1)
        brace-spans (tokenize-with-spans "{brace_src}")
        brace-pos (ref-new 1)
        paren-spans (tokenize-with-spans "{paren_src}")
        unclosed-paren-spans (tokenize-with-spans "{unclosed_paren_src}")
        unclosed-bracket-spans (tokenize-with-spans "{unclosed_bracket_src}")
        recover-pos (ref-new 1)]
    (do
      (parse-skip-bracket-v3 bracket-spans bracket-pos 1)
      (parse-skip-brace-v3 brace-spans brace-pos 1)
      (print (p-current bracket-spans bracket-pos))
      (print (p-current brace-spans brace-pos))
      (print (span-kind paren-spans
                        (scan-defn-param-form-end-v3
                          paren-spans 1 (/ (vector-length paren-spans) 3) 1)))
      (print (parse-delimiter-diagnostic-code unclosed-paren-spans))
      (print (parse-delimiter-diagnostic-code unclosed-bracket-spans))
      (recover-to-next paren-spans recover-pos)
      (print (p-current paren-spans recover-pos))
      0)))
"#,
        bracket_src = bracket_src,
        brace_src = brace_src,
        paren_src = paren_src,
        unclosed_paren_src = unclosed_paren_src,
        unclosed_bracket_src = unclosed_bracket_src,
    );
    let output = run_parser_runtime(&harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        ["99", "99", "99", "1001", "1002", "1"],
        "Parser の structural/recovery scanner は64要素境界を跨いでも結果を保持するべき"
    );
}
