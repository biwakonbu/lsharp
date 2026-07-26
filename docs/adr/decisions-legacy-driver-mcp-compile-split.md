# ADR: driver MCP compile/run execution 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-driver/src/mcp_server.rs`, `crates/lsharp-driver/src/mcp_compile.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-driver/src/mcp_server.rs` は MCP protocol/tool dispatch、validation/context に加えて、MCP の
`lsharp_compile_run` 用 temporary source staging、Wasm compile、WASI execution も保持していた。この execution
boundary は read-only context や diagnostics と異なり、compile target と runtime output をまとめて扱う。

## Decision

- `compile_run_tool` を `mcp_compile.rs` へ移動する。
- 親では `include!("mcp_compile.rs")` を使い、既存 `call_tool` path、`commands::compile`、`mcp_io_error`、WASI runner
  への private reference を同一 module namespace に維持する。
- source/file input の precedence、Preview1 target、formatted source、stdout、exit code、temporary directory
  staging と error messages は変更しない。
- source/file がない場合に compile/runtime を開始せず、既存診断を返す contract test を追加する。

## Evidence

- RED: `include!("mcp_compile.rs")` を追加した child 不在状態で `E0583` を確認。
- GREEN: `test_compile_run_tool_requires_source_or_file`、MCP focused 34 tests、`lsharp-driver` unit 156 件が pass。
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings`、専用 target の `cargo check --workspace`、対象 Rust
  2024 `rustfmt --check`、`git diff --check`、docs audit が pass。
- parent は 1240 行から 1211 行、`mcp_compile.rs` は 37 行となった。`default_path_delegation` の既知 7 failures
  は origin/main の embedded component/selfhost default-path boundary として今回の移動とは独立に分類する。

## Consequences

MCP compile/run の source staging、artifact、runtime output boundary を他の MCP tool から独立してレビューできる。
既存の Preview1/WASI response semantics は維持される。残る MCP error/search/tool implementation の分割、default-path
integration blocker、selfhost/native parity、I-01 / I-08 aggregate はこの partial slice では完了扱いにしない。
