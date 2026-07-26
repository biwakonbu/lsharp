# ADR: driver MCP validation input 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-driver/src/mcp_server.rs`, `crates/lsharp-driver/src/mcp_validation.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-driver/src/mcp_server.rs` は MCP protocol、tool registry、各 tool 実装に加えて、
`lsharp_validate` の source/file/manifest/manifest_file 入力選択と validation graph の parse/report を保持していた。
validation input boundary は他の tool 実装から独立しており、入力の排他性と診断 semantics を一箇所で追跡できる。

## Decision

- `validate_tool`、validation input selector、source/manifest graph parser、`include_manifest` option helper を
  `mcp_validation.rs` へ移動する。
- 親では `include!("mcp_validation.rs")` を使い、`call_tool` からの既存 private path、`source_argument` / `mcp_io_error`
  参照、validation report shape を同じ module namespace に維持する。
- source/file/manifest/manifest_file の exactly-one 要件、source/manifest parse diagnostics、canonical manifest
  inclusion、unsupported manifest error は変更しない。
- 入力がない場合に parse を実行せず、既存の日本語診断を返す contract test を追加する。

## Evidence

- RED: `include!("mcp_validation.rs")` を追加した child 不在状態で `E0583` を確認。
- GREEN: `test_validation_graph_rejects_missing_input_before_parsing`、validation focused 15 tests、
  `lsharp-driver` unit 154 件が pass。
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings`、専用 target の `cargo check --workspace`、
  対象 Rust 2024 `rustfmt --check`、`git diff --check`、docs audit が pass。
- parent は 1476 行から 1412 行、`mcp_validation.rs` は 75 行となった。`default_path_delegation` の既知 7 failures
  は origin/main の embedded component/selfhost default-path boundary として今回の移動とは独立に分類する。

## Consequences

MCP validation の入力契約・diagnostics・canonical report を tool dispatch から独立してレビューできる。
source/manifest parity と private caller path は維持される。残る MCP tool implementation の分割、default-path
integration blocker、selfhost/native parity、I-01 / I-08 aggregate はこの partial slice では完了扱いにしない。
