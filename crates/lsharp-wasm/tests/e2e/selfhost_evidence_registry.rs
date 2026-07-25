use super::support::*;

fn run_evidence_registry_runtime(harness: &str) -> String {
    let intent_source = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/IntentSource.ls"),
    )
    .expect("canonical IntentSource.ls が読み込めない");
    let evidence = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/Evidence.ls"),
    )
    .expect("canonical Evidence.ls が読み込めない");
    compile_and_run(&format!(
        "{}\n{}\n{}\n{}",
        selfhost_parser_runtime_bundle(),
        intent_source,
        evidence,
        harness
    ))
}

/// EC-M2-02: required evidence record と supports edge を selfhost registry へ投影する。
#[test]
fn test_e2e_selfhost_evidence_registry_registers_record_and_edge() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn verify [] :intent \"intent:checkout/cancel\" \"Users can cancel\" :claim \"claim:checkout/rejects\" \"Shipped orders are rejected\" true)")
        nodes (source-result-value (source-collect-nodes program))
        shrinks (vector-push-pair-rooted-v3 (vector-new 0) 1 2)
        coverage-entry (vector-push-pair-rooted-v3 (vector-new 0) "smoke" 3)
        coverage (vector-push-single-rooted-v3 (vector-new 0) coverage-entry)
        payload (source-evidence-payload
          "evidence:checkout/verified"
          "claim:checkout/rejects"
          "case"
          "pass"
          "cargo-test"
          "aarch64-apple-darwin"
          "deadbeef"
          "sha256:abc"
          3
          42
          "checkout-generator"
          shrinks
          coverage
          "lsharp-test"
          "0.2"
          "2026-07-25T00:00:00Z"
          "same-author")
        form (source-evidence-form payload 200 260)
        registered (source-evidence-register-form (source-evidence-registry-new) nodes form)
        registry (source-result-value registered)
        evidence-record (vector-get registry 0)
        edge-result (source-evidence-edge-result
          (source-edge-supports)
          "evidence:checkout/verified"
          "claim:checkout/rejects"
          registry
          nodes
          300
          350)
        edge (source-result-value edge-result)]
    (do
      (print (source-result-status registered))
      (print (vector-length registry))
      (print-string (source-evidence-record-id evidence-record))
      (print-string "\n")
      (print-string (source-evidence-record-subject evidence-record))
      (print-string "\n")
      (print-string (source-evidence-record-method evidence-record))
      (print-string "\n")
      (print-string (source-evidence-record-outcome evidence-record))
      (print-string "\n")
      (print (source-evidence-record-cases evidence-record))
      (print (source-evidence-record-seed evidence-record))
      (print (vector-length (source-evidence-record-shrinks evidence-record)))
      (print (vector-length (source-evidence-record-coverage evidence-record)))
      (print (source-evidence-record-start evidence-record))
      (print (source-evidence-record-end evidence-record))
      (print (source-result-status edge-result))
      (print (source-edge-kind edge))
      (print-string (source-edge-left edge))
      (print-string "\n")
      (print-string (source-edge-right edge))
      (print-string "\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "1",
            "1",
            "evidence:checkout/verified",
            "claim:checkout/rejects",
            "case",
            "pass",
            "3",
            "42",
            "2",
            "1",
            "200",
            "260",
            "1",
            "13",
            "evidence:checkout/verified",
            "claim:checkout/rejects",
        ],
        "selfhost evidence registry は required fields と registered supports edge を保持するべき"
    );
}

/// EC-M2-02: required field 欠落は evidence registry に登録しない。
#[test]
fn test_e2e_selfhost_evidence_registry_rejects_empty_required_field() {
    let harness = r#"
(defn main []
  (let [payload (source-evidence-payload
          "evidence:checkout/invalid"
          "claim:checkout/rejects"
          "case"
          "pass"
          ""
          "aarch64-apple-darwin"
          "deadbeef"
          "sha256:abc"
          1
          0
          "generator"
          (vector-new 0)
          (vector-new 0)
          "producer"
          "0.2"
          "2026-07-25T00:00:00Z"
          "same-author")
        result (source-evidence-register-form
          (source-evidence-registry-new)
          (vector-new 0)
          (source-evidence-form payload 50 90))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print-string (source-evidence-error-value error))
      (print-string "\n")
      (print (source-evidence-error-start error))
      (print (source-evidence-error-end error))
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "4", "runner", "", "50", "90"],
        "evidence の required field 欠落は span 付きで fail-closed にするべき"
    );
}

/// EC-M2-02: coverage bucket の重複は deterministic sampling plan を壊すため拒否する。
#[test]
fn test_e2e_selfhost_evidence_registry_rejects_duplicate_coverage_bucket() {
    let harness = r#"
(defn main []
  (let [nodes (vector-push-single-rooted-v3
                (vector-new 0)
                (source-node-record
                  (source-node-claim)
                  "claim:checkout/rejects"
                  "rejects shipped orders"
                  1
                  2))
        first (vector-push-pair-rooted-v3 (vector-new 0) "smoke" 1)
        second (vector-push-pair-rooted-v3 (vector-new 0) "smoke" 2)
        coverage (vector-push-pair-rooted-v3 (vector-new 0) first second)
        payload (source-evidence-payload
          "evidence:checkout/coverage"
          "claim:checkout/rejects"
          "property"
          "pass"
          "runner"
          "aarch64-apple-darwin"
          "deadbeef"
          "sha256:abc"
          1
          0
          "generator"
          (vector-new 0)
          coverage
          "producer"
          "0.2"
          "2026-07-25T00:00:00Z"
          "same-author")
        result (source-evidence-register-form
          (source-evidence-registry-new)
          nodes
          (source-evidence-form payload 10 20))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print-string (source-evidence-error-value error))
      (print-string "\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "10", "coverage", "smoke"],
        "coverage bucket の重複は evidence registry で拒否するべき"
    );
}

/// EC-M2-02: evidence ID の重複は first/current span を保持して拒否する。
#[test]
fn test_e2e_selfhost_evidence_registry_rejects_duplicate_id() {
    let harness = r#"
(defn main []
  (let [nodes (vector-push-single-rooted-v3
                (vector-new 0)
                (source-node-record
                  (source-node-claim)
                  "claim:checkout/rejects"
                  "rejects shipped orders"
                  1
                  2))
        payload (source-evidence-payload
          "evidence:checkout/duplicate"
          "claim:checkout/rejects"
          "case"
          "pass"
          "runner"
          "aarch64-apple-darwin"
          "deadbeef"
          "sha256:abc"
          1
          0
          "generator"
          (vector-new 0)
          (vector-new 0)
          "producer"
          "0.2"
          "2026-07-25T00:00:00Z"
          "same-author")
        first (source-evidence-form payload 20 30)
        second (source-evidence-form payload 40 50)
        first-result (source-evidence-register-form
          (source-evidence-registry-new)
          nodes
          first)
        result (source-evidence-register-form
          (source-result-value first-result)
          nodes
          second)
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print (source-evidence-error-start error))
      (print (source-evidence-error-end error))
      (print (source-evidence-error-related-start error))
      (print (source-evidence-error-related-end error))
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "3", "40", "50", "20", "30"],
        "evidence ID の重複は current/first span を返すべき"
    );
}
