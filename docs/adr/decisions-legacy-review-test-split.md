# ADR: `review.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-docs/src/review.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`review.rs` はレビューチェックポイント生成、ソース位置変換、context 抽出、YAML 出力の
production code と、review summary/metadata/span/YAML/context を確認する 11 件の回帰テストを
同じファイルに保持していた。test-only fixture を分離すると、review production の変更と
DocTools fixture の ownership/review 境界を明確にできる。

## Decision

- `generate_review`、offset/context extraction、YAML serialization、Freshness semantics は変更しない。
- `review::tests` の 7 件を `crates/lsharp-docs/src/review_tests.rs` へ移動する。
- `review::context_tests` の 4 件を `crates/lsharp-docs/src/review_context_tests.rs` へ移動する。
- `review.rs` は path module で既存の `review::tests` / `review::context_tests` namespace を維持する。
- DocTools schema、CLI review surface、metadata diagnostics 経路は同一コミットで変更しない。

## Evidence

- 分離前後の `review::tests` focused gate: 7 passed、`review::context_tests`: 4 passed。
- `cargo test -p lsharp-docs`: 23 passed、doc-tests 0 passed。
- `cargo clippy -p lsharp-docs --all-targets -- -D warnings`、Rust 2024 rustfmt、`git diff --check`: pass。
- `review.rs` は 441 行から 296 行へ、`review_tests.rs` は 97 行、`review_context_tests.rs` は 48 行となった。
- `bash scripts/audit_docs.sh`: エラー 0、警告 0。

## Consequences

review production と DocTools fixture の ownership/review 境界が明確になり、summary と context の
回帰テストを責務別に単独再実行できる。review production の追加責務分割、他の大規模 Rust file、
I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
