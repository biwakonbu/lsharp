//! selfhost evidence registry runtime harness。

use super::super::support::*;

pub(super) fn run_evidence_registry_runtime(harness: &str) -> String {
    let intent_source = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/IntentSource.ls"),
    )
    .expect("canonical IntentSource.ls が読み込めない");
    let whitespace = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/Whitespace.ls"),
    )
    .expect("canonical Whitespace.ls が読み込めない");
    let evidence = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/Evidence.ls"),
    )
    .expect("canonical Evidence.ls が読み込めない");
    let json_rpc =
        std::fs::read_to_string(selfhost_project_root().join("selfhost/src/Tools/Lsp/JsonRpc.ls"))
            .expect("canonical JsonRpc.ls が読み込めない");
    compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_parser_runtime_bundle(),
        json_rpc,
        whitespace,
        intent_source,
        evidence,
        harness
    ))
}
