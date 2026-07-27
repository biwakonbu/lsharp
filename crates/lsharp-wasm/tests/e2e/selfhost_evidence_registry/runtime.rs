//! evidence record の登録・manifest projection・Rust oracle parity tests。

use super::super::support::*;
use super::harness::run_evidence_registry_runtime;

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

/// EC-M2-02: review/change edge の Evidence subject は未登録なら fail-closed にする。
#[test]
fn test_e2e_selfhost_evidence_registry_rejects_unregistered_review_change_evidence_subject() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn review [] :review \"review:checkout/reviewer-001\" \"sha256:review-provenance-001\" \"redacted\" :claim \"claim:checkout/rejects\" \"Shipped orders are rejected\" :evaluates \"review:checkout/reviewer-001\" \"evidence:checkout/missing\" true)"))
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-graph-error-code error))
      (print (source-graph-error-kind error))
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "6", "17"],
        "review/change edge の未登録 evidence subject は registry required として拒否するべき"
    );
}

/// EC-M2-02: review registry と review/change edge を selfhost manifest graph へ接続する。
#[test]
fn test_e2e_selfhost_evidence_registry_projects_review_change_edges() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn review [] :review \"review:checkout/reviewer-001\" \"sha256:review-provenance-001\" \"redacted\" :claim \"claim:checkout/rejects\" \"Shipped orders are rejected\" :evaluates \"review:checkout/reviewer-001\" \"claim:checkout/rejects\" :invalidates \"change:checkout/api-v2\" \"review:checkout/reviewer-001\" true)"))
        graph (source-result-value result)
        edges (source-graph-edges graph)
        reviews (source-evidence-graph-reviews graph)
        evaluates (vector-get edges 0)
        invalidates (vector-get edges 1)
        review (vector-get reviews 0)]
    (do
      (print (source-result-status result))
      (print (vector-length reviews))
      (print-string (source-review-id review))
      (print-string "\n")
      (print (vector-length edges))
      (print (source-edge-kind evaluates))
      (print-string (source-edge-left evaluates))
      (print-string "\n")
      (print-string (source-edge-right evaluates))
      (print-string "\n")
      (print (source-edge-kind invalidates))
      (print-string (source-edge-left invalidates))
      (print-string "\n")
      (print-string (source-edge-right invalidates))
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
            "review:checkout/reviewer-001",
            "2",
            "17",
            "review:checkout/reviewer-001",
            "claim:checkout/rejects",
            "18",
            "change:checkout/api-v2",
            "review:checkout/reviewer-001",
        ],
        "manifest consumer は review registry と evaluates/invalidates endpoint を保持するべき"
    );
}

/// EC-M2-02: optional reviews registry と typed review/change edge を version 1 manifest JSON へ投影する。
#[test]
fn test_e2e_selfhost_evidence_registry_serializes_review_change_edges() {
    let harness = r#"
(defn main []
  (let [graph (source-result-value
                (source-evidence-graph-from-program
                  (parse-program "(defn review [] :review \"review:checkout/reviewer-001\" \"sha256:review-provenance-001\" \"redacted\" :claim \"claim:checkout/rejects\" \"Shipped orders are rejected\" :evaluates \"review:checkout/reviewer-001\" \"claim:checkout/rejects\" :invalidates \"change:checkout/api-v2\" \"review:checkout/reviewer-001\" true)")))]
    (do
      (print-string (validation-source-manifest-json graph))
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let manifest: serde_json::Value =
        serde_json::from_str(output.trim()).expect("selfhost manifest should be JSON");

    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["reviews"].as_array().unwrap().len(), 1);
    assert_eq!(manifest["reviews"][0]["namespace"], "checkout");
    assert_eq!(manifest["reviews"][0]["key"], "reviewer-001");
    assert_eq!(
        manifest["reviews"][0]["provenance_digest"],
        "sha256:review-provenance-001"
    );
    assert_eq!(manifest["reviews"][0]["visibility"], "redacted");
    assert_eq!(manifest["edges"].as_array().unwrap().len(), 2);
    assert_eq!(manifest["edges"][0]["relation"], "evaluates");
    assert_eq!(manifest["edges"][0]["review"]["key"], "reviewer-001");
    assert_eq!(manifest["edges"][0]["subject"]["kind"], "claim");
    assert_eq!(manifest["edges"][1]["relation"], "invalidates");
    assert_eq!(manifest["edges"][1]["change"]["key"], "api-v2");
    assert_eq!(manifest["edges"][1]["subject"]["kind"], "review");
}

/// EC-M2-02: evaluates/invalidates の Evidence subject は登録済み evidence にだけ閉じる。
#[test]
fn test_e2e_selfhost_evidence_registry_closes_review_change_evidence_subjects() {
    let harness = r#"
(defn main []
  (let [result (source-evidence-graph-from-program
                 (parse-program "(defn review [] :claim \"claim:checkout/rejects\" \"Shipped orders are rejected\" :review \"review:checkout/reviewer-001\" \"sha256:review-provenance-001\" \"redacted\" :evidence \"evidence:checkout/review-001\" :subject \"claim:checkout/rejects\" :method \"review\" :outcome \"pass\" :runner \"review-tool\" :target \"aarch64-apple-darwin\" :source-commit \"commit-review-1\" :artifact-digest \"sha256:review-1\" :cases 1 :seed 42 :generator \"review-fixture\" :producer \"review-tool\" :tool-version \"0.2.0\" :timestamp \"2026-07-27T00:00:00Z\" :independence \"independent-review\" :evaluates \"review:checkout/reviewer-001\" \"evidence:checkout/review-001\" :invalidates \"change:checkout/api-v2\" \"evidence:checkout/review-001\" true)"))
        graph (source-result-value result)
        edges (source-graph-edges graph)]
    (do
      (print (source-result-status result))
      (print (vector-length (source-evidence-graph-registry graph)))
      (print (vector-length edges))
      (print (source-edge-kind (vector-get edges 0)))
      (print (source-edge-kind (vector-get edges 1)))
      0)))
"#;

    let output = run_evidence_registry_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "1", "2", "17", "18"],
        "evidence subject の review/change edge は evidence registry に閉じて保持するべき"
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

/// EC-M3-01: native stage0 smoke が再利用する source/manifest fixture は Rust oracle と一致する。
#[test]
fn test_e2e_ec_m3_canonical_manifest_fixture_matches_rust_oracle() {
    let source_path =
        selfhost_project_root().join("tests/fixtures/validation/ec-m3-canonical-source.ls");
    let manifest_path =
        selfhost_project_root().join("tests/fixtures/validation/ec-m3-canonical-manifest.json");
    let source = std::fs::read_to_string(&source_path)
        .unwrap_or_else(|e| panic!("{} 読み込み失敗: {e}", source_path.display()));
    let expected_json = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("{} 読み込み失敗: {e}", manifest_path.display()));
    let program =
        lsharp_syntax::parse(&source).expect("EC-M3-01 fixture は Rust oracle で parse できるべき");
    let graph = lsharp_types::validation_source::source_program_to_intent_graph(&program)
        .expect("EC-M3-01 fixture は Rust oracle で graph 化できるべき");
    let expected_value: serde_json::Value =
        serde_json::from_str(&expected_json).expect("canonical fixture は JSON であるべき");

    assert_eq!(
        graph.to_manifest_json_value(),
        expected_value,
        "EC-M3-01 fixture の manifest value は Rust canonical oracle と一致するべき"
    );
    assert_eq!(
        graph
            .to_manifest_json_string()
            .expect("canonical fixture の bytes を生成できるべき"),
        expected_json.trim_end_matches(['\n', '\r']),
        "EC-M3-01 fixture の manifest bytes は Rust canonical serializer と一致するべき"
    );
}

/// EC-M3-01: duplicate source node は Rust oracle でも fail-closed にする。
#[test]
fn test_e2e_ec_m3_duplicate_node_fixture_is_rejected_by_rust_oracle() {
    let source_path =
        selfhost_project_root().join("tests/fixtures/validation/ec-m3-duplicate-node-source.ls");
    let source = std::fs::read_to_string(&source_path)
        .unwrap_or_else(|e| panic!("{} 読み込み失敗: {e}", source_path.display()));
    let program = lsharp_syntax::parse(&source)
        .expect("EC-M3-01 duplicate fixture は Rust oracle で parse できるべき");

    let error = lsharp_types::validation_source::source_program_to_intent_graph(&program)
        .expect_err("duplicate node は Rust oracle で fail-closed に拒否するべき");
    match error {
        lsharp_types::validation_source::SourceGraphError::DuplicateNode { id, .. } => {
            assert_eq!(id, "claim:checkout/rejects");
        }
        other => panic!("duplicate node 以外の source graph error: {other:?}"),
    }
}
