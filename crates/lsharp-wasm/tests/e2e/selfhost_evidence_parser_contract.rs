//! selfhost parser の evidence metadata 入力契約。

use super::support::*;

fn run_evidence_parser_runtime(harness: &str) -> String {
    let intent_source = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/IntentSource.ls"),
    )
    .expect("canonical IntentSource.ls が読み込めない");
    let evidence = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/Evidence.ls"),
    )
    .expect("canonical Evidence.ls が読み込めない");
    let whitespace = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/Whitespace.ls"),
    )
    .expect("canonical Whitespace.ls が読み込めない");
    let review_identity = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/ReviewIdentity.ls"),
    )
    .expect("canonical ReviewIdentity.ls が読み込めない");
    let json_rpc =
        std::fs::read_to_string(selfhost_project_root().join("selfhost/src/Tools/Lsp/JsonRpc.ls"))
            .expect("canonical JsonRpc.ls が読み込めない");
    compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_parser_runtime_bundle(),
        json_rpc,
        whitespace,
        intent_source,
        review_identity,
        evidence,
        harness
    ))
}

/// Rust parser と同様に、同じ :evidence field の重複を registry 前に拒否する。
#[test]
fn selfhost_evidence_parser_rejects_duplicate_named_field() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn invalid [] :claim \"claim:checkout/rejects\" \"Shipped orders are rejected\" :evidence \"evidence:checkout/duplicate\" :subject \"claim:checkout/rejects\" :subject \"claim:checkout/rejects\" :method \"case\" :outcome \"pass\" :runner \"cargo-test\" :target \"aarch64-apple-darwin\" :source-commit \"deadbeef\" :artifact-digest \"sha256:abc\" :cases 1 :seed 42 :generator \"checkout-generator\" :producer \"lsharp-test\" :tool-version \"0.2\" :timestamp \"2026-07-25T00:00:00Z\" :independence \"same-author\" true)")
        result (source-evidence-graph-from-program program)]
    (do
      (print-string (int-to-string (source-result-status result)))
      (print-string "\n")
      (if (= (source-result-status result) 0)
        (print-string (int-to-string (source-evidence-error-code (source-result-error result))))
        (print-string "valid"))
      0)))
"#;

    assert_eq!(run_evidence_parser_runtime(harness).trim(), "0\n1");
}
