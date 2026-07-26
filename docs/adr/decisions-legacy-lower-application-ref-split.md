# ADR: lower application の Ref operation 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr/application_ref_vector.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`application_ref_vector.rs` は Ref Cell の allocation/read/write と、可変 Vector、HashMap
dispatch を同じ `lower_app_ref_vector` match に保持していた。Ref Cell は root lease、tagged
pointer、固定 layout を持つ独立した heap responsibility であり、vector/map の可変 collection
logic と分けてレビューできる。

## Decision

- `ref-new`、`ref-get`、`ref-set` の lowering を `expr/application_ref.rs` の
  `Lower::lower_app_ref` へ移動する。
- `lower_app_ref_vector` は child helper を先に dispatch し、Vector と既存 HashMap fallback の
  lowering を保持する。
- Ref Cell の `HEAP_TAG_REF`、16-byte allocation、root push/pop、untag、load/store opcode、
  diagnostic span は変更しない。

## Evidence

- RED: `application_ref` child file がない状態で existing `ref-new` focused test を実行し、
  module include の `E0583` を確認した。
- `RUST_MIN_STACK=33554432 cargo test -q -p lsharp-ir lower:: --lib`: 167 passed。
- `module_and_lambdas` 10 passed、`rooting_calls` 28 passed、`cargo test -q -p lsharp-wasm --test wasmgc_probe`: 101 passed。
- `application_ref_vector.rs` は 414 行から 340 行へ、child は 101 行となった。
- `RUST_MIN_STACK=33554432 cargo test -q -p lsharp-ir --lib`: 282 passed / 1 failed。唯一の失敗は
  既存 `incremental_analysis_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`
  の `IntentSource.ls` における `vector-push-pair-rooted-v3` 未定義診断であり、今回の分離とは無関係。
- `cargo clippy -q -p lsharp-ir --lib -- -D warnings`、`cargo check --workspace --quiet`、対象 files
  の Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

Ref Cell の rooting/layout 契約を単独で確認でき、Vector/HashMap lowering との責務境界が明確に
なった。既存の `Lower` 内部 API と runtime semantics は維持される。一方、Vector/HashMap の
追加分割、lower expr 全体、Rust/native parity、I-01 / I-08 aggregate は未完了であり TODO に残す。
