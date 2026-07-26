# ADR: Infer error 型の責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/infer.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [infer test split](decisions-legacy-infer-test-split.md)

## Context

`infer.rs` は型推論本体と `TypeError` / `TypeErrorCode`、公開診断 code/span、Display/Error 実装を同じ
file に保持していた。診断契約は parser/type inference の複数箇所から利用されるため、推論アルゴリズムと
独立してレビューできる境界が必要だった。

## Decision

- `TypeError`、`TypeErrorCode`、stable code/span、Display、Error trait の実装を
  `infer/error.rs` へ移動する。
- parent の `pub use error::{TypeError, TypeErrorCode};` で既存の `infer::TypeError` / `infer::TypeErrorCode`
  公開 path を維持する。
- 全 variant、診断 code、span、表示文、Error semantics は変更しない。

## Evidence

- RED: `mod error;` 追加後、child 不在の `E0583` を確認。
- GREEN: `cargo test -p lsharp-types infer_error_types_remain_exported_from_infer_module -- --nocapture` — 1 passed。
- Package: `RUST_MIN_STACK=33554432 cargo test -p lsharp-types -- --nocapture` — unit 216、integration 117、doc-test 0 が全て pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象 Rust 2024 `rustfmt --check`、`git diff --check` が pass。
- parent は 2319 行から 2168 行へ、child は 155 行となった。

## Consequences

診断型の変更範囲を推論アルゴリズムから独立してレビューできる。既存の公開 path と診断契約は維持される。
infer の他 production 責務、selfhost/native parity、I-01 / I-08 aggregate はこの partial slice では完了扱いにしない。
