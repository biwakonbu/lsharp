# ADR: `rename.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-lsp/src/rename.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`rename.rs` は prepare rename の symbol range 取得と references からの TextEdit 生成という
production code と、symbol/whitespace/function/parameter/typed parameter/let binding を確認する
6 件の回帰テストを同じファイルに保持していた。test-only fixture を分離すると、rename
production の変更と LSP fixture の ownership/review 境界を明確にできる。

## Decision

- `prepare_rename`、`compute_rename_edits`、LSP Range/TextEdit semantics は変更しない。
- `#[cfg(test)] mod tests` の 6 件を `crates/lsharp-lsp/src/rename_tests.rs` へ移動する。
- `rename.rs` は `#[cfg(test)] #[path = "rename_tests.rs"] mod tests;` で既存の `rename::tests` namespace を維持する。
- LSP rename protocol、references collection、position conversion 経路は同一コミットで変更しない。

## Evidence

- 分離前後の `rename::tests` focused gate: 6 passed。
- `cargo test -p lsharp-lsp`: 61 passed、doc-tests 0 passed。
- `cargo clippy -p lsharp-lsp --all-targets -- -D warnings`、Rust 2024 rustfmt、`git diff --check`: pass。
- `rename.rs` は 128 行から 32 行へ、`rename_tests.rs` は 96 行となった。
- `bash scripts/audit_docs.sh`: エラー 0、警告 0。

## Consequences

rename production と LSP fixture の ownership/review 境界が明確になり、6 件の回帰テストを
単独で再実行できる。rename production の追加責務分割、他の大規模 Rust file、I-01 / I-08
aggregate は未完了であるため、TODO の partial slice を維持する。
