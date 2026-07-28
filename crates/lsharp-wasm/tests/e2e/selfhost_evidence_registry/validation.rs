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

/// EC-M2-02: source evidence の required runner は空値のまま registry へ登録しない。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_empty_runner() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/empty-runner\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"\" :target \"aarch64-apple-darwin\" :source-commit \"source-empty-runner-commit\" :artifact-digest \"sha256:source-empty-runner\" :cases 1 :seed 0 :generator \"source-empty-runner-generator\" :producer \"source-empty-runner-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
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
        ["0", "4", "runner"],
        "source evidence の empty runner は required-field error として拒否するべき"
    );
}

/// EC-M2-02: source evidence の required target は空値のまま registry へ登録しない。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_empty_target() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/empty-target\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"source-empty-target-runner\" :target \"\" :source-commit \"source-empty-target-commit\" :artifact-digest \"sha256:source-empty-target\" :cases 1 :seed 0 :generator \"source-empty-target-generator\" :producer \"source-empty-target-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
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
        ["0", "4", "target"],
        "source evidence の empty target は required-field error として拒否するべき"
    );
}

/// EC-M2-02: source evidence の required source commit は空値のまま registry へ登録しない。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_empty_source_commit() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/empty-source-commit\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"source-empty-source-commit-runner\" :target \"aarch64-apple-darwin\" :source-commit \"\" :artifact-digest \"sha256:source-empty-source-commit\" :cases 1 :seed 0 :generator \"source-empty-source-commit-generator\" :producer \"source-empty-source-commit-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
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
        ["0", "4", "source-commit"],
        "source evidence の empty source commit は required-field error として拒否するべき"
    );
}

/// EC-M2-02: source evidence の required artifact digest は空値のまま registry へ登録しない。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_empty_artifact_digest() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/empty-artifact-digest\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"source-empty-artifact-digest-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-empty-artifact-digest-commit\" :artifact-digest \"\" :cases 1 :seed 0 :generator \"source-empty-artifact-digest-generator\" :producer \"source-empty-artifact-digest-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
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
        ["0", "4", "artifact-digest"],
        "source evidence の empty artifact digest は required-field error として拒否するべき"
    );
}

/// EC-M2-02: source evidence の required producer は空値のまま registry へ登録しない。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_empty_producer() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/empty-producer\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"source-empty-producer-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-empty-producer-commit\" :artifact-digest \"sha256:source-empty-producer\" :cases 1 :seed 0 :generator \"source-empty-producer-generator\" :producer \"\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
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
        ["0", "4", "producer"],
        "source evidence の empty producer は required-field error として拒否するべき"
    );
}

/// EC-M2-02: source evidence の required tool version は空値のまま registry へ登録しない。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_empty_tool_version() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/empty-tool-version\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"source-empty-tool-version-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-empty-tool-version-commit\" :artifact-digest \"sha256:source-empty-tool-version\" :cases 1 :seed 0 :generator \"source-empty-tool-version-generator\" :producer \"source-empty-tool-version-producer\" :tool-version \"\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
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
        ["0", "4", "tool-version"],
        "source evidence の empty tool version は required-field error として拒否するべき"
    );
}

/// EC-M2-02: source evidence の required timestamp は空値のまま registry へ登録しない。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_empty_timestamp() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/empty-timestamp\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"source-empty-timestamp-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-empty-timestamp-commit\" :artifact-digest \"sha256:source-empty-timestamp\" :cases 1 :seed 0 :generator \"source-empty-timestamp-generator\" :producer \"source-empty-timestamp-producer\" :tool-version \"0.2.0-dev\" :timestamp \"\" :independence \"same-author\" true)"))
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
        ["0", "4", "timestamp"],
        "source evidence の empty timestamp は required-field error として拒否するべき"
    );
}

/// EC-M2-02: empty evidence method は enum typed-field code 8 として拒否する。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_empty_method_as_typed_field_error() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/empty-method\" :subject \"claim:checkout/cancel\" :method \"\" :outcome \"pass\" :runner \"empty-method-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-empty-method\" :artifact-digest \"sha256:empty-method\" :cases 1 :seed 0 :generator \"empty-method-generator\" :producer \"empty-method-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print-string "[")
      (print-string (source-evidence-error-value error))
      (print-string "]\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "8", "method", "[]"],
        "empty evidence method は enum typed-field code 8 として拒否するべき"
    );
}

/// EC-M2-02: empty evidence outcome は enum typed-field code 8 として拒否する。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_empty_outcome_as_typed_field_error() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/empty-outcome\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"\" :runner \"empty-outcome-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-empty-outcome\" :artifact-digest \"sha256:empty-outcome\" :cases 1 :seed 0 :generator \"empty-outcome-generator\" :producer \"empty-outcome-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print-string "[")
      (print-string (source-evidence-error-value error))
      (print-string "]\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "8", "outcome", "[]"],
        "empty evidence outcome は enum typed-field code 8 として拒否するべき"
    );
}

/// EC-M2-02: empty evidence independence は enum typed-field code 8 として拒否する。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_empty_independence_as_typed_field_error() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/empty-independence\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"empty-independence-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-empty-independence\" :artifact-digest \"sha256:empty-independence\" :cases 1 :seed 0 :generator \"empty-independence-generator\" :producer \"empty-independence-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print-string "[")
      (print-string (source-evidence-error-value error))
      (print-string "]\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "8", "independence", "[]"],
        "empty evidence independence は enum typed-field code 8 として拒否するべき"
    );
}

/// EC-M2-02: source evidence の whitespace-only runner は空値と同じく registry へ登録しない。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_whitespace_only_runner() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/whitespace-runner\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"  \" :target \"aarch64-apple-darwin\" :source-commit \"source-whitespace-evidence-runner\" :artifact-digest \"sha256:whitespace-evidence-runner\" :cases 1 :seed 0 :generator \"whitespace-evidence-runner-generator\" :producer \"whitespace-evidence-runner-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
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
        ["0", "4", "runner"],
        "source evidence の whitespace-only runner は required-field error として拒否するべき"
    );
}

/// EC-M2-02: whitespace-only subject は required-field ではなく invalid stable ID として拒否する。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_whitespace_only_subject_as_invalid_id() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/whitespace-subject\" :subject \"  \" :method \"case\" :outcome \"pass\" :runner \"whitespace-subject-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-whitespace-subject\" :artifact-digest \"sha256:whitespace-subject\" :cases 1 :seed 0 :generator \"whitespace-subject-generator\" :producer \"whitespace-subject-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print-string "[")
      (print-string (source-evidence-error-value error))
      (print-string "]\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "2", "subject", "[  ]"],
        "source evidence の whitespace-only subject は invalid stable ID として拒否するべき"
    );
}

/// EC-M2-02: whitespace-only evidence ID は required-field ではなく invalid stable ID として拒否する。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_whitespace_only_id_as_invalid_id() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"  \" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"whitespace-id-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-whitespace-id\" :artifact-digest \"sha256:whitespace-id\" :cases 1 :seed 0 :generator \"whitespace-id-generator\" :producer \"whitespace-id-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print-string "[")
      (print-string (source-evidence-error-value error))
      (print-string "]\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "2", "id", "[  ]"],
        "source evidence の whitespace-only ID は invalid stable ID として拒否するべき"
    );
}

/// EC-M2-02: empty evidence ID は required-field ではなく invalid stable ID として拒否する。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_empty_id_as_invalid_id() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"empty-id-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-empty-id\" :artifact-digest \"sha256:empty-id\" :cases 1 :seed 0 :generator \"empty-id-generator\" :producer \"empty-id-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print-string "[")
      (print-string (source-evidence-error-value error))
      (print-string "]\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "2", "id", "[]"],
        "source evidence の empty ID は invalid stable ID として拒否するべき"
    );
}

/// EC-M2-02: invalid evidence method は source validation の typed-field code 8 に揃える。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_invalid_method_as_typed_field_error() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/invalid-method\" :subject \"claim:checkout/cancel\" :method \"not-a-method\" :outcome \"pass\" :runner \"invalid-method-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-invalid-method\" :artifact-digest \"sha256:invalid-method\" :cases 1 :seed 0 :generator \"invalid-method-generator\" :producer \"invalid-method-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print-string "[")
      (print-string (source-evidence-error-value error))
      (print-string "]\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "8", "method", "[not-a-method]"],
        "invalid evidence method は typed-field code 8 と値を保持して拒否するべき"
    );
}

/// EC-M2-02: invalid evidence independence は source validation の typed-field code 8 に揃える。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_invalid_independence_as_typed_field_error() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/invalid-independence\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"pass\" :runner \"invalid-independence-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-invalid-independence\" :artifact-digest \"sha256:invalid-independence\" :cases 1 :seed 0 :generator \"invalid-independence-generator\" :producer \"invalid-independence-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"not-an-independence\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print-string "[")
      (print-string (source-evidence-error-value error))
      (print-string "]\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "8", "independence", "[not-an-independence]"],
        "invalid evidence independence は typed-field code 8 と値を保持して拒否するべき"
    );
}

/// EC-M2-02: unsupported evidence subject kind は source validation の typed-field code 8 に揃える。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_invalid_subject_as_typed_field_error() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/invalid-subject\" :subject \"evidence:checkout/wrong-kind\" :method \"case\" :outcome \"pass\" :runner \"invalid-subject-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-invalid-subject\" :artifact-digest \"sha256:invalid-subject\" :cases 1 :seed 0 :generator \"invalid-subject-generator\" :producer \"invalid-subject-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print-string "[")
      (print-string (source-evidence-error-value error))
      (print-string "]\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "8", "subject", "[evidence:checkout/wrong-kind]"],
        "invalid evidence subject kind は typed-field code 8 と値を保持して拒否するべき"
    );
}

/// EC-M2-02: invalid evidence outcome は source validation の typed-field code 8 に揃える。
#[test]
fn test_e2e_selfhost_source_evidence_rejects_invalid_outcome_as_typed_field_error() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn cancel [] :claim \"claim:checkout/cancel\" \"The API rejects shipped orders\" :evidence \"evidence:checkout/invalid-outcome\" :subject \"claim:checkout/cancel\" :method \"case\" :outcome \"not-an-outcome\" :runner \"invalid-outcome-runner\" :target \"aarch64-apple-darwin\" :source-commit \"source-invalid-outcome\" :artifact-digest \"sha256:invalid-outcome\" :cases 1 :seed 0 :generator \"invalid-outcome-generator\" :producer \"invalid-outcome-producer\" :tool-version \"0.2.0-dev\" :timestamp \"2026-07-28T00:00:00Z\" :independence \"same-author\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print-string "[")
      (print-string (source-evidence-error-value error))
      (print-string "]\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "8", "outcome", "[not-an-outcome]"],
        "invalid evidence outcome は typed-field code 8 と値を保持して拒否するべき"
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

/// EC-M2-02: whitespace-only coverage bucket は empty-field code 4 として拒否する。
#[test]
fn test_e2e_selfhost_evidence_registry_rejects_whitespace_only_coverage_bucket() {
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
        coverage-entry (vector-push-pair-rooted-v3 (vector-new 0) "  " 1)
        coverage (vector-push-single-rooted-v3 (vector-new 0) coverage-entry)
        payload (source-evidence-payload
          "evidence:checkout/whitespace-coverage"
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
      (print-string "[")
      (print-string (source-evidence-error-value error))
      (print-string "]\n")
      (print (source-evidence-error-start error))
      (print (source-evidence-error-end error))
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "4", "coverage", "[  ]", "10", "20"],
        "whitespace-only coverage bucket は empty-field code 4 と raw value/span で拒否するべき"
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
        ["0", "11", "coverage", "", "10", "20"],
        "malformed coverage entry は invalid-sampling と directive span 付きで fail-closed にするべき"
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
