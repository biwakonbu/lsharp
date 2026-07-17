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
        has-metadata (if (= (vector-length meta) 6) 1 0)
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

/// EC-M1-03: canonical :assert は selfhost parser / formatter 間で grouping と順序を保つ。
#[test]
fn test_e2e_selfhost_formatter_roundtrips_canonical_assert_form() {
    let output = run_formatter_source_harness(
        r#"
(module Main)
(defn main []
  (let [src "(defn positive [] :assert [(> 1 0) (= 1 1)] true)"
        program (parse-program src)
        formatted (format-program-with-source program src)
        canonical (format-program program 0)
        roundtrip (parse-program formatted)
        decl (vector-get roundtrip 0)
        meta (extract-defn-metadata decl)
        forms (vector-get meta 5)
        form (vector-get forms 0)
        predicates (vector-get form 1)]
    (do
      (print-string formatted)
      (print-string canonical)
      (print (vector-length forms))
      (print (vector-get form 0))
      (print (vector-length predicates))
      0)))
"#,
    );

    assert_eq!(
        output.lines().map(str::to_owned).collect::<Vec<_>>(),
        vec![
            "(defn positive [] :assert [(> 1 0) (= 1 1)] true)".to_owned(),
            "(defn positive [] :assert [(> 1 0) (= 1 1)] true)".to_owned(),
            "1".to_owned(),
            "3".to_owned(),
            "2".to_owned(),
        ],
        "canonical :assert は kind・grouping・formatter roundtrip を保持するべき"
    );
}

/// EC-M1-03: canonical :case は source-aware formatter で落とさず roundtrip する。
#[test]
fn test_e2e_selfhost_formatter_roundtrips_canonical_case_form() {
    let output = run_formatter_source_harness(
        r#"
(module Main)
(defn main []
  (let [src "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))"
        program (parse-program src)
        formatted (format-program-with-source program src)
        canonical (format-program program 0)
        roundtrip (parse-program formatted)
        decl (vector-get roundtrip 0)
        meta (extract-defn-metadata decl)
        forms (vector-get meta 5)
        form (vector-get forms 0)
        expectations (vector-get form 1)
        first-expectation (vector-get expectations 0)
        actual (vector-get first-expectation 0)
        expected (vector-get first-expectation 1)]
    (do
      (print-string formatted)
      (print-string canonical)
      (print (vector-get form 0))
      (print (vector-length expectations))
      (print (vector-get actual 0))
      (print (vector-get expected 0))
      (print (vector-get expected 1))
      0)))
"#,
    );

    assert_eq!(
        output.lines().map(str::to_owned).collect::<Vec<_>>(),
        vec![
            "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))".to_owned(),
            "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))".to_owned(),
            "4".to_owned(),
            "2".to_owned(),
            "5".to_owned(),
            "1".to_owned(),
            "2".to_owned(),
        ],
        "canonical :case は kind・expectation 順序・formatter roundtrip を保持するべき"
    );
}

/// EC-M1-03: formatter は複数の ordered contract form を順序どおり保持する。
#[test]
fn test_e2e_selfhost_formatter_preserves_multiple_ordered_contract_forms() {
    let output = run_formatter_source_harness(
        r#"
(module Main)
(defn main []
  (let [src "(defn law [x] :example [(law 0)] :invariant (> x 0) :case [(expect (law 0) 1)] :assert [(= 1 1)] :case [(expect (law 1) 2)] :example [(law 1) (law 2)] (+ x 1))"
        program (parse-program src)
        canonical (format-program program 0)]
    (do
      (print-string (format-program-with-source program src))
      (print-string canonical)
      0)))
"#,
    );

    assert_eq!(
        output,
        "(defn law [x] :example [(law 0)] :invariant (> x 0) :case [(expect (law 0) 1)] :assert [(= 1 1)] :case [(expect (law 1) 2)] :example [(law 1) (law 2)] (+ x 1))\n(defn law [x] :example [(law 0)] :invariant (> x 0) :case [(expect (law 0) 1)] :assert [(= 1 1)] :case [(expect (law 1) 2)] :example [(law 1) (law 2)] (+ x 1))\n",
        "formatter は複数の canonical contract form の順序と payload を保持するべき"
    );
}
