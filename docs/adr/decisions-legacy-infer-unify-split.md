# ADR: Infer unify 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/infer.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [infer error split](decisions-legacy-infer-error-split.md)

## Context

`infer.rs` は式・宣言の型推論と、unification、代入合成、occurs check、Int/heap compatibility の実装を
同じ `impl Infer` に保持していた。unification は Algorithm W の複数箇所から呼ばれる独立した契約であり、
診断型の分離後は推論本体と別にレビューできる境界が必要だった。

## Decision

- `unify`、`int_heap_compatible`、occurs-check 付き `bind_var` を `infer/unify.rs` へ移動する。
- `unify` は `pub(super)` とし、親の既存呼び出しと `infer` 内 test module からの内部 seam を維持する。
- 関数・型適用・record の再帰的 unify、代入合成、`TypeError` variant、`global_subst` 更新の semantics は変更しない。

## Evidence

- RED: `mod unify;` 追加後、child 不在の `E0583` を確認。
- GREEN: `unify_property_tests` — occurs-check 診断境界と既存対称性 property の 2 件が pass。
- Package: `RUST_MIN_STACK=33554432 cargo test -p lsharp-types -- --nocapture` — unit 217、integration 117、doc-test 0 が全て pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象 Rust 2024 `rustfmt --check`、`git diff --check` が pass。
- parent は 2168 行から 2061 行へ、child は 118 行となった。

## Consequences

unification と occurs-check の変更範囲を式・宣言推論から独立してレビューできる。既存の内部呼び出しと
diagnostic contract は維持される。infer の他 production 責務、selfhost/native parity、I-01 / I-08 aggregate
はこの partial slice では完了扱いにしない。
