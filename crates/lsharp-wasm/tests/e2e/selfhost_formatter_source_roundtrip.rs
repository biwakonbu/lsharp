use super::support::*;

fn selfhost_formatter_source_bundle() -> String {
    ["Token.ls", "AST.ls", "Lexer.ls", "Parser.ls", "Formatter.ls"]
        .into_iter()
        .map(selfhost_module)
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_formatter_source_harness(harness: &str) -> String {
    compile_and_run(&format!("{}\n{}", selfhost_formatter_source_bundle(), harness))
}

/// FMT-01: source-aware formatter が string literal を fallback せず再構成できること
#[test]
fn test_e2e_selfhost_formatter_format_program_with_source_string_literal() {
    let output = run_formatter_source_harness(
        r#"
(module Main)
(defn main []
  (let [src "\"abc\""
        program (parse-program src)]
    (do
      (print-string (format-program-with-source program src))
      0)))
"#,
    );

    assert_eq!(
        output,
        "\"abc\"\n",
        "format-program-with-source は string literal を source から復元するべき"
    );
}

/// FMT-01: source-aware formatter が float literal を fallback せず再構成できること
#[test]
fn test_e2e_selfhost_formatter_format_program_with_source_float_literal() {
    let output = run_formatter_source_harness(
        r#"
(module Main)
(defn main []
  (let [src "1.25"
        program (parse-program src)]
    (do
      (print-string (format-program-with-source program src))
      0)))
"#,
    );

    assert_eq!(
        output,
        "1.25\n",
        "format-program-with-source は float literal を source から復元するべき"
    );
}
