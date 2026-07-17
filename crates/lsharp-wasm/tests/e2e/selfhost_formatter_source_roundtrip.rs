use super::support::*;

fn selfhost_formatter_source_bundle() -> String {
    [
        "Token.ls",
        "AST.ls",
        "Lexer.ls",
        "Parser.ls",
        "FormatterExpr.ls",
        "FormatterDecl.ls",
        "Formatter.ls",
    ]
    .into_iter()
    .map(selfhost_module)
    .collect::<Vec<_>>()
    .join("\n")
}

fn run_formatter_source_harness(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_formatter_source_bundle(),
        harness
    ))
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
        output, "\"abc\"\n",
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
        output, "1.25\n",
        "format-program-with-source は float literal を source から復元するべき"
    );
}

/// FMT-01: source-aware formatter が defn metadata を canonical 順で保持できること
#[test]
fn test_e2e_selfhost_formatter_format_program_with_source_defn_metadata() {
    let output = run_formatter_source_harness(
        r#"
(module Main)
(defn main []
  (let [src "(defn add [x y] :doc \"Add two ints\" :params [(x \"left\") (y \"right\")] :returns \"sum\" :example [(add 1 2)] (+ x y))"
        program (parse-program src)]
    (do
      (print-string (format-program-with-source program src))
      0)))
"#,
    );

    assert_eq!(
        output,
        "(defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y))\n",
        "format-program-with-source は defn metadata を canonical 順で保持するべき"
    );
}

/// EC-M1-02: formatter metadata accessor が typed defn の signature を飛ばすこと
#[test]
fn test_e2e_selfhost_formatter_extracts_typed_defn_metadata() {
    let output = run_formatter_source_harness(
        r#"
(module Main)
(defn main []
  (let [program (parse-program "(defn add [(: x Int)] : Int :doc \"typed\" :example [(add 1)] (+ x 1))")
        decl (vector-get program 0)
        meta (extract-defn-metadata decl)
        has-metadata (if (= (vector-length meta) 5) 1 0)
        has-doc (if (= has-metadata 1)
          (if (string-eq (vector-get meta 0) "typed") 1 0)
          0)
        has-example (if (= has-metadata 1)
          (if (string-eq (vector-get meta 1) "(add 1)") 1 0)
          0)]
    (do
      (print has-metadata)
      (print has-doc)
      (print has-example)
      0)))
"#,
    );

    assert_eq!(
        output.trim().lines().collect::<Vec<_>>(),
        vec!["1", "1", "1"],
        "formatter metadata accessor は typed defn の signature 後ろの metadata を返すべき"
    );
}

/// EC-M1-02: source-aware formatter が legacy :invariant metadata を保持できること
#[test]
fn test_e2e_selfhost_formatter_format_program_with_source_invariant_metadata() {
    let output = run_formatter_source_harness(
        r#"
(module Main)
(defn main []
  (let [src "(defn succ [x] :invariant (= result (+ x 1)) :doc \"successor\" (+ x 1))"
        program (parse-program src)]
    (do
      (print-string (format-program-with-source program src))
      0)))
"#,
    );

    assert_eq!(
        output, "(defn succ [x] :doc \"successor\" :invariant (= result (+ x 1)) (+ x 1))\n",
        "format-program-with-source は legacy :invariant metadata を保持するべき"
    );
}
