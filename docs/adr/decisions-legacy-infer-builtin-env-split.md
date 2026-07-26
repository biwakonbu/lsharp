# ADR: Infer builtin environment の責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/infer.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [infer test split](decisions-legacy-infer-test-split.md)

## Context

`infer.rs` は Hindley–Milner 型推論、宣言/式推論、module visibility、constraint/trait/kind 処理に加え、
組み込み関数の型環境を 472 行の `Infer::builtin_env` に保持していた。builtin scheme の変更が本体の
推論制御と同じ file に埋まり、runtime helper の型契約を単独でレビューしにくかった。

## Decision

- `Infer::builtin_env` の組み込み operator、string/ref/vector/map/file/argv/root helper と
  Functor/Monad の kind/trait 登録を `infer/builtin_env.rs` へ移動する。
- child は `impl Infer` の `pub(super)` method とし、parent の `infer_program` から既存の
  `self.builtin_env()` 呼び出しを維持する。
- builtin の TypeScheme、fresh variable の生成順序、kind/trait registry の登録順序、公開 `infer`
  module API は変更しない。

## Evidence

- RED: `mod builtin_env;` 追加後、child 不在の `E0583` を確認。
- GREEN: `cargo test -p lsharp-types test_builtin_env_keeps_core_operator_scheme -- --nocapture` — 1 passed。
- Package: `RUST_MIN_STACK=33554432 cargo test -p lsharp-types -- --nocapture` — unit/integration/doc-test が全て pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象 Rust 2024 `rustfmt --check`、`git diff --check` が pass。
- parent は 2789 行から 2319 行へ、child は 475 行となった。

## Consequences

Builtin 型契約を推論制御から独立してレビュー・変更できる。既存の `Infer` API、scheme、kind/trait
登録 semantics は維持される。infer の他 production 責務、selfhost/native parity、I-01 / I-08 aggregate
はこの partial slice では完了扱いにしない。
