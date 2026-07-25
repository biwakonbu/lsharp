# ADR: `lsharp-lsp/util.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-lsp/src/util.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`util.rs` は LSP の position/offset、symbol usage、parse/type diagnostics、incremental module
diagnostics を実装し、同じファイルに UTF-16 座標、symbol lookup、stable code/range、file URI
override の 12 件の回帰テストを保持していた。test-only fixture を分離すると、LSP utility
production と診断/incremental fixture の ownership/review 境界を明確にできる。

## Decision

- position/offset conversion、symbol lookup、diagnostic mapping、incremental override の
  production semantics と公開 crate API は変更しない。
- `#[cfg(test)] mod tests` の 12 件を
  `crates/lsharp-lsp/src/util_tests.rs` へ移動する。
- `util.rs` は `#[cfg(test)] #[path = "util_tests.rs"] mod tests;` で既存の
  `util::tests` namespace を維持する。
- UTF-16 boundary、LS0101/LS0103/LS1004/LS3102 code、source range、file URI override contract
  は同一コミットで変更しない。

## Evidence

- 分離前後の `util::tests` focused gate: 12 passed。
- `util.rs` は 862 行から 653 行へ、`util_tests.rs` は 206 行となった。
- `cargo test -p lsharp-lsp`: 61 passed、doc-tests 0 passed / 0 failed。
- `cargo clippy -p lsharp-lsp --all-targets -- -D warnings`、Rust 2024 rustfmt、
  `git diff --check`、`bash scripts/audit_docs.sh` は pass。

## Consequences

LSP utility production と診断/incremental fixture の ownership/review 境界が明確になり、12 件の
回帰テストを単独で再実行できる。LSP の他の大規模 file、production の責務分割、I-01 / I-08
aggregate は未完了であるため、TODO の partial slice を維持する。
