use super::support::*;

fn run_source_adapter_runtime(harness: &str) -> String {
    let adapter = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/IntentSource.ls"),
    )
    .expect("canonical IntentSource.ls が読み込めない");
    compile_and_run(&format!(
        "{}\n{}\n{}",
        selfhost_parser_runtime_bundle(),
        adapter,
        harness
    ))
}

fn run_source_evidence_runtime(harness: &str) -> String {
    let intent_source = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/IntentSource.ls"),
    )
    .expect("canonical IntentSource.ls が読み込めない");
    let evidence_source = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/Evidence.ls"),
    )
    .expect("canonical Evidence.ls が読み込めない");
    let json_rpc_source =
        std::fs::read_to_string(selfhost_project_root().join("selfhost/src/Tools/Lsp/JsonRpc.ls"))
            .expect("canonical JsonRpc.ls が読み込めない");
    compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        selfhost_parser_runtime_bundle(),
        intent_source,
        json_rpc_source,
        evidence_source,
        harness
    ))
}

/// EC-M2-01: parser-owned source metadata を node/edge record へ投影する。
#[test]
fn test_e2e_selfhost_source_adapter_projects_nodes_and_edges() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn cancel [] :intent \"intent:checkout/safe-cancel\" \"Users can cancel\" :claim \"claim:checkout/rejects-shipped\" \"Shipped orders are rejected\" :assumption \"assumption:checkout/state-authoritative\" \"Shipment state is authoritative\" :open-question \"open-question:checkout/after-label\" \"Can cancellation happen after a label?\" :motivates \"intent:checkout/safe-cancel\" \"claim:checkout/rejects-shipped\" :constrained-by \"claim:checkout/rejects-shipped\" \"assumption:checkout/state-authoritative\" :tested-by \"claim:checkout/rejects-shipped\" \"contract:checkout/cancel-case\" true)"))
        graph (source-graph-result-value result)
        nodes (source-graph-nodes graph)
        edges (source-graph-edges graph)
        intent (vector-get nodes 0)
        claim (vector-get nodes 1)
        assumption (vector-get nodes 2)
        question (vector-get nodes 3)
        motivates (vector-get edges 0)
        constrained (vector-get edges 1)
        tested (vector-get edges 2)]
    (do
      (print (source-graph-result-status result))
      (print (vector-length nodes))
      (print (vector-length edges))
      (print (source-node-kind intent))
      (print-string (source-node-id intent))
      (print-string "\n")
      (print (source-node-start intent))
      (print (source-node-end intent))
      (print-string (source-node-text intent))
      (print-string "\n")
      (print (source-node-kind claim))
      (print (source-node-kind assumption))
      (print (source-node-kind question))
      (print (source-edge-kind motivates))
      (print (source-edge-start motivates))
      (print (source-edge-end motivates))
      (print (source-edge-kind constrained))
      (print (source-edge-kind tested))
      (print-string (source-edge-left motivates))
      (print-string "\n")
      (print-string (source-edge-right motivates))
      (print-string "\n")
      (print-string (source-edge-left constrained))
      (print-string "\n")
      (print-string (source-edge-right constrained))
      (print-string "\n")
      (print-string (source-edge-right tested))
      (print-string "\n")
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "1",
            "4",
            "3",
            "6",
            "intent:checkout/safe-cancel",
            "16",
            "72",
            "Users can cancel",
            "7",
            "8",
            "9",
            "10",
            "324",
            "397",
            "11",
            "12",
            "intent:checkout/safe-cancel",
            "claim:checkout/rejects-shipped",
            "claim:checkout/rejects-shipped",
            "assumption:checkout/state-authoritative",
            "contract:checkout/cancel-case",
        ],
        "selfhost source adapter は node/edge の kind・ID・本文・endpoint を保持するべき"
    );
}

/// EC-M2-01: nested module/private/impl の defn metadata も source order で収集する。
#[test]
fn test_e2e_selfhost_source_adapter_walks_nested_declarations() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(module Outer (private (defn hidden [] :intent \"intent:checkout/hidden\" \"Hidden intent\")) (impl (Show Int) (defn show [self] :claim \"claim:checkout/show\" \"Show claim\" :motivates \"intent:checkout/hidden\" \"claim:checkout/show\")) (module Inner (defn nested [] :assumption \"assumption:checkout/nested\" \"Nested assumption\")))"))]
    (if (= (source-graph-result-status result) 1)
      (let [graph (source-graph-result-value result)
        nodes (source-graph-nodes graph)
        edges (source-graph-edges graph)
        edge (vector-get edges 0)]
        (do
          (print (source-graph-result-status result))
          (print (vector-length nodes))
          (print (vector-length edges))
          (print-string (source-node-id (vector-get nodes 0)))
          (print-string "\n")
          (print-string (source-node-id (vector-get nodes 1)))
          (print-string "\n")
          (print-string (source-node-id (vector-get nodes 2)))
          (print-string "\n")
          (print (source-edge-kind edge))
          (print-string (source-edge-left edge))
          (print-string "\n")
          (print-string (source-edge-right edge))
          (print-string "\n")
          0))
      (do
        (print (source-graph-result-status result))
        (print (source-graph-error-code (source-graph-result-error result)))
        0))))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "1",
            "3",
            "1",
            "intent:checkout/hidden",
            "claim:checkout/show",
            "assumption:checkout/nested",
            "10",
            "intent:checkout/hidden",
            "claim:checkout/show",
        ],
        "nested module/private/impl の source node は宣言順に収集されるべき"
    );
}

/// EC-M2-01: stable ID の kind mismatch は fail-closed にする。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_kind_mismatch() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn bad [] :claim \"intent:checkout/not-a-claim\" \"wrong kind\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      (print-string "\n")
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "3", "intent:checkout/not-a-claim"],
        "source node の kind と stable ID が不一致なら拒否するべき"
    );
}

/// EC-M2-01: stable ID の segment / wire format が不正なら拒否する。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_invalid_stable_id() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn bad [] :claim \"claim:checkout/bad/key\" \"invalid id\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      (print-string "\n")
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "2", "claim:checkout/bad/key"],
        "stable ID の不正な segment は invalid-id として拒否するべき"
    );
}

/// EC-M2-01: node 本文が whitespace-only なら Rust canonical と同じく拒否する。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_whitespace_only_node_text() {
    const SOURCE: &str = r#"(defn bad [] :claim "claim:checkout/whitespace-text" "  " true)"#;
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn bad [] :claim \"claim:checkout/whitespace-text\" \"  \" true)"))]
    (if (= (source-graph-result-status result) 0)
      (let [error (source-graph-result-error result)]
        (do
          (print (source-graph-result-status result))
          (print (source-graph-error-code error))
          (print (source-graph-error-kind error))
          (print-string (source-graph-error-id error))
          (print-string "\n")
          (print (source-graph-error-start error))
          (print (source-graph-error-end error))
          (print-string "\n")
          0))
      (let [graph (source-graph-result-value result)
        node (vector-get (source-graph-nodes graph) 0)]
        (do
          (print (source-graph-result-status result))
          (print-string (source-node-text node))
          (print-string "\n")
          0)))))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        &lines[..4],
        ["0", "1", "7", "claim:checkout/whitespace-text"],
        "selfhost source node の whitespace-only text は malformed として拒否するべき"
    );
    let start = lines[4]
        .parse::<usize>()
        .expect("whitespace node text の start span は整数であるべき");
    let end = lines[5]
        .parse::<usize>()
        .expect("whitespace node text の end span は整数であるべき");
    assert!(start < end);
    assert!(SOURCE[start..end].starts_with(":claim"));
}

/// EC-M2-01: whitespace-only node text は invalid stable ID より先に malformed code 1 を返す。
#[test]
fn test_e2e_selfhost_source_adapter_reports_empty_node_text_before_invalid_stable_id() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn bad [] :claim \\\"claim:checkout/bad/key\\\" \\\"  \\\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print (source-graph-error-kind error))
      (print-string (source-graph-error-id error))
      (print-string "\n")
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "7"],
        "empty node text は invalid stable ID より先に malformed code 1 を返すべき"
    );
}

/// EC-M2-01: 重複 node は後勝ちにせず拒否する。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_duplicate_nodes() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn duplicate [] :intent \"intent:checkout/cancel\" \"first\" :intent \"intent:checkout/cancel\" \"second\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      (print-string "\n")
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "4", "intent:checkout/cancel"],
        "source node の stable ID 重複は拒否するべき"
    );
}

/// EC-M2-01: graph-owned endpoint が未登録なら edge を追加しない。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_missing_edge_node() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn missing [] :claim \"claim:checkout/rejects-shipped\" \"Shipped orders are rejected\" :motivates \"intent:checkout/absent\" \"claim:checkout/rejects-shipped\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      (print-string "\n")
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "5", "intent:checkout/absent"],
        "edge が参照する未登録 node は fail-closed にするべき"
    );
}

/// EC-M2-01 boundary: source edge payload は endpoint の2要素だけを受理する。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_extra_edge_payload() {
    let harness = r#"
(defn main []
  (let [nodes (vector-push-pair-rooted-v3
                (vector-new 0)
                (source-node-record (source-node-intent) "intent:checkout/cancel" "Users can cancel" 1 2)
                (source-node-record (source-node-claim) "claim:checkout/rejects" "Shipped orders are rejected" 3 4))
        payload (vector-push-triple-rooted-v3
                  (vector-new 0)
                  "intent:checkout/cancel"
                  "claim:checkout/rejects"
                  "unexpected")
        form (vector-push-quad-rooted-v3
               (vector-new 4)
               (source-edge-motivates)
               payload
               10
               20)
        result (source-edge-form-result form nodes)
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print (source-graph-error-kind error))
      (print-string (source-graph-error-id error))
      (print-string "\n")
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "10"],
        "source edge の余分な payload は malformed として fail-closed にするべき"
    );
}

/// EC-M2-01 boundary: source node payload は ID と本文の2要素だけを受理する。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_extra_node_payload() {
    let harness = r#"
(defn main []
  (let [payload (vector-push-triple-rooted-v3
                  (vector-new 0)
                  "claim:checkout/rejects"
                  "Shipped orders are rejected"
                  "unexpected")
        form (vector-push-quad-rooted-v3
               (vector-new 4)
               (source-node-claim)
               payload
               10
               20)
        result (source-node-form-result form)
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-graph-error-code error))
      (print (source-graph-error-kind error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "7"],
        "source node の余分な payload は malformed として fail-closed にするべき"
    );
}

/// EC-M2-01 boundary: tagged node/edge form は [kind, payload, start, end] だけを受理する。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_extra_form_fields() {
    let harness = r#"
(defn main []
  (let [node-payload (vector-push-pair-rooted-v3
                       (vector-new 0)
                       "claim:checkout/rejects"
                       "Shipped orders are rejected")
        node-base (vector-push-quad-rooted-v3
                    (vector-new 4)
                    (source-node-claim)
                    node-payload
                    10
                    20)
        node-form (vector-push-single-rooted-v3 node-base "unexpected")
        node-result (source-node-form-result node-form)
        node-error (source-result-error node-result)
        nodes (vector-push-pair-rooted-v3
                (vector-new 0)
                (source-node-record (source-node-intent) "intent:checkout/cancel" "Users can cancel" 1 2)
                (source-node-record (source-node-claim) "claim:checkout/rejects" "Shipped orders are rejected" 3 4))
        edge-payload (vector-push-pair-rooted-v3
                       (vector-new 0)
                       "intent:checkout/cancel"
                       "claim:checkout/rejects")
        edge-base (vector-push-quad-rooted-v3
                    (vector-new 4)
                    (source-edge-motivates)
                    edge-payload
                    30
                    40)
        edge-form (vector-push-single-rooted-v3 edge-base "unexpected")
        edge-result (source-edge-form-result edge-form nodes)
        edge-error (source-result-error edge-result)]
    (do
      (print (source-result-status node-result))
      (print (source-graph-error-code node-error))
      (print (source-graph-error-kind node-error))
      (print (source-graph-error-start node-error))
      (print (source-graph-error-end node-error))
      (print (source-result-status edge-result))
      (print (source-graph-error-code edge-error))
      (print (source-graph-error-kind edge-error))
      (print (source-graph-error-start edge-error))
      (print (source-graph-error-end edge-error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "7", "10", "20", "0", "1", "10", "30", "40"],
        "source node/edge の余分な form field は span 付き malformed として拒否するべき"
    );
}

/// EC-M2-01 boundary: short tagged form でも利用可能な kind/span を失わずに拒否する。
#[test]
fn test_e2e_selfhost_source_adapter_preserves_partial_malformed_form_context() {
    let harness = r#"
(defn main []
  (let [node-payload (vector-push-pair-rooted-v3
                       (vector-new 0)
                       "claim:checkout/rejects"
                       "Shipped orders are rejected")
        node-form (vector-push-triple-rooted-v3
                    (vector-new 3)
                    (source-node-claim)
                    node-payload
                    10)
        node-result (source-node-form-result node-form)
        node-error (source-result-error node-result)
        edge-payload (vector-push-pair-rooted-v3
                       (vector-new 0)
                       "intent:checkout/cancel"
                       "claim:checkout/rejects")
        edge-form (vector-push-triple-rooted-v3
                    (vector-new 3)
                    (source-edge-motivates)
                    edge-payload
                    30)
        edge-result (source-edge-form-result edge-form (vector-new 0))
        edge-error (source-result-error edge-result)]
    (do
      (print (source-result-status node-result))
      (print (source-graph-error-code node-error))
      (print (source-graph-error-kind node-error))
      (print (source-graph-error-start node-error))
      (print (source-graph-error-end node-error))
      (print (source-result-status edge-result))
      (print (source-graph-error-code edge-error))
      (print (source-graph-error-kind edge-error))
      (print (source-graph-error-start edge-error))
      (print (source-graph-error-end edge-error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1", "7", "10", "-1", "0", "1", "10", "30", "-1"],
        "短い source node/edge form は利用可能な kind と開始位置を保持するべき"
    );
}

/// EC-M2-02 boundary: evidence registry が未接続の supports/contradicts は成功にしない。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_unregistered_evidence_edge() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn counterexample [] :claim \"claim:checkout/rejects-shipped\" \"Shipped orders are rejected\" :contradicts \"evidence:checkout/counterexample\" \"claim:checkout/rejects-shipped\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      (print-string "\n")
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "6", "evidence:checkout/counterexample"],
        "evidence registry 未接続の contradicts は明示 boundary error にするべき"
    );
}

/// EC-M2-02: 未登録 evidence は壊れた evidence ID より先に registry-required code 6 を返す。
#[test]
fn test_e2e_selfhost_source_adapter_reports_evidence_registry_before_invalid_edge_id() {
    let harness = r#"
(defn main []
  (let [supports (source-graph-from-program
                   (parse-program "(defn supports [] :claim \"claim:checkout/rejects\" \"The API rejects shipped orders\" :supports \"evidence:checkout\" \"claim:checkout/rejects\" true)"))
        contradicts (source-graph-from-program
                      (parse-program "(defn contradicts [] :claim \"claim:checkout/rejects\" \"The API rejects shipped orders\" :contradicts \"evidence:checkout\" \"claim:checkout/rejects\" true)"))
        supports-error (source-graph-result-error supports)
        contradicts-error (source-graph-result-error contradicts)]
    (do
      (print (source-graph-result-status supports))
      (print (source-graph-error-code supports-error))
      (print-string (source-graph-error-id supports-error))
      (print-string "\n")
      (print (source-graph-result-status contradicts))
      (print (source-graph-error-code contradicts-error))
      (print-string (source-graph-error-id contradicts-error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "6", "evidence:checkout", "0", "6", "evidence:checkout",],
        "未登録 supports/contradicts は壊れた evidence ID より先に registry-required になるべき"
    );
}

/// EC-M2-02: evidence required field は invalid evidence ID より先に code 4 を返す。
#[test]
fn test_e2e_selfhost_source_evidence_reports_empty_runner_before_invalid_id() {
    let harness = r#"
(defn main []
  (let [nodes (vector-push-single-rooted-v3
                (vector-new 0)
                (source-node-record (source-node-claim) "claim:checkout/cancel" "The API rejects shipped orders" 1 2))
        payload (source-evidence-payload
                  "evidence:checkout"
                  "claim:checkout/cancel"
                  "case"
                  "pass"
                  ""
                  "aarch64-apple-darwin"
                  "source-required-precedence"
                  "sha256:required-precedence"
                  1
                  0
                  "required-precedence-generator"
                  (vector-new 0)
                  (vector-new 0)
                  "required-precedence-producer"
                  "0.2.0-dev"
                  "2026-07-28T00:00:00Z"
                  "same-author")
        form (source-evidence-form payload 10 20)
        result (source-evidence-form-result form nodes)
        error (source-result-error result)]
    (do
      (print (source-result-status result))
      (print (source-evidence-error-code error))
      (print-string (source-evidence-error-field error))
      (print-string "\n")
      (print (source-evidence-error-start error))
      (print (source-evidence-error-end error))
      0)))
"#;

    let output = run_source_evidence_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "4", "runner", "10", "20"],
        "empty runner は invalid evidence ID より先に required-field code 4 を返すべき"
    );
}

/// EC-M2-01: fail-closed error は現在の directive span と関連する最初の span を返す。
#[test]
fn test_e2e_selfhost_source_adapter_reports_error_spans() {
    let harness = r#"
(defn main []
  (let [duplicate (source-graph-from-program
                    (parse-program "(defn duplicate [] :intent \"intent:checkout/cancel\" \"first\" :intent \"intent:checkout/cancel\" \"second\" true)"))
        duplicate-error (source-graph-result-error duplicate)
        missing (source-graph-from-program
                  (parse-program "(defn missing [] :claim \"claim:checkout/rejects-shipped\" \"Shipped orders are rejected\" :motivates \"intent:checkout/absent\" \"claim:checkout/rejects-shipped\" true)"))
        missing-error (source-graph-result-error missing)]
    (do
      (print (source-graph-error-code duplicate-error))
      (print (source-graph-error-start duplicate-error))
      (print (source-graph-error-end duplicate-error))
      (print (source-graph-error-related-start duplicate-error))
      (print (source-graph-error-related-end duplicate-error))
      (print (source-graph-error-code missing-error))
      (print (source-graph-error-start missing-error))
      (print (source-graph-error-end missing-error))
      (print (source-graph-error-related-start missing-error))
      (print (source-graph-error-related-end missing-error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["4", "60", "101", "19", "59", "5", "87", "155", "-1", "-1"],
        "selfhost source adapter は現在の directive span と duplicate の first span を保持するべき"
    );
}

/// EC-M2-02: source の opaque review registry を graph の node/edge と分離して保持する。
#[test]
fn test_e2e_selfhost_source_adapter_projects_review_registry() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn review [] :review \"review:checkout/reviewer-001\" \"sha256:review-provenance-001\" \"redacted\" true)"))
        graph (source-graph-result-value result)
        nodes (source-graph-nodes graph)
        edges (source-graph-edges graph)
        reviews (source-graph-reviews graph)
        review (vector-get reviews 0)]
    (do
      (print (source-graph-result-status result))
      (print (vector-length nodes))
      (print (vector-length edges))
      (print (vector-length reviews))
      (print-string (source-review-id review))
      (print-string "\n")
      (print-string (source-review-provenance-digest review))
      (print-string "\n")
      (print-string (source-review-visibility review))
      (print-string "\n")
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "1",
            "0",
            "0",
            "1",
            "review:checkout/reviewer-001",
            "sha256:review-provenance-001",
            "redacted",
        ],
        "selfhost source adapter は review registry を node/edge と混ぜず opaque field のまま保持するべき"
    );
}

/// EC-M2-02: review ID の重複は後勝ちにせず source span 付きで拒否する。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_duplicate_reviews() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn duplicate [] :review \"review:checkout/reviewer-001\" \"sha256:first\" \"public\" :review \"review:checkout/reviewer-001\" \"sha256:second\" \"redacted\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      (print-string "\n")
      (print (source-graph-error-related-start error))
      (print (source-graph-error-related-end error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "7", "review:checkout/reviewer-001", "19", "81"],
        "review registry の duplicate ID は first declaration の span を関連情報として保持するべき"
    );
}

/// EC-M2-02: review の provenance digest と visibility は fail-closed に検証する。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_invalid_review_fields() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn invalid [] :review \"review:checkout/reviewer-001\" \"   \" \"public\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "8", "review:checkout/reviewer-001"],
        "空 digest や未知 visibility の review は source graph に登録しないべき"
    );
}

/// EC-M2-02: empty review ID は required review metadata code 8 として拒否する。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_empty_review_id_as_invalid_review_field() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn invalid [] :review \"\" \"sha256:review-provenance-001\" \"public\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "8"],
        "empty review ID は required review metadata code 8 として拒否するべき"
    );
}

/// EC-M2-02: blank review provenance は invalid review code 8 を invalid ID より先に返す。
#[test]
fn test_e2e_selfhost_source_adapter_reports_blank_review_digest_before_invalid_review_id() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn invalid [] :review \\\"review:checkout\\\" \\\"  \\\" \\\"public\\\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "8"],
        "blank review digest は invalid review code 8 を invalid ID より先に返すべき"
    );
}

/// EC-M2-02: review/change edge metadata は typed edge record へ投影する。
#[test]
fn test_e2e_selfhost_source_adapter_projects_review_change_edges() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn review [] :review \"review:checkout/reviewer-001\" \"sha256:review-provenance-001\" \"redacted\" :claim \"claim:checkout/cancel-rejects-shipped\" \"The API rejects shipped orders\" :evaluates \"review:checkout/reviewer-001\" \"claim:checkout/cancel-rejects-shipped\" :invalidates \"change:checkout/api-v2\" \"review:checkout/reviewer-001\" true)"))
        graph (source-graph-result-value result)
        edges (source-graph-edges graph)
        evaluates (vector-get edges 0)
        invalidates (vector-get edges 1)]
    (do
      (print (source-graph-result-status result))
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

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "1",
            "2",
            "17",
            "review:checkout/reviewer-001",
            "claim:checkout/cancel-rejects-shipped",
            "18",
            "change:checkout/api-v2",
            "review:checkout/reviewer-001",
        ],
        "evaluates/invalidates は relation と endpoint の stable ID を保持するべき"
    );
}

/// EC-M2-02: review registry が明示されている場合、未登録 review endpoint は拒否する。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_missing_review_edge_endpoint() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn review [] :review \"review:checkout/reviewer-001\" \"sha256:review-provenance-001\" \"redacted\" :claim \"claim:checkout/cancel-rejects-shipped\" \"The API rejects shipped orders\" :evaluates \"review:checkout/missing\" \"claim:checkout/cancel-rejects-shipped\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "10", "review:checkout/missing"],
        "明示 review registry にない evaluates review は graph closure error にするべき"
    );
}

/// EC-M2-02: 未登録 review は不正な evaluates subject kind より先に code 10 を返す。
#[test]
fn test_e2e_selfhost_source_adapter_reports_missing_review_before_invalid_evaluates_subject() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn review [] :review \"review:checkout/registered\" \"sha256:review-provenance-001\" \"public\" :evaluates \"review:checkout/missing\" \"review:checkout/registered\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "10", "review:checkout/missing"],
        "未登録 evaluates review は不正な subject kind より先に missing-review になるべき"
    );
}

/// EC-M2-02: invalidates の subject は review/evidence に限定する。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_invalid_invalidation_subject_kind() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn review [] :claim \"claim:checkout/cancel-rejects-shipped\" \"The API rejects shipped orders\" :invalidates \"change:checkout/api-v2\" \"claim:checkout/cancel-rejects-shipped\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "9", "claim:checkout/cancel-rejects-shipped"],
        "invalidates の claim subject は typed kind mismatch として拒否するべき"
    );
}

/// EC-M2-02: invalidates の missing review は directive span を保持する。
#[test]
fn test_e2e_selfhost_source_adapter_preserves_missing_invalidates_review_span() {
    const SOURCE: &str = "(defn review [] :review \"review:checkout/registered\" \"sha256:review-provenance-001\" \"public\" :invalidates \"change:checkout/api-v2\" \"review:checkout/missing\" true)";
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn review [] :review \"review:checkout/registered\" \"sha256:review-provenance-001\" \"public\" :invalidates \"change:checkout/api-v2\" \"review:checkout/missing\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      (print-string "\n")
      (print (source-graph-error-start error))
      (print (source-graph-error-end error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        &lines[..3],
        ["0", "10", "review:checkout/missing"],
        "missing invalidates review は code 10 と ID を返すべき"
    );
    let start = lines[3]
        .parse::<usize>()
        .expect("missing invalidates review の start span は整数であるべき");
    let end = lines[4]
        .parse::<usize>()
        .expect("missing invalidates review の end span は整数であるべき");
    assert!(start < end);
    assert!(SOURCE[start..end].starts_with(":invalidates"));
}

/// EC-M2-02 boundary: selfhost IntentSource が evidence registry を未接続のまま扱う間は fail-closed にする。
#[test]
fn test_e2e_selfhost_source_adapter_rejects_unregistered_review_evidence_subject() {
    let harness = r#"
(defn main []
  (let [result (source-graph-from-program
                 (parse-program "(defn review [] :review \"review:checkout/reviewer-001\" \"sha256:review-provenance-001\" \"redacted\" :evaluates \"review:checkout/reviewer-001\" \"evidence:checkout/review-001\" true)"))
        error (source-graph-result-error result)]
    (do
      (print (source-graph-result-status result))
      (print (source-graph-error-code error))
      (print-string (source-graph-error-id error))
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "6", "evidence:checkout/review-001"],
        "evidence registry 未接続の evaluates subject は明示 boundary error にするべき"
    );
}
