# ADR: driver MCP tool schema/registry 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-driver/src/mcp_server.rs`, `crates/lsharp-driver/src/mcp_schema.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-driver/src/mcp_server.rs` は MCP protocol/tool dispatch、validation/context、compile/run、language tools に加えて、
各 tool の input/output schema と JSON schema helper も保持していた。schema/registry は tool 実装とは異なり、MCP client
に公開する required/properties/diagnostic output 契約を集約する境界である。

## Decision

- tool constructor、input/output schema、JSON schema helper、validation input schema を `mcp_schema.rs` へ移動する。
- `McpTool` 型、`list_tools`、`call_tool`、JSON-RPC protocol dispatch は親に残し、親では `include!("mcp_schema.rs")` を使う。
- tool names、required/properties、`lsharp_check` / `lsharp_validate` の output schema、error reference schema の semantics は変更しない。
- `lsharp_errors` input schema に `error_code` が required であることを contract test で固定する。

## Evidence

- RED: `include!("mcp_schema.rs")` を追加した child 不在状態で `E0583` を確認。
- GREEN: `test_error_tool_input_schema_requires_error_code`、MCP focused 36 tests、`lsharp-driver` unit 158 件が pass。
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings`、専用 target の `cargo check --workspace`、対象 Rust
  2024 `rustfmt --check`、`git diff --check`、docs audit が pass。
- parent は 1001 行から 796 行、`mcp_schema.rs` は 212 行となった。`default_path_delegation` の既知 7 failures
  は origin/main の embedded component/selfhost default-path boundary として今回の移動とは独立に分類する。

## Consequences

MCP client-facing schema/registry contract を protocol、validation、context、compile/run、language tool implementation
から独立してレビューできる。既存 tool registration と schema response semantics は維持される。残る MCP search/tool
implementation の分割、default-path integration blocker、selfhost/native parity、I-01 / I-08 aggregate はこの partial slice
では完了扱いにしない。
