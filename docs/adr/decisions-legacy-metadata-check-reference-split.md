# ADR: `metadata_check.rs` 参照収集 helper の責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-types/src/metadata_check.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`, `decisions-legacy-metadata-check-test-split.md`

## Context

`metadata_check.rs` は metadata の診断・legacy invariant の型検査・test 生成に加え、式の変数参照、lexical scope、
ドキュメント識別子、組み込み名の判定 helper まで同じファイルに保持していた。既存の test module 分離後も production
parent は 846 行あり、metadata checker の診断ロジックと再利用可能な参照収集ロジックの変更境界が混在していた。

## Decision

- `span_contains`、`:doc` 識別子抽出、式の参照収集、scoped reference 収集、pattern binder 収集、builtin 判定を
  `crates/lsharp-types/src/metadata_check/references.rs` へ移動する。
- helper は `pub(super)` とし、親 module の private `use` で従来の名前を維持する。`metadata_check::tests` と
  `metadata_check::test_generation_tests` の namespace、crate の公開 API、diagnostic/test 生成 semantics は変更しない。
- diagnostics、legacy invariant probe、property smoke profile、generated test の責務は `metadata_check.rs` に残す。
- metadata parser/checker の追加責務分割、I-01 / I-08 aggregate、Rust-free/native parity は後続タスクとして残す。

## Evidence

- 分離前後の `cargo test -p lsharp-types`: unit 209 件、integration/property 49 件、doc-tests 0 件が全て pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`: pass。
- `metadata_check.rs` は 846 行から 601 行へ、`metadata_check/references.rs` は 255 行となった。
- 対象 files の Rust 2024 rustfmt、`git diff --check`: pass。

## Consequences

metadata checker の診断本体と参照解析 helper を独立してレビュー・変更できる。helper は crate 内 private boundary のまま
なので既存利用側の変更は不要である。metadata checker の全 production 責務分割、他の大規模 Rust file、I-01 / I-08 aggregate
は未完了であり、TODO の verified partial slice を維持する。
