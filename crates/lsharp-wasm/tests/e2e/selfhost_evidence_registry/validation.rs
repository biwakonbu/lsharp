//! malformed sampling、payload、duplicate registry の fail-closed tests。

use super::harness::run_evidence_registry_runtime;

/// EC-M2-02: source evidence の required generator は空値のまま registry へ登録しない。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_empty_generator() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/empty-generator\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"source-empty-generator\" :target \"aarch64-apple-darwin\" :source-commit \"source-empty-generator-commit\" :artifact-digest \"sha256:source-empty-generator\" :cases 1 :seed 0 :generator \"\" :producer \"source-empty-generator-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "4", "generator"],
        "source evidence の empty generator は required-field error として拒否するべき"
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

/// EC-M2-02: coverage entry は bucket/count の2要素だけを受理する。
#[test]
fn test_e2e_selfhost_evidence_registry_rejects_malformed_coverage_entry() {
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
        coverage-entry (vector-push-triple-rooted-v3 (vector-new 0) "smoke" 1 99)
        coverage (vector-push-single-rooted-v3 (vector-new 0) coverage-entry)
        payload (source-evidence-payload
          "evidence:checkout/malformed-coverage"
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
      (print (source-evidence-error-start error))
      (print (source-evidence-error-end error))
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "11", "coverage", "", "-1", "-1"],
        "malformed coverage entry は invalid-sampling として fail-closed にするべき"
    );
}

/// EC-M2-02: evidence payload は canonical 17-field shape だけを受理する。
#[test]
fn test_e2e_selfhost_evidence_registry_rejects_extra_payload_field() {
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
        base-payload (source-evidence-payload
          "evidence:checkout/extra-field"
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
          (vector-new 0)
          "producer"
          "0.2"
          "2026-07-25T00:00:00Z"
          "same-author")
        payload (vector-push-single-rooted-v3 base-payload "unexpected")
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
      (print (source-evidence-error-start error))
      (print (source-evidence-error-end error))
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "form", "", "10", "20"],
        "extra payload field は malformed form として fail-closed にするべき"
    );
}

/// EC-M2-02: 負の shrink 値は canonical sampling と同じ fail-closed code で拒否する。
#[test]
fn test_e2e_selfhost_evidence_registry_rejects_negative_shrink() {
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
        shrinks (vector-push-single-rooted-v3 (vector-new 0) (- 0 1))
        payload (source-evidence-payload
          "evidence:checkout/negative-shrink"
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
          shrinks
          (vector-new 0)
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
      (print (source-evidence-error-start error))
      (print (source-evidence-error-end error))
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "11", "shrinks", "", "10", "20"],
        "負の shrink 値は invalid-sampling error として span 付きで拒否するべき"
    );
}

/// EC-M2-02: seed は canonical `u64` sampling と同じ fail-closed code で拒否する。
#[test]
fn test_e2e_selfhost_evidence_registry_rejects_negative_seed() {
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
          "evidence:checkout/negative-seed"
          "claim:checkout/rejects"
          "property"
          "pass"
          "runner"
          "aarch64-apple-darwin"
          "deadbeef"
          "sha256:abc"
          1
          (- 0 1)
          "generator"
          (vector-new 0)
          (vector-new 0)
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
      (print (source-evidence-error-start error))
      (print (source-evidence-error-end error))
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "11", "seed", "", "10", "20"],
        "負の seed は invalid-sampling error として span 付きで拒否するべき"
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
