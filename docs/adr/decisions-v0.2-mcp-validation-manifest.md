# ADR: v0.2 MCP `lsharp_validate` manifest input

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-driver/src/mcp_server.rs`
- Related: `EC-M2-03`, `decisions-v0.2-mcp-validation-tool.md`,
  `decisions-v0.2-validation-input-parser.md`

## Context

Rust MCP の `lsharp_validate` は source/file から intent graph report を返せるようになったが、
CLI と同じ version 1 JSON manifest を MCP consumer が直接検証する入力境界は未接続だった。
manifest の parser を MCP 専用に複製すると、schema version、referential closure、fail-closed
diagnostic が CLI と乖離する。

## Decision

- `lsharp_validate` の入力は `source`、`file`、`manifest`、`manifest_file` のいずれか一つに限定する。
- `manifest` は version 1 JSON object または JSON string、`manifest_file` は JSON file path とする。
- manifest input は `lsharp_types::validation_input::parse_intent_graph_json` へ渡し、source input と
  同じ `IntentGraph::validate()` / `ValidationReport::to_json_value()` を使う。
- `tools/list` の input schema は入力を `oneOf` で表し、manifest の object/string variant と file path
  の型を公開する。複数入力、未対応 schema version、unknown field、referential error は tool error
  (`isError: true`) として返し、成功 report に変換しない。
- この slice では MCP からの manifest emission は追加せず、validation report と manifest artifact を
  別の出力境界として維持する。

## Evidence

- RED: manifest 未対応時に direct object/file、schema、error boundary の 4 tests が失敗することを確認。
- GREEN: source/file 回帰を含む `mcp_server::tests` 27 tests が pass。direct object/string、
  `manifest_file`、JSON-RPC、複数 input rejection、schema version error を固定した。
- Follow-up GREEN: `tools/list` の `manifest` object schema が version 1 の required envelope
  (`schema_version` / `nodes` / `evidence` / `edges`)、optional `reviews`、unknown top-level field
  rejection を公開することを `test_validate_tool_manifest_input_schema_declares_versioned_graph_fields`
  で固定した。input/output schema は同じ helper を共有し、MCP consumer が parser の必須境界を
  schema だけで欠落させない。
- Gate: `cargo test -p lsharp-driver --bin lsharp mcp_server::tests -- --nocapture`（41 tests）、
  対象ファイルの rustfmt、`git diff --check`。

## Boundary and follow-up

これは Rust MCP の manifest input/report wiring に限定した verified slice である。EmbeddedCli の
Rust-host actual Wasm manifest output wiringは別ADRで接続済みだが、native manifest emission、
selfhost/native report parity、Mac Apple Silicon / Linux x86_64 artifact/runtime evidence は
未完了のため、`TODO.md` の `EC-M2-03` は `[~]` のまま維持する。
