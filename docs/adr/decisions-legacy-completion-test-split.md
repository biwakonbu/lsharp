# ADR: `completion.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-lsp/src/completion.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`completion.rs` は LSP completion の prefix/import context 判定と CompletionItem builder という production code と、keyword/function completion および import module candidate の 2 件の回帰テストを同じファイルに保持していた。test-only fixture を分離すると、completion production の変更と LSP fixture の ownership/review 境界を明確にできる。

## Decision

- `complete`、prefix/import 判定、CompletionItem の kind/insert text semantics は変更しない。
- `#[cfg(test)] mod tests` の 2 件を `crates/lsharp-lsp/src/completion_tests.rs` へ移動する。
- `completion.rs` は `#[cfg(test)] #[path = "completion_tests.rs"] mod tests;` で既存の `completion::tests` namespace を維持する。
- LSP protocol、completion result schema、background diagnostics 経路は同一コミットで変更しない。

## Evidence

- 分離前後の `completion::tests` focused gate: 2 passed。
- `cargo test -p lsharp-lsp`: 61 passed、doc-tests 0 passed。
- `cargo clippy -p lsharp-lsp --all-targets -- -D warnings`、Rust 2024 rustfmt、`git diff --check`: pass。
- `completion.rs` は 130 行から 97 行へ、`completion_tests.rs` は 33 行となった。
- `bash scripts/audit_docs.sh`: エラー 0、警告 0。

## Consequences

completion production と LSP fixture の ownership/review 境界が明確になり、2 件の回帰テストを単独で再実行できる。LSP 統合部の追加責務分割、他の大規模 Rust file、I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
