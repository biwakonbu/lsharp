# ADR: lower application の heap-backed string operation 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr/application_scalar.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

前段の分離後も `application_scalar.rs` には、linear-memory の rooting と WasmGC array
操作を含む `string-char-at`、`substring`、`string-concat` が残っていた。これらは
boolean/arithmetic や print、`__alloc` dispatch とは異なる heap-backed string の責務軸であり、
同じ match に置き続けると lower expression の変更衝突とレビュー範囲が広がる。

## Decision

- `string-char-at`、`substring`、`string-concat` の lowering を
  `expr/application_string_heap.rs` の `Lower::lower_app_string_heap` へ移動する。
- `lower_app_scalar` は既存の `lower_app_string_scalar` に続いて child helper を先に dispatch
  し、残りの scalar operation と `ref-new` の fallback を保持する。
- linear-memory root push/pop、WasmGC array type、substring range guard、allocator/import
  lookup、opcode の順序、diagnostic span は変更しない。

## Evidence

- RED: `application_string_heap` child file がない状態で string-concat の existing focused test
  を実行し、module include の `E0583` を確認した。
- `RUST_MIN_STACK=33554432 cargo test -q -p lsharp-ir lower:: --lib`: 167 passed。
- `rooting_calls` 28 passed、`wasm_gc_and_roots` 14 passed、`cargo test -q -p lsharp-wasm --test wasmgc_probe`: 101 passed。
- `application_scalar.rs` は 475 行から 142 行へ、heap child は 359 行となった。
- `RUST_MIN_STACK=33554432 cargo test -q -p lsharp-ir --lib`: 282 passed / 1 failed。唯一の失敗は
  既存 `incremental_analysis_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`
  の `IntentSource.ls` における `vector-push-pair-rooted-v3` 未定義診断であり、今回の分離とは無関係。
- `cargo clippy -q -p lsharp-ir --lib -- -D warnings`、`cargo check --workspace --quiet`、対象 files
  の Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

6つの heap/string builtin（scalar child の length/equality/conversion と heap child の
char-at/substring/concat）が責務別 module に分かれ、`application_scalar.rs` は非文字列 scalar
dispatch に集中する。既存の `Lower` 内部 API と runtime semantics は維持される。一方、
lower expr 全体の追加分割、Rust/native parity、I-01 / I-08 aggregate は未完了であり TODO に残す。
