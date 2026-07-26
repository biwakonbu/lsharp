# ADR: driver MCP protocol transport 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-driver/src/mcp_server.rs`, `crates/lsharp-driver/src/mcp_protocol.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-driver/src/mcp_server.rs` は stdio の入出力、JSON-RPC dispatch、MCP tool registry/schema、各 tool
実装を一つのファイルに保持していた。transport/dispatch は tool 実装の変更とは独立しており、MCP protocol
boundary を親の CLI/tool 実装から切り離せる。

## Decision

- `MCP_PROTOCOL_VERSION`、`run_stdio_server`、`handle_jsonrpc_message` を `mcp_protocol.rs` へ移動する。
- 親では `include!("mcp_protocol.rs")` を使い、既存の private name、tool list/call wiring、JSON-RPC result/error
  helpers への参照を同一 module namespace に維持する。
- stdio の空行処理、JSON parse/serialize/flush error、initialize/ping/tools/list/tools/call の response shape、
  unknown method の error code は変更しない。
- initialize response の protocol version、tools capability、server name を contract test で固定する。

## Evidence

- RED: `include!("mcp_protocol.rs")` を追加した child 不在状態で `E0583` を確認。
- GREEN: `test_initialize_response_advertises_mcp_protocol_and_tools_capability` と既存 MCP focused 31 tests が pass。
- `lsharp-driver` unit 153 件、`cargo clippy -p lsharp-driver --all-targets -- -D warnings`、専用 target の
  `cargo check --workspace`、対象 Rust 2024 `rustfmt --check`、`git diff --check`、docs audit が pass。
- parent は 1578 行から 1476 行、`mcp_protocol.rs` は 119 行となった。`default_path_delegation` の既知 7 failures
  は origin/main の embedded component/selfhost default-path boundary として今回の移動とは独立に分類する。

## Consequences

MCP transport/dispatch boundary を tool implementation から独立してレビューできる。既存の MCP protocol
response と private call path は維持される。tool implementation の追加分割、default-path integration blocker、
selfhost/native parity、I-01 / I-08 aggregate はこの partial slice では完了扱いにしない。
