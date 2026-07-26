# ADR: LSP inline test suite 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-lsp/src/lib.rs`, `crates/lsharp-lsp/src/lib_tests.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-lsp/src/lib.rs` は LSP backend の state、diagnostics、LanguageServer 実装と、788 行の
inline test suite を同じファイルへ保持していた。test-only fixture の変更が production backend の差分へ
混ざり、親ファイルの責務とレビュー範囲を不要に広げていた。

## Decision

- `#[cfg(test)] mod tests` の本体を `crates/lsharp-lsp/src/lib_tests.rs` へ移動する。
- 親では `#[cfg(test)] include!("lib_tests.rs");` を使い、既存の `tests::*` module path、`super::*` の
  private helper access、test fixture の ownership を維持する。
- production の LSP backend、公開再 export、request handling、diagnostic timing behavior は変更しない。
- incremental sync と formatting capability の protocol surface を回帰テストで明示する。

## Evidence

- RED: `include!("lib_tests.rs")` を追加した child 不在状態で `E0583` を確認。
- GREEN: `test_protocol_surface_keeps_incremental_sync_and_formatting` と既存 suite が pass。
- Package: `cargo test -p lsharp-lsp -- --nocapture` — unit 63、main 0、doc-test 0 が全て pass。
- `cargo clippy -p lsharp-lsp --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象
  Rust 2024 `rustfmt --check`、`git diff --check` が pass。
- parent は 1270 行から 504 行、`lib_tests.rs` は 788 行となった。

## Consequences

LSP production backend と test-only fixture を独立してレビューでき、親の変更範囲を縮小できる。既存の
test module path と behavior は維持される。LSP production backend の追加分割、selfhost/native parity、
I-01 / I-08 aggregate はこの partial slice では完了扱いにしない。
