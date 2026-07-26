# ADR: lower application の Vector operation 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr/application_ref_vector.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

Ref Cell 分離後の `application_ref_vector.rs` は、Ref dispatch の薄い wrapper と Vector の
allocation/access/mutation/reallocation を同じ module に保持していた。Vector は tagged pointer、
固定 header、root lease、capacity growth を持つ独立した heap responsibility であり、Ref の
固定 layout と同じ match に残す必要はない。

## Decision

- `vector-new`、`vector-length`、`vector-get`、`vector-set`、`vector-push` の lowering を
  `expr/application_vector.rs` の `Lower::lower_app_vector` へ移動する。
- `application_ref_vector.rs` は既存の `lower_app_ref_vector` method path を維持し、Ref child と
  Vector child を順番に dispatch する互換 wrapper とする。
- Vector の `HEAP_TAG_VECTOR`、16-byte header、capacity growth、memory.copy、root push/pop、
  tagged pointer、opcode emission の順序は変更しない。

## Evidence

- RED: `application_vector` child file がない状態で existing `vector-push` focused test を実行し、
  module include の `E0583` を確認した。
- `RUST_MIN_STACK=33554432 cargo test -q -p lsharp-ir lower:: --lib`: 167 passed。
- `rooting_calls` 28 passed、`rooting_loops` 8 passed、`cargo test -q -p lsharp-wasm --test wasmgc_probe`: 101 passed。
- `application_ref_vector.rs` は 340 行から 19 行へ、Vector child は 327 行となった。
- `RUST_MIN_STACK=33554432 cargo test -q -p lsharp-ir --lib`: 282 passed / 1 failed。唯一の失敗は
  既存 `incremental_analysis_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`
  の `IntentSource.ls` における `vector-push-pair-rooted-v3` 未定義診断であり、今回の分離とは無関係。
- `cargo clippy -q -p lsharp-ir --lib -- -D warnings`、`cargo check --workspace --quiet`、対象 files
  の Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

Vector の memory/rooting 契約を単独で確認でき、Ref dispatch と collection lowering の境界が明確に
なった。既存の内部 method path と runtime semantics は維持される。一方、HashMap の追加分割、
lower expr 全体、Rust/native parity、I-01 / I-08 aggregate は未完了であり TODO に残す。
