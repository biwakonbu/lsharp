#[path = "e2e/support.rs"]
mod support;

use support::{
    NATIVE_HARNESS_STACK_BYTES, compile_and_run, run_with_expanded_stack,
    selfhost_parser_typeinfer_runtime_bundle,
};

#[test]
fn property_probe_diagnostic_prints_parameter_and_return_shape() {
    let harness = r#"
(defn main []
  (let [payload "(for-all [value Int] :cases 1 :postcondition (= value 0))"
        expression "(= value 0)"
        parameter-source (property-probe-parameter-source payload)
        probe-source (string-concat "(defn __lsharp_property_probe " (string-concat parameter-source (string-concat " " (string-concat expression ")"))))
        program (parse-program probe-source)
        analysis (infer-program-analysis program)
        raw-type (infer-program-analysis-type analysis)
        resolved (property-probe-return-type raw-type)
        predicate (property-probe-predicate program)]
    (do
      (print-string parameter-source)
      (print-string "\n")
      (print-string probe-source)
      (print-string "\n")
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (ty-tag raw-type))
      (print (ty-tag resolved))
      (print (ty-name resolved))
      (print (vector-get predicate 0))
      (print (statically-boolean-result predicate))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec![
            "[(: value Int) result]",
            "(defn __lsharp_property_probe [(: value Int) result] (= value 0))",
            "0",
            "3",
            "1",
            "200",
            "5",
            "0",
        ],
        "property probe の source / inferred Bool / predicate AST が一致するべき"
    );
}

#[test]
fn property_probe_diagnostic_accepts_dynamic_complement_as_bool() {
    let harness = r#"
(defn main []
  (let [payload "(for-all [value Int] :cases 1 :postcondition (or (= value 0) (not (= value 0))))"
        expression "(or (= value 0) (not (= value 0)))"
        parameter-source (property-probe-parameter-source payload)
        probe-source (string-concat "(defn __lsharp_property_probe " (string-concat parameter-source (string-concat " " (string-concat expression ")"))))
        program (parse-program probe-source)
        analysis (infer-program-analysis program)
        raw-type (infer-program-analysis-type analysis)
        resolved (property-probe-return-type raw-type)
        predicate (property-probe-predicate program)
        check-code (check-property-predicate payload expression 1 0)]
    (do
      (print-string parameter-source)
      (print-string "\n")
      (print-string probe-source)
      (print-string "\n")
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (ty-tag raw-type))
      (print (ty-tag resolved))
      (print (ty-name resolved))
      (print (vector-get predicate 0))
      (print (statically-boolean-result predicate))
      (print check-code)
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec![
            "[(: value Int) result]",
            "(defn __lsharp_property_probe [(: value Int) result] (or (= value 0) (not (= value 0))))",
            "0",
            "3",
            "1",
            "200",
            "5",
            "1",
            "2005",
        ],
        "dynamic complement property probe は Bool として解析され、vacuous 判定だけを返すべき"
    );
}

#[test]
fn parsed_dynamic_property_reaches_canonical_checker_as_vacuous() {
    let harness = r#"
(defn main []
  (let [source "(defn identity [x] :property [(for-all [value Int] :postcondition (or (= value 0) (not (= value 0))))] x)"
        program (parse-program source)
        decl (vector-get program 0)
        metadata (defn-metadata decl)
        forms (defn-ordered-forms decl)
        form (vector-get forms 0)
        analysis (infer-program-analysis program)
        result (check-canonical-properties-with-analysis program analysis)]
    (do
      (print (vector-length program))
      (print (vector-length metadata))
      (print (vector-length forms))
      (print (vector-get form 0))
      (print-string (vector-get form 1))
      (print-string "\n")
      (print (vector-get result 0))
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec![
            "1",
            "6",
            "1",
            "5",
            "(for-all [value Int] :postcondition (or (= value 0) (not (= value 0))))",
            "1",
            "2005",
        ],
        "parser metadata の dynamic property は canonical checker まで同じ payload を届けるべき"
    );
}
