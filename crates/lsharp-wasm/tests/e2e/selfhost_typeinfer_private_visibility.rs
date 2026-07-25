use super::support::*;

/// EC-M1-01: private record は宣言元 module の型推論では可視であること
#[test]
fn test_e2e_selfhost_typeinfer_analysis_accepts_private_record_in_same_module() {
    let valid_source =
        "(module Lib) (private (type Secret (record (: x Int)))) (defn reveal [] {Secret x 1})";
    let invalid_source =
        "(module Lib) (private (type Secret (record (: x Int)))) (defn reveal [] {Secret x true})";
    let leaked_source = "(module Lib) (private (type Secret (record (: x Int)))) (module Main) (defn main [] {Secret x 1})";
    let pattern_source = "(module Lib) (private (type Secret (record (: x Int)))) (defn reveal [point] : Int (match point [{Secret x x} x] [_ 0]))";
    let leaked_pattern_source = "(module Lib) (private (type Secret (record (: x Int)))) (module Main) (defn reveal [point] : Int (match point [{Secret x x} x] [_ 0]))";
    // Rust oracle は private wrapper 内の record schema をまだ登録しないため、
    // 同じ local record literal の公開相当 fixture で構文・型の契約を照合する。
    let oracle_source =
        "(module Lib) (type Secret (record (: x Int))) (defn reveal [] {Secret x 1})";
    let program =
        lsharp_syntax::parse(oracle_source).expect("private record oracle は parse できるべき");
    let mut oracle = lsharp_types::infer::Infer::new();
    let oracle_result = oracle.infer_program(&program);
    assert!(
        oracle_result.is_ok(),
        "Rust oracle は宣言元 module の private record literal を受理するべき: {:?}",
        oracle_result.err()
    );

    let invalid_oracle_source =
        "(module Lib) (type Secret (record (: x Int))) (defn reveal [] {Secret x true})";
    let invalid_oracle_program = lsharp_syntax::parse(invalid_oracle_source)
        .expect("private record mismatch oracle は parse できるべき");
    let mut invalid_oracle = lsharp_types::infer::Infer::new();
    assert!(
        invalid_oracle
            .infer_program(&invalid_oracle_program)
            .is_err(),
        "Rust oracle は private record の field 型不一致を拒否するべき"
    );
    let pattern_oracle_source = "(module Lib) (type Secret (record (: x Int))) (defn reveal [point] : Int (match point [{Secret x x} x] [_ 0]))";
    let pattern_oracle_program = lsharp_syntax::parse(pattern_oracle_source)
        .expect("private record pattern oracle は parse できるべき");
    let mut pattern_oracle = lsharp_types::infer::Infer::new();
    assert!(
        pattern_oracle
            .infer_program(&pattern_oracle_program)
            .is_ok(),
        "Rust oracle は local record pattern を受理するべき"
    );

    let harness = format!(
        r#"
(defn main []
  (let [valid
          (infer-program-analysis
            (parse-program "{}"))
        invalid
          (infer-program-analysis
            (parse-program "{}"))
        leaked
          (infer-program-analysis
            (parse-program "{}"))
        pattern
          (infer-program-analysis
            (parse-program "{}"))
        leaked-pattern
          (infer-program-analysis
            (parse-program "{}"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid))
      (print (infer-program-analysis-diagnostic-count invalid))
      (print (infer-program-analysis-diagnostic-count leaked))
      (print (infer-program-analysis-diagnostic-count pattern))
      (print (infer-program-analysis-diagnostic-count leaked-pattern))
      0)))
"#,
        valid_source, invalid_source, leaked_source, pattern_source, leaked_pattern_source
    );

    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "1", "0", "1"],
        "private record schema は宣言元 module だけで可視であるべき"
    );
}
