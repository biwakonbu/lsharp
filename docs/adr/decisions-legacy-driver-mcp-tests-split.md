# ADR: driver MCP inline test suite 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-driver/src/mcp_server.rs`, `crates/lsharp-driver/src/mcp_tests.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-driver/src/mcp_server.rs` は MCP protocol/tool dispatch と各 helper のほか、JSON-RPC、schema、validation、
context、compile、language tool の inline tests も保持していた。test-only fixture が production dispatch と同居し、
MCP implementation の追加分割時に diff と private namespace の境界を見通しにくくしていた。

## Decision

- inline `mcp_server::tests` module を `mcp_tests.rs` へ移動する。
- parent では `#[cfg(test)] include!("mcp_tests.rs")` を使い、既存の `mcp_server::tests::*` namespace と private helper access を保つ。
- MCP JSON-RPC/schema/validation/context/compile/language test fixtures、assertion、runtime semantics は変更しない。
- private `jsonrpc_result` helper access contract test を追加する。

## Evidence

- RED: `include!("mcp_tests.rs")` を追加した child 不在状態で `E0583` を確認。
- GREEN: MCP focused 37件、`lsharp-driver` unit 159件が pass。
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings`、専用 target の `cargo check --workspace`、対象 Rust 2024 `rustfmt --check`、`git diff --check` が pass。
- parent は 796 行から 110 行、`mcp_tests.rs` は 697 行となった。今回の変更で MCP production semantics は変更していない。

## Consequences

MCP test fixtures and private contract coverage are independently reviewable while the parent keeps protocol/tool production integration small. Remaining MCP search production split, default-path integration boundary, selfhost/native parity, and I-01/I-08 aggregate are incomplete.
