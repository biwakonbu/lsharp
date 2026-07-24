# ADR: `analysis.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-lsp/src/analysis.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`analysis.rs` は LSP hover、symbol range、doc lookup、signature rendering の production code と、top-level function の type/doc hover を確認する 1 件の回帰テストを同じファイルに保持していた。test-only fixture を分離すると、analysis production の変更と hover fixture の ownership/review 境界を明確にできる。

## Decision

- `hover`、symbol/doc lookup、signature rendering と LSP hover semantics は変更しない。
- `#[cfg(test)] mod tests` の 1 件を `crates/lsharp-lsp/src/analysis_tests.rs` へ移動する。
- `analysis.rs` は `#[cfg(test)] #[path = "analysis_tests.rs"] mod tests;` で既存の `analysis::tests` namespace を維持する。
- LSP hover protocol、type inference 呼び出し、background diagnostics 経路は同一コミットで変更しない。

## Evidence

- 分離前後の `analysis::tests` focused gate: 1 passed。
- `cargo test -p lsharp-lsp`: 61 passed、doc-tests 0 passed。
- `cargo clippy -p lsharp-lsp --all-targets -- -D warnings`、Rust 2024 rustfmt、`git diff --check`: pass。
- `analysis.rs` は 103 行から 80 行へ、`analysis_tests.rs` は 23 行となった。
- `bash scripts/audit_docs.sh`: エラー 0、警告 0。

## Consequences

hover production と LSP fixture の ownership/review 境界が明確になり、1 件の回帰テストを単独で再実行できる。LSP 統合部の追加責務分割、他の大規模 Rust file、I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
