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
- Presence follow-up GREEN: `reviews` を省略した `evaluates` edge は opaque endpoint として
  `status: unknown` を返し、`reviews: []` を明示した同じ未登録 edge は tool error として拒否する
  対照を `mcp_server::tests` へ追加した。CLI の presence semantics と MCP の report/error boundary
  を同じ parser policyで固定する。
- Gate: `cargo test -p lsharp-driver --bin lsharp mcp_server::tests -- --nocapture`（41 tests）、
  対象ファイルの rustfmt、`git diff --check`。
- Presence gate: 同 focused suite 43 tests（review registry 6 tests）と `cargo clippy -p lsharp-driver
  --bin lsharp --tests -- -D warnings`。
- Numeric schema follow-up: `tools/list` の manifest object schema に `nodes[].span` と
  `evidence[].execution.sampling` の unsigned fields（`start` / `end` / `cases` / `seed` /
  `shrinks[]` / `coverage.*`）を `type: integer` と `minimum: 0` で宣言した。入力と出力で同じ
  helper を共有し、MCP consumer が小数や負数を静的契約上受け付けない境界を parser の typed
  serde 契約と同期する。
- Numeric schema gate: 新規 schema boundary test と既存 `mcp_server::tests` 44 tests、対象 binary
  の `cargo clippy --tests -- -D warnings`、rustfmt、`git diff --check` を pass した。
- Numeric runtime follow-up: MCP の direct `manifest` string input でも、`span.start/end` と
  `sampling.cases/seed/shrinks[]/coverage.*` の fractional、`null`、`u64::MAX + 1` を全6 fieldで
  `validation manifest の parse` error として fail-closed にする回帰 matrix（18 cases）を追加した。
  MCP は report JSON や canonical manifest を返さず、`isError: true` の tool result に留める。
- Numeric runtime gate: `mcp_server::tests` 45 tests と対象 binary の `cargo clippy --tests -- -D warnings`
  を pass。既存 typed serde parser の境界を MCP tool まで接続した Rust-host verified sliceであり、
  production code の変更はない。
- Typed edge schema follow-up: 公開 `docs/schemas/intent-graph.schema.json` と同じ6 relation variant
  （`motivates` / `constrained-by` / `tested-by` / `supports|contradicts` / `evaluates` /
  `invalidates`）を `edges[].oneOf` へ追加し、stable ID の namespace/key pattern と、evidence /
  review / invalidation subject の kind enum を MCP input/output schema に反映した。input/output は
  同じ manifest helper を共有する。
- Typed edge schema gate: 新規 schema parity test と `mcp_server::tests` 46 tests、対象 binary の
  `cargo clippy --tests -- -D warnings`、rustfmt、`git diff --check`、docs audit（0 errors/warnings）を
  passした。

## Boundary and follow-up

これは Rust MCP の manifest input/report wiring に限定した verified slice である。EmbeddedCli の
Rust-host actual Wasm manifest output wiringは別ADRで接続済みだが、native manifest emission、
selfhost/native report parity、Mac Apple Silicon / Linux x86_64 artifact/runtime evidence は
未完了のため、`TODO.md` の `EC-M2-03` は `[~]` のまま維持する。今回の numeric schema は static
`tools/list` 契約と Rust MCP lane に限定され、JSON Schema validator 実行、selfhost/native MCP、
current-source artifact/runtime、supported 2 targets の完了証拠には数えない。
MCP runtime matrixも同じく Rust-host lane に限られ、native/selfhost の診断・target parity は未検証である。
今回の typed edge schema も static `tools/list` contract と Rust MCP lane に限定され、JSON Schema 実
validator、selfhost/native MCP producer、Mac/Linux artifact/runtime の完了証拠には数えない。
