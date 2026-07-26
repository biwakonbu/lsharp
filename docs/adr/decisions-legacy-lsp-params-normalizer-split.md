# ADR: LSP params normalizer 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-lsp/src/lib.rs`, `crates/lsharp-lsp/src/params_normalizer.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-lsp/src/lib.rs` は LSP backend の起動・handler・state 管理に加えて、tower-lsp の
request params を正規化する middleware とそのテストも保持していた。params normalizer は独立した
protocol boundary であり、親の責務と同じ変更範囲へ巻き込まれる状態だった。

## Decision

- `params_normalizer` module を `crates/lsharp-lsp/src/params_normalizer.rs` へ移動する。
- `params_normalizer::ParamsNormalizer` の module path と `run_server` からの wiring を維持する。
- `shutdown` など param-less method の `null` / 空 object だけを strip し、non-empty params、request id、
  method forwarding、`Service` の readiness / call semantics は変更しない。
- 既存テストを新 module 内に保持し、protocol normalization boundary の回帰を同じ場所で検証する。

## Evidence

- RED: `mod params_normalizer;` を追加した段階で child 不在の `E0583` を確認。
- GREEN: `test_shutdown_request_with_non_empty_params_is_preserved` と既存 params normalizer tests が pass。
- Package: `cargo test -p lsharp-lsp -- --nocapture` — unit 62、main 0、doc-test 0 が全て pass。
- `cargo clippy -p lsharp-lsp --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象
  Rust 2024 `rustfmt --check`、`git diff --check` が pass。
- parent は 1397 行から 1270 行、`params_normalizer.rs` は 134 行となった。

## Consequences

LSP protocol normalization を backend orchestration から独立してレビュー・変更でき、parent の変更範囲を
縮小できる。公開 API と request behavior は維持される。LSP backend の追加 production 分割、
selfhost/native parity、I-01 / I-08 aggregate はこの partial slice では完了扱いにしない。
