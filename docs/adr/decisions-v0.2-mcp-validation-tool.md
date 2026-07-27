# ADR: v0.2 MCP `lsharp_validate` source report

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-driver/src/mcp_server.rs`
- Related: `EC-M2-03`, `v0.2-milestone-02.md`, `decisions-v0.2-validation-cli.md`

## Context

Rust の `lsharp validate --source` は source parser、intent graph adapter、
`ValidationReport` まで接続済みだが、MCP には同じ report を返す tool がなかった。
MCP consumer が `lsharp_check` の型診断だけで intent/evidence の trace gap を推測すると、
implementation conformance と intent validation を混同する。CLI と同じ canonical model を
使い、MCP からも fact-oriented report を取得できる入口が必要だった。

## Decision

- `lsharp_validate` を MCP `tools/list` / `tools/call` に追加する。
- 入力は `source` または `file` のどちらか一方とし、既存の source/file 解決境界を共有する。
- source を Rust parser、`validation_source::source_program_to_intent_graph`、
  `IntentGraph::validate()` の順に処理し、`ValidationReport::to_json_value()` を
  `structuredContent` として返す。
- output schema は `status` (`pass` / `fail` / `unknown`)、`trace_gaps`、
  `open_questions`、`independent_reviews`、`contradicting_observations`、
  `stale_reviews`、`stale_evidence` を必須とする。stale facts が残る場合は status を
  `unknown` とし、CLI と同じく top-level `verified` は生成しない。
- parse または source graph adapter の失敗は `isError: true` の MCP tool error とし、
  不完全な report や成功値へ変換しない。

## Evidence

- RED: `lsharp_validate` 未登録時に tools/list、direct call、JSON-RPC tools/call の 3 契約が
  失敗することを確認した。
- GREEN: `test_validate_tool_declares_source_input_and_report_output_schema`、
  `test_validate_tool_projects_source_to_fact_oriented_report`、
  `test_validate_tool_is_available_through_jsonrpc_tools_call`、file 入力、parse error の
  5 tests が pass した。
- Gate: `cargo test -p lsharp-driver mcp_server::tests -- --nocapture`。

## Boundary and follow-up

これは Rust MCP の source/file → report wiring に限定した verified slice である。
manifest input、`--emit-manifest`、EmbeddedCli、selfhost/native parity、Mac Apple Silicon /
Linux x86_64 artifact/runtime evidence は未完了のため、`TODO.md` の `EC-M2-03` は `[~]` のまま
維持する。次は同じ observable report を EmbeddedCli と native stage0 へ接続し、両 target の
runtime/artifact evidence を追加する。
