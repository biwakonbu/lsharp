# ADR: Infer expr 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/infer.rs`, `crates/lsharp-types/src/infer/expr.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [infer generalize split](decisions-legacy-infer-generalize-split.md)

## Context

`infer.rs` は式の型推論、record/pattern の補助推論、宣言推論、型環境・診断 helper を同じ
`impl Infer` に保持していた。式推論を独立してレビューできる境界がなく、以後の責務分離で
変更範囲とコンフリクトを抑えにくかった。

## Decision

- `Infer::infer_expr` を `infer/expr.rs` へ移動し、親からの内部呼び出しは `pub(super)` seam で維持する。
- `infer_record_lit`、`infer_field_access`、`infer_record_update`、`infer_pattern`、`bind_pattern`、`lit_type`
  と式推論本体を child module に集約する。
- `resolve_type_expr`、`resolve_qualified_name`、`detect_alias_name`、`record_expr_type`、`instantiate` は
  child が利用する最小の `pub(super)` seam として親に残す。
- 既存の式/let/lambda/application/match/do/annotation/computation、record/constructor/pattern、
  unification/generalization/diagnostic semantics は変更しない。

## Evidence

- RED: `mod expr;` を追加した段階で child 不在の `E0583` を確認。
- GREEN: `infer_expr_preserves_if_branch_unification` — `if` の then/else branch が `Int` に統一される契約が pass。
- Package: `RUST_MIN_STACK=33554432 cargo test -p lsharp-types -- --nocapture` — unit 219、integration 117、doc-test 0 が全て pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象 Rust 2024 `rustfmt --check`、`git diff --check` が pass。
- parent は 2047 行から 1414 行、child は 644 行となった。

## Consequences

式推論と record/pattern helper の変更を宣言推論・型環境 helper から独立してレビューできる。親の内部 API
は crate 内に限定され、公開 API と推論 semantics は維持される。`infer_program`/宣言責務、selfhost/native
parity、I-01 / I-08 aggregate はこの partial slice では完了扱いにしない。
