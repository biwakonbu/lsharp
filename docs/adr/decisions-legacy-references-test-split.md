# ADR: `references.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-lsp/src/references.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`references.rs` は LSP の定義・使用箇所収集と参照範囲生成の production code と、関数、
parameter、let binding、annotation/field access の 7 件の回帰テストを同じファイルに保持していた。
test-only fixture を分離すると、references production の変更と LSP fixture の ownership/review
境界を明確にできる。

## Decision

- `find_references` と declaration/use range、include-declaration semantics は変更しない。
- `#[cfg(test)] mod tests` の 7 件を `crates/lsharp-lsp/src/references_tests.rs` へ移動する。
- `references.rs` は `#[cfg(test)] #[path = "references_tests.rs"] mod tests;` で既存の
  `references::tests` namespace を維持する。
- LSP references protocol、symbol collection、position conversion 経路は同一コミットで変更しない。

## Evidence

- 分離前後の `references::tests` focused gate: 7 passed。
- `cargo test -p lsharp-lsp`: 61 passed、doc-tests 0 passed。
- `cargo clippy -p lsharp-lsp --all-targets -- -D warnings`、Rust 2024 rustfmt、`git diff --check`: pass。
- `references.rs` は 143 行から 59 行へ、`references_tests.rs` は 84 行となった。
- `bash scripts/audit_docs.sh`: エラー 0、警告 0。

## Consequences

references production と LSP fixture の ownership/review 境界が明確になり、7 件の回帰テストを
単独で再実行できる。references production の追加責務分割、他の大規模 Rust file、I-01 / I-08
aggregate は未完了であるため、TODO の partial slice を維持する。
