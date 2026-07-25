# ADR: v0.2 MCP `lsharp_validate` inline manifest output

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-driver/src/mcp_server.rs`
- Related: `EC-M2-03`, `decisions-v0.2-mcp-validation-manifest.md`,
  `decisions-v0.2-source-manifest-emission.md`

## Context

MCP の `lsharp_validate` は source/file/manifest を同じ validation report へ投影できるように
なったが、consumer がその graph を保存・比較するための canonical version 1 manifest は取得できなかった。
MCP で path を直接書き換えると CLI の atomic/durable artifact boundary と責務が混ざるため、
report と inline artifact を分離する必要がある。

## Decision

- `include_manifest: true` を指定した `lsharp_validate` は、report に `manifest` object を追加する。
- manifest は入力種別にかかわらず、検証に使った同じ `IntentGraph::to_manifest_json_value()` から生成する。
  source と JSON manifest の producer が異なる wire shape を返さないことを保証する。
- `include_manifest` は boolean 以外を拒否し、既定値は false とする。既定の report shape は従来と
  変えず、top-level `verified` も追加しない。
- MCP は inline value のみを返し、ファイル書き込みは行わない。`--emit-manifest` の atomic/durable
  path は Rust CLI の責務として維持する。

## Evidence

- RED: `include_manifest` 未対応時に canonical manifest output と option type の契約が失敗することを確認。
- GREEN: `mcp_server::tests` 30 tests が passし、source/manifest 両入力、JSON-RPC report、manifest schema、
  canonical nodes/evidence/edges、invalid option を固定した。
- Gate: `cargo test -p lsharp-driver mcp_server::tests -- --nocapture`、対象 file の rustfmt、
  `git diff --check`。

## Boundary and follow-up

これは Rust MCP の inline artifact wiring に限定した verified slice である。MCP file emission、
EmbeddedCli、selfhost/native report/artifact parity、Mac Apple Silicon / Linux x86_64 runtime evidence は
未完了のため、`TODO.md` の `EC-M2-03` は `[~]` のまま維持する。
