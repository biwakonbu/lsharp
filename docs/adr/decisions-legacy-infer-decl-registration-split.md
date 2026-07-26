# ADR: Infer declaration / registration 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/infer.rs`, `crates/lsharp-types/src/infer/decl.rs`, `crates/lsharp-types/src/infer/registration.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [infer expr split](decisions-legacy-infer-expr-split.md)

## Context

`infer.rs` はプログラム全体の orchestration、型/trait/constraint 登録、nested module、defn の二段階推論を
同じ `impl Infer` に保持していた。式推論を分離した後も親は 1414 行あり、宣言処理と登録処理の変更が同じ
ファイルへ集中していた。

## Decision

- `infer_program`、`register_nested_module_types`、`infer_decl_functions`、signature helper、`infer_defn` を
  `infer/decl.rs` に移動する。
- ADT、record、type alias、constrained type、trait、impl の登録責務を `infer/registration.rs` に移動する。
- `infer_program` の公開 API、宣言順序、nested module の qualified name、2-pass defn inference、constructor/
  accessor/type scheme、trait/default impl、constraint registration の semantics は変更しない。
- child module 間で必要な登録・constraint・defn 呼び出しだけを `pub(super)` seam として明示する。

## Evidence

- RED: `mod decl;` と `mod registration;` を追加した段階で、それぞれ child 不在の `E0583` を確認。
- GREEN: `infer_declaration_pipeline_preserves_definition_order`、`infer_registration_preserves_record_constructor_scheme` が pass。
- Package: `RUST_MIN_STACK=33554432 cargo test -p lsharp-types -- --nocapture` — unit 221、integration 117、doc-test 0 が全て pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象 Rust 2024 `rustfmt --check`、`git diff --check` が pass。
- parent は 1414 行から 474 行、`infer/decl.rs` は 458 行、`infer/registration.rs` は 489 行となった。

## Consequences

宣言 orchestration、関数定義推論、型登録の変更範囲を独立してレビューでき、全 production module が 500〜800 行の責務単位に収まる。公開 API と型推論 semantics は維持される。selfhost/native parity、I-01 / I-08 aggregate はこの partial slice では完了扱いにしない。
