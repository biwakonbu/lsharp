# ADR: `format.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-lsp/src/format.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`format.rs` は S 式ソースの whitespace、comment、string literal、indentation を整形する
production code と、formatter の deterministic な出力を確認する 7 件の回帰テストを同じ
ファイルに保持していた。test-only fixture を分離すると、formatter production の変更と
LSP formatter fixture の ownership/review 境界を明確にできる。

## Decision

- `format_source` と indentation、comment、string literal、newline の semantics は変更しない。
- `#[cfg(test)] mod tests` の 7 件を `crates/lsharp-lsp/src/format_tests.rs` へ移動する。
- `format.rs` は `#[cfg(test)] #[path = "format_tests.rs"] mod tests;` で既存の
  `format::tests` namespace を維持する。
- LSP formatting protocol、parser integration、source-preservation 経路は同一コミットで変更しない。

## Evidence

- 分離前後の `format::tests` focused gate: 7 passed。
- `cargo test -p lsharp-lsp`: 61 passed、doc-tests 0 passed。
- `cargo clippy -p lsharp-lsp --all-targets -- -D warnings`、Rust 2024 rustfmt、`git diff --check`: pass。
- `format.rs` は 236 行から 159 行へ、`format_tests.rs` は 77 行となった。
- `bash scripts/audit_docs.sh`: エラー 0、警告 0。

## Consequences

formatter production と LSP fixture の ownership/review 境界が明確になり、7 件の回帰テストを
単独で再実行できる。formatter production の追加責務分割、他の大規模 Rust file、I-01 / I-08
aggregate は未完了であるため、TODO の partial slice を維持する。
