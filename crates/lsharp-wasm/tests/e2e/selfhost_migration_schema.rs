use super::support::*;

fn run_migration_schema_runtime(harness: &str) -> String {
    let combined = format!("{}\n{}", selfhost_migration_runtime_bundle(), harness);
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    })
}

/// EC-M1-03: selfhost migration row の enum wire value を canonical schema として検証する。
#[test]
fn test_e2e_selfhost_migration_row_schema_rejects_unknown_enum_values() {
    let harness = r#"
(defn main []
  (let [valid (legacy-migration-row-with-expression-span
                (legacy-example-code)
                (legacy-doc-example-disposition)
                10
                20
                30
                ""
                10
                20)
        invalid-code (legacy-migration-row-with-expression-span
                       9999
                       (legacy-doc-example-disposition)
                       10
                       20
                       30
                       ""
                       10
                       20)
        invalid-disposition (legacy-migration-row-with-expression-span
                              (legacy-example-code)
                              99
                              10
                              20
                              30
                              ""
                              10
                              20)
        base6 (vector-push-single-rooted
                (vector-push-quad-rooted
                  (vector-new 4)
                  (legacy-example-code)
                  (legacy-doc-example-disposition)
                  10
                  20)
                30)
        base6-with-message (vector-push-single-rooted base6 "")
        invalid-selected (vector-push-pair-rooted
                           (vector-push-single-rooted base6-with-message 99)
                           10
                           20)
        valid-text (legacy-migration-row-detail-text valid)
        invalid-text (legacy-migration-row-detail-text invalid-code)
        invalid-json (legacy-migration-row-detail-json invalid-disposition)
        invalid-summary (legacy-migration-row-text invalid-selected)]
    (do
      (print (legacy-migration-row-schema-valid? valid))
      (print (legacy-migration-row-schema-valid? invalid-code))
      (print (legacy-migration-row-schema-valid? invalid-disposition))
      (print (legacy-migration-row-schema-valid? invalid-selected))
      (print (if (> (string-length valid-text) 0) 1 0))
      (print (string-length invalid-text))
      (print (string-length invalid-json))
      (print (string-length invalid-summary))
      0)))
"#;

    let output = run_migration_schema_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "0", "0", "0", "1", "0", "0", "0"],
        "selfhost migration projection は未知の enum を既定値へ丸めず fail-closed にするべき"
    );
}
