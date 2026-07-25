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
    let json_rpc =
        std::fs::read_to_string(selfhost_project_root().join("selfhost/src/Tools/Lsp/JsonRpc.ls"))
            .expect("canonical JsonRpc.ls が読み込めない");
    compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        selfhost_parser_runtime_bundle(),
        json_rpc,
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

/// EC-M2-02: parser の :evidence form を registry consumer へ直接渡す。
#[test]
fn test_e2e_selfhost_evidence_registry_consumes_parser_form() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn verify [] :intent \"intent:checkout/cancel\" \"Users can cancel\" :claim \"claim:checkout/rejects\" \"Shipped orders are rejected\" :evidence \"evidence:checkout/verified\" :subject \"claim:checkout/rejects\" :method \"case\" :outcome \"pass\" :runner \"cargo-test\" :target \"aarch64-apple-darwin\" :source-commit \"deadbeef\" :artifact-digest \"sha256:abc\" :cases 3 :seed 42 :generator \"checkout-generator\" :shrinks [8 3 1] :coverage [(\"smoke\" 3)] :producer \"lsharp-test\" :tool-version \"0.2\" :timestamp \"2026-07-25T00:00:00Z\" :independence \"same-author\" true)")
        result (source-evidence-registry-from-program program)
        registry (source-result-value result)
        evidence-record (vector-get registry 0)]
    (do
      (print (source-result-status result))
      (print (vector-length registry))
      (print-string (source-evidence-record-id evidence-record))
      (print-string "\n")
      (print-string (source-evidence-record-subject evidence-record))
      (print-string "\n")
      (print-string (source-evidence-record-method evidence-record))
      (print-string "\n")
      (print (source-evidence-record-cases evidence-record))
      (print (source-evidence-record-seed evidence-record))
      (print (vector-length (source-evidence-record-shrinks evidence-record)))
      (print (vector-length (source-evidence-record-coverage evidence-record)))
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
            "3",
            "42",
            "3",
            "1",
        ],
        "parser の evidence form は selfhost registry に登録されるべき"
    );
}

/// EC-M2-02: parser の evidence registry と supports/contradicts edge を同じ graph に投影する。
#[test]
fn test_e2e_selfhost_evidence_registry_wires_source_edges() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn verify [] :claim \"claim:checkout/rejects\" \"Shipped orders are rejected\" :evidence \"evidence:checkout/verified\" :subject \"claim:checkout/rejects\" :method \"case\" :outcome \"pass\" :runner \"cargo-test\" :target \"aarch64-apple-darwin\" :source-commit \"deadbeef\" :artifact-digest \"sha256:abc\" :cases 3 :seed 42 :generator \"checkout-generator\" :producer \"lsharp-test\" :tool-version \"0.2\" :timestamp \"2026-07-25T00:00:00Z\" :independence \"same-author\" :evidence \"evidence:checkout/counterexample\" :subject \"claim:checkout/rejects\" :method \"case\" :outcome \"fail\" :runner \"cargo-test\" :target \"aarch64-apple-darwin\" :source-commit \"deadbeef\" :artifact-digest \"sha256:def\" :cases 1 :seed 7 :generator \"checkout-generator\" :producer \"lsharp-test\" :tool-version \"0.2\" :timestamp \"2026-07-25T00:00:00Z\" :independence \"same-author\" :supports \"evidence:checkout/verified\" \"claim:checkout/rejects\" :contradicts \"evidence:checkout/counterexample\" \"claim:checkout/rejects\" true)")
        result (source-evidence-graph-from-program program)
        graph (source-result-value result)
        nodes (source-graph-nodes graph)
        edges (source-graph-edges graph)
        registry (source-evidence-graph-registry graph)
        supports (vector-get edges 0)
        contradicts (vector-get edges 1)]
    (do
      (print (source-result-status result))
      (print (vector-length nodes))
      (print (vector-length registry))
      (print (vector-length edges))
      (print (source-edge-kind supports))
      (print-string (source-edge-left supports))
      (print-string "\n")
      (print (source-edge-kind contradicts))
      (print-string (source-edge-left contradicts))
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
            "2",
            "2",
            "13",
            "evidence:checkout/verified",
            "14",
            "evidence:checkout/counterexample",
        ],
        "parser の evidence registry は supports/contradicts edge と同じ graph に接続されるべき"
    );
}

/// EC-M2-02: 新しい source graph 経路も未登録 evidence edge を fail-closed にする。
#[test]
fn test_e2e_selfhost_evidence_registry_rejects_unregistered_source_edge() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn counterexample [] :claim \"claim:checkout/rejects\" \"Shipped orders are rejected\" :supports \"evidence:checkout/missing\" \"claim:checkout/rejects\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      (print-string "\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "6", "evidence:checkout/missing"],
        "source graph 経路は未登録 evidence edge を明示的に拒否するべき"
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

/// EC-M2-03: source graph manifest serializer は Rust の version 1 wire shape を保持する。
#[test]
fn test_e2e_selfhost_evidence_manifest_serializer_matches_version_one_shape() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn verify [] :claim \"claim:checkout/rejects\" \"Shipped orders are rejected\" :tested-by \"claim:checkout/rejects\" \"contract:checkout/case\" :evidence \"evidence:checkout/verified\" :subject \"claim:checkout/rejects\" :method \"case\" :outcome \"pass\" :runner \"cargo-test\" :target \"aarch64-apple-darwin\" :source-commit \"deadbeef\" :artifact-digest \"sha256:abc\" :cases 3 :seed 42 :generator \"checkout-generator\" :shrinks [8 3 1] :coverage [(\"smoke\" 3)] :producer \"lsharp-test\" :tool-version \"0.2\" :timestamp \"2026-07-25T00:00:00Z\" :independence \"same-author\" :supports \"evidence:checkout/verified\" \"claim:checkout/rejects\" true)")
        result (source-evidence-graph-from-program program)
        graph (source-result-value result)]
    (do
      (print (source-result-status result))
      (print-string (validation-source-manifest-json graph))
      (print-string "\n")
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let mut lines = output.trim().lines();
    assert_eq!(lines.next(), Some("1"));
    let manifest: serde_json::Value =
        serde_json::from_str(lines.next().expect("manifest JSON が出力されるべき"))
            .expect("manifest JSON は parse 可能であるべき");

    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["nodes"][0]["kind"], "claim");
    assert_eq!(manifest["nodes"][0]["namespace"], "checkout");
    assert_eq!(manifest["nodes"][0]["key"], "rejects");
    assert_eq!(manifest["evidence"][0]["execution"]["sampling"]["cases"], 3);
    assert_eq!(
        manifest["evidence"][0]["execution"]["sampling"]["shrinks"],
        serde_json::json!([8, 3, 1])
    );
    assert_eq!(manifest["edges"][0]["relation"], "tested-by");
    assert_eq!(manifest["edges"][1]["relation"], "supports");
}

/// EC-M3-01: selfhost の manifest は Rust canonical serializer と同じ JSON value を返す。
#[test]
fn test_e2e_selfhost_evidence_manifest_matches_rust_canonical_value() {
    let source = r#"(defn verify [] :intent "intent:checkout/safe-cancel" "Users can cancel an order" :claim "claim:checkout/rejects" "Shipped orders are rejected" :assumption "assumption:checkout/state-authoritative" "Shipment state is authoritative" :open-question "open-question:checkout/after-label" "Can cancellation happen after a label?" :motivates "intent:checkout/safe-cancel" "claim:checkout/rejects" :constrained-by "claim:checkout/rejects" "assumption:checkout/state-authoritative" :tested-by "claim:checkout/rejects" "contract:checkout/case" :evidence "evidence:checkout/verified" :subject "claim:checkout/rejects" :method "case" :outcome "pass" :runner "cargo-test" :target "aarch64-apple-darwin" :source-commit "deadbeef" :artifact-digest "sha256:abc" :cases 3 :seed 42 :generator "checkout-generator" :shrinks [8 3 1] :coverage [("smoke" 3)] :producer "lsharp-test" :tool-version "0.2" :timestamp "2026-07-25T00:00:00Z" :independence "same-author" :supports "evidence:checkout/verified" "claim:checkout/rejects" true)"#;
    let program = lsharp_syntax::parse(source).expect("Rust oracle source は parse できるべき");
    let graph = lsharp_types::validation_source::source_program_to_intent_graph(&program)
        .expect("Rust oracle source graph は構築できるべき");
    let expected = graph.to_manifest_json_value();
    let escaped_source = source.replace('\\', "\\\\").replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [program (parse-program "{escaped_source}")
        result (source-evidence-graph-from-program program)
        graph (source-result-value result)]
    (do
      (print (source-result-status result))
      (print-string (validation-source-manifest-json graph))
      (print-string "\n")
      0)))
"#
    );

    let output = run_evidence_registry_runtime(&harness);
    let mut lines = output.trim().lines();
    assert_eq!(lines.next(), Some("1"));
    let actual_json = lines
        .next()
        .expect("selfhost manifest JSON が出力されるべき");
    let actual: serde_json::Value =
        serde_json::from_str(actual_json).expect("selfhost manifest JSON は parse 可能であるべき");
    let expected_json = graph
        .to_manifest_json_string()
        .expect("Rust canonical manifest JSON を出力できるべき");

    assert_eq!(
        actual, expected,
        "selfhost/Rust manifest の wire value が一致するべき"
    );
    assert_eq!(
        actual_json, expected_json,
        "selfhost/Rust manifest の canonical bytes が一致するべき"
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
