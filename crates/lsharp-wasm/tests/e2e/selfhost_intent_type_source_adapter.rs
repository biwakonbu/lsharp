use super::support::*;

fn run_source_adapter_runtime(harness: &str) -> String {
    let adapter = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/IntentSource.ls"),
    )
    .expect("canonical IntentSource.ls が読み込めない");
    let whitespace = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/Whitespace.ls"),
    )
    .expect("canonical Whitespace.ls が読み込めない");
    compile_and_run(&format!(
        "{}\n{}\n{}\n{}",
        selfhost_parser_runtime_bundle(),
        whitespace,
        adapter,
        harness
    ))
}

/// EC-M2-01: ADT 宣言後の source metadata も node registry へ投影する。
#[test]
fn test_e2e_selfhost_source_adapter_projects_type_definition_metadata() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(type (Result e) (Ok Int) (Err e) :intent \"intent:checkout/result\" \"The result models checkout completion\" :claim \"claim:checkout/result-total\" \"Every checkout returns a result\" :motivates \"intent:checkout/result\" \"claim:checkout/result-total\")")
        result (source-graph-from-program program)
        graph (source-graph-result-value result)
        nodes (source-graph-nodes graph)
        edges (source-graph-edges graph)
        edge (vector-get edges 0)]
    (do
      (print (vector-length program))
      (print (source-graph-result-status result))
      (print (vector-length nodes))
      (print-string (source-node-id (vector-get nodes 0)))
      (print-string "\n")
      (print-string (source-node-id (vector-get nodes 1)))
      (print-string "\n")
      (print (vector-length edges))
      (print (source-edge-kind edge))
      (print-string (source-edge-left edge))
      (print-string "\n")
      (print-string (source-edge-right edge))
      (print-string "\n")
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "1",
            "1",
            "2",
            "intent:checkout/result",
            "claim:checkout/result-total",
            "1",
            "10",
            "intent:checkout/result",
            "claim:checkout/result-total"
        ],
        "selfhost type 定義の source metadata は宣言順に投影されるべき"
    );
}

/// EC-M2-01: record 宣言後の source metadata も node registry へ投影する。
#[test]
fn test_e2e_selfhost_source_adapter_projects_record_definition_metadata() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(type (Point a) (record (: x Int) (: y a)) :intent \"intent:geometry/point\" \"A point has two coordinates\" :claim \"claim:geometry/point-typed\" \"Each coordinate follows the declared type\" :constrained-by \"claim:geometry/point-typed\" \"assumption:geometry/coordinate-typed\" :assumption \"assumption:geometry/coordinate-typed\" \"Each coordinate is typed\")")
        result (source-graph-from-program program)
        graph (source-graph-result-value result)
        nodes (source-graph-nodes graph)
        edges (source-graph-edges graph)
        edge (vector-get edges 0)]
    (do
      (print (vector-length program))
      (print (source-graph-result-status result))
      (print (vector-length nodes))
      (print-string (source-node-id (vector-get nodes 0)))
      (print-string "\n")
      (print-string (source-node-id (vector-get nodes 1)))
      (print-string "\n")
      (print-string (source-node-id (vector-get nodes 2)))
      (print-string "\n")
      (print (vector-length edges))
      (print (source-edge-kind edge))
      (print-string (source-edge-left edge))
      (print-string "\n")
      (print-string (source-edge-right edge))
      (print-string "\n")
      0)))
"#;

    let output = run_source_adapter_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "1",
            "1",
            "3",
            "intent:geometry/point",
            "claim:geometry/point-typed",
            "assumption:geometry/coordinate-typed",
            "1",
            "11",
            "claim:geometry/point-typed",
            "assumption:geometry/coordinate-typed"
        ],
        "selfhost record 定義の source metadata は宣言順に投影されるべき"
    );
}
