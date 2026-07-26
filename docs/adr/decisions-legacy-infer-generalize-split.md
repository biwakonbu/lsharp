# ADR: Infer generalize 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/infer.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [infer unify split](decisions-legacy-infer-unify-split.md)

## Context

`infer.rs` は式・宣言の型推論と、型環境に対する free variable の計算および `TypeScheme` の汎化を
同じ `impl Infer` に保持していた。let 多相や top-level binding で共有される汎化契約を推論本体から独立して
レビューできる境界が必要だった。

## Decision

- `Infer::generalize` を `infer/generalize.rs` へ移動する。
- 親の既存呼び出しは `pub(super)` seam で維持する。
- `TypeEnv` と対象 `Type` の free variable を比較し、environment-bound variable を除外した
  `TypeScheme` を返す既存 semantics は変更しない。

## Evidence

- RED: `mod generalize;` 追加後、child 不在の `E0583` を確認。
- GREEN: `generalize_excludes_environment_free_vars` — environment に束縛された変数を汎化しない契約が pass。
- Package: `RUST_MIN_STACK=33554432 cargo test -p lsharp-types -- --nocapture` — unit 218、integration 117、doc-test 0 が全て pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象 Rust 2024 `rustfmt --check`、`git diff --check` が pass。
- parent は 2061 行から 2047 行へ、child は 19 行となった。

## Consequences

汎化契約の変更範囲を式・宣言推論から独立してレビューできる。既存の内部呼び出しと polymorphism semantics
は維持される。infer の他 production 責務、selfhost/native parity、I-01 / I-08 aggregate はこの partial slice
では完了扱いにしない。
