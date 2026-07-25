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
