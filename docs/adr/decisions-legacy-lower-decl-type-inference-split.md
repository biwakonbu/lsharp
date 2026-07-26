# ADR: lower declaration type-name inference helper 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/decl.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`lower/decl.rs` は宣言 lowering、trait の static dispatch、record/ADT の accessor・constructor、
constraint 生成、function lowering と、式・型変数から表示用の型名を推定する helper 群を同じ
module に保持していた。型名推定は dispatch や code generation の順序を変更せずに独立して
レビューできる責務軸であり、親 module のサイズと変更衝突を抑えるため分離候補となった。

## Decision

- `infer_*type_name*`、型変数名の収集、builtin/let/argument の戻り型推定を
  `lower/decl/type_inference.rs` へ移動する。
- child module も `impl Lower` として実装し、`Lower` の private field、既存の helper 呼び出し、
  `lower::decl` 内の method path を変更しない。
- 親 `decl.rs` には宣言 dispatch、accessor/constructor/constraint 生成、function lowering を残す。
- 型推論の意味論、trait/ADT dispatch、公開 API、Rust/native parity の判定は変更しない。

## Evidence

- RED: child file がない状態で `lower::decl` focused test を実行し、module include の `E0583`
  を確認した。
- `RUST_MIN_STACK=33554432 cargo test -q -p lsharp-ir lower:: --lib`: 167 passed / 0 failed。
- `RUST_MIN_STACK=33554432 cargo test -q -p lsharp-ir --lib`: 282 passed / 1 failed。唯一の失敗は
  既存 `incremental_analysis_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`
  の `IntentSource.ls` における `vector-push-pair-rooted-v3` 未定義診断であり、今回の分離とは無関係。
- `decl.rs` は 692 行から 486 行へ、`decl/type_inference.rs` は 215 行となった。
- `cargo clippy -q -p lsharp-ir --lib -- -D warnings`、`cargo check --workspace --quiet`、対象 files
  の Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

型名推定の helper を単独で確認でき、宣言 lowering 本体との責務境界と変更衝突の範囲が明確になった。
既存の `Lower` 内部 API と production semantics は維持される。一方、lower 全体の責務分割、
Rust/native parity、I-01 / I-08 aggregate は未完了であり、TODO に残す。
