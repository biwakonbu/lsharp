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
