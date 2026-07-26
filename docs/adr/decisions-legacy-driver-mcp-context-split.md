# ADR: driver MCP context/read-only 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-driver/src/mcp_server.rs`, `crates/lsharp-driver/src/mcp_context.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-driver/src/mcp_server.rs` は MCP dispatch と validation に加えて、project context、installed package
API、stdlib API、依存関係要約、package/module discovery を保持していた。これらは source mutation や compile/run
とは独立した read-only context boundary であり、同じ責務として追跡できる。

## Decision

- `project_context_tool`、`package_api_tool`、`stdlib_api_tool` と project/dependency/package/module helper を
  `mcp_context.rs` へ移動する。
- 親では `include!("mcp_context.rs")` を使い、`call_tool`、completion、search からの既存 private path と
  `config` / `api_doc` / `mcp_io_error` の参照を同じ module namespace に維持する。
- explicit `project_dir` precedence、dependency summary shape、installed package ordering、package API fallback、
  stdlib module filtering/sorting、missing directory behavior は変更しない。
- explicit project directory の project metadata と array fields を contract test で固定する。

## Evidence

- RED: `include!("mcp_context.rs")` を追加した child 不在状態で `E0583` を確認。
- GREEN: `test_project_context_tool_honors_explicit_project_dir`、MCP focused 33 tests、`lsharp-driver` unit 155 件が
  pass。
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings`、専用 target の `cargo check --workspace`、対象 Rust
  2024 `rustfmt --check`、`git diff --check`、docs audit が pass。
- parent は 1412 行から 1240 行、`mcp_context.rs` は 203 行となった。`default_path_delegation` の既知 7 failures
  は origin/main の embedded component/selfhost default-path boundary として今回の移動とは独立に分類する。

## Consequences

MCP の read-only project/package/stdlib context を dispatch、validation、compile/run から独立してレビューできる。
completion/search の shared discovery path と既存 response shape は維持される。残る MCP compile/error/search/tool
implementation の分割、default-path integration blocker、selfhost/native parity、I-01 / I-08 aggregate はこの
partial slice では完了扱いにしない。
