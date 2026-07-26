# ADR: lower application の scalar string operation 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr/application_scalar.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`application_scalar.rs` は boolean/arithmetic、print、allocator、文字列操作を一つの
`lower_app_scalar` match に保持していた。文字列の length/equality/conversion は backend
分岐と runtime import 呼び出しを含む独立した責務軸であり、親 module のサイズと変更衝突を
抑えるため先に分離できる。

## Decision

- `string-length`、`string-eq`、`int-to-string` の lowering を
  `expr/application_strings.rs` の `Lower::lower_app_string_scalar` へ移動する。
- 親の `lower_app_scalar` は child helper を先に呼び、対象外の scalar operation と、rooting/GC
  が密接な `string-char-at`、`substring`、`string-concat` は既存の場所に残す。
- WasmGC array、linear-memory import、rooting、diagnostic span、既存 method visibility と
  opcode emission の順序は変更しない。

## Evidence

- RED: `application_strings` child file がない状態で既存 string lowering focused test を実行し、
  module include の `E0583` を確認した。
- `RUST_MIN_STACK=33554432 cargo test -q -p lsharp-ir lower:: --lib`: 167 passed。
- `rooting_calls` 28 passed、`wasm_gc_and_roots` 14 passed、`cargo test -q -p lsharp-wasm --test wasmgc_probe`: 101 passed。
- `application_scalar.rs` は 606 行から 475 行へ、child は 158 行となった。
- `RUST_MIN_STACK=33554432 cargo test -q -p lsharp-ir --lib`: 282 passed / 1 failed。唯一の失敗は
  既存 `incremental_analysis_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`
  の `IntentSource.ls` における `vector-push-pair-rooted-v3` 未定義診断であり、今回の分離とは無関係。
- `cargo clippy -q -p lsharp-ir --lib -- -D warnings`、`cargo check --workspace --quiet`、対象 files
  の Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

文字列の scalar operation を単独でレビューでき、親の boolean/arithmetic と host/runtime
bridge の境界が明確になった。既存の `Lower` 内部 API、opcode emission、runtime semantics は
維持される。残る string-char-at/substring/string-concat と lower expr 全体の分割、Rust/native
parity、I-01 / I-08 aggregate は未完了であり、TODO に残す。
