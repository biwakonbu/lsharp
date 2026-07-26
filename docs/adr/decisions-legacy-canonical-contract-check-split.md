# ADR: canonical metadata contract checker の責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/canonical_contract_check.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [I-01](../../ISSUES.md#i-01), [I-08](../../ISSUES.md#i-08)

## Context

`canonical_contract_check.rs` は canonical `:assert` / `:case` / `:property` の非空性・
静的 vacuity 判定と、synthetic probe を使った HM 型検査を同じ 794 行の file に保持していた。
両者は入力契約と診断型を共有するが、実装上は独立した責務であり、metadata checker の次の変更時に
非空性判定と型推論 probe の境界を追いにくかった。

## Decision

- `canonical_contract_check.rs` は module facade と crate-private re-export だけに縮小する。
- `canonical_contract_check/non_vacuity.rs` に canonical form の空集合、binder、case count、静的
  Bool/Int 判定を移動する。
- `canonical_contract_check/types.rs` に assertion / case / property の synthetic probe、
  HM 推論、型診断を移動する。
- `metadata_check.rs` の既存 `crate::canonical_contract_check::*` 導線、可視性、診断文面、
  probe の lexical scope は変更しない。空 `Program` を通す module seam test を追加する。

## Evidence

- RED: child module 宣言と seam test を先に追加し、child file 不在の `E0583` を確認した。
- GREEN: `canonical_contract_check_modules_preserve_empty_program_contract` が pass。
- Regression: `cargo test -p lsharp-types --test metadata_contract_check -- --nocapture` — 30 passed。
- Package: `RUST_MIN_STACK=33554432 cargo test -p lsharp-types -- --nocapture` — unit 214、
  integration 117、doc-test 0 が全て pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、`cargo check --workspace`、
  対象 Rust 2024 `rustfmt --check`、`git diff --check`、`bash scripts/audit_docs.sh` が pass。
- parent は 794 行から 11 行へ縮小し、`non_vacuity.rs` は 370 行、`types.rs` は 428 行となった。

## Consequences

非空性/vacuity と synthetic HM probe の変更を独立に review・検証できる。既存の crate-private API、
metadata checker の診断契約、production semantics は維持する。canonical checker の追加分割、
selfhost/native parity、I-01 / I-08 aggregate、両 target の runtime evidence は未完了である。
