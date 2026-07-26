# ADR: driver MCP language-tool implementation 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-driver/src/mcp_server.rs`, `crates/lsharp-driver/src/mcp_language.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-driver/src/mcp_server.rs` は MCP protocol/tool dispatch、validation/context、compile/run に加えて、
hover/check、completion、format、definition/references、errors と source/position 診断 helper も保持していた。
これらの language-tool implementation は LSP analysis と診断 response をまとめて扱う独立した境界である。

## Decision

- hover/check、legacy migration diagnostics、completion、format、definition、references、errors と source/position helper を `mcp_language.rs` へ移動する。
- 親では `include!("mcp_language.rs")` を使い、既存 `call_tool` dispatch、LSP API、project context helper、error reference、`mcp_io_error` への private reference を同一 module namespace に維持する。
- source/file precedence、position と `col` alias、diagnostic code/range、migration response、completion candidates、error reference response は変更しない。
- `error_code` がない場合に未知コード response を生成せず、既存診断を返す contract test を追加する。

## Evidence

- RED: `include!("mcp_language.rs")` を追加した child 不在状態で `E0583` を確認。
- GREEN: `test_errors_tool_requires_error_code`、MCP focused 35 tests、`lsharp-driver` unit 157 件が pass。
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings`、専用 target の `cargo check --workspace`、対象 Rust
  2024 `rustfmt --check`、`git diff --check`、docs audit が pass。
- parent は 1211 行から 1001 行、`mcp_language.rs` は 217 行となった。`default_path_delegation` の既知 7 failures
  は origin/main の embedded component/selfhost default-path boundary として今回の移動とは独立に分類する。

## Consequences

MCP language-tool implementation の LSP analysis、diagnostics、source/position conversion、error reference boundary を
protocol、validation、context、compile/run tool から独立してレビューできる。既存の tool response semantics は維持される。
残る MCP search/tool implementation の分割、default-path integration blocker、selfhost/native parity、I-01 / I-08 aggregate は
この partial slice では完了扱いにしない。
