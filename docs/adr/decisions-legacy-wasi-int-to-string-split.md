# ADR: WASI int-to-string helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-wasi-print-string-split.md`

## Context

`wasi.rs` は WASI preview1 / preview2、GC runtime、I/O helper を一つの production module
に持つ。`__int_to_string` は i64 を scratch buffer `BUF_END` の末尾から十進化し、allocator
で String object を確保して tag・length・payload を書き、tagged i64 handle を返す独立した
code-emission 責務だが、他の helper と同じ parent にあり、WASI 変更の衝突単位を増やしていた。

## Decision

- `emit_int_to_string_func` を `crates/lsharp-wasm/src/wasi/int_to_string.rs`（161 行）へ
  移動する。これにより WASI production parent は 3392 行から 3237 行へ縮小する。
- Preview1 / Preview2 の helper registration は `int_to_string` module 経由にし、function
  ordering、allocator function index、scratch `BUF_END`、`8 + str_len` allocation、String
  object layout、tagged-pointer i64 return contract は維持する。
- parent の `BUF_END` と `emit_tagged_pointer_from_i64_local` を明示 import し、数値変換の
  zero/negative/unsigned-division、payload `memory.copy` semantics は変更しない。
  `int_to_string_tests.rs` の module seam test で helper body の登録を固定する。

## Evidence

- RED: 空の `int_to_string` module に対する seam test が `emit_int_to_string_func` の
  unresolved import で失敗。
- GREEN: `cargo test -p lsharp-wasm --lib int_to_string_module_emits_int_to_string_function_body -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --test e2e test_e2e_int_to_string -- --nocapture`
  （5 passed: positive、zero、negative、large、string-concat）。
- `cargo test -p lsharp-wasm --lib`（99 tests のうち 98 passed、既存の
  `RootLifetime::RootSetWithoutActiveSlot` failure 1 件）。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`

## Boundary

これは WASI `int-to-string` code-emission の責務分離だけを扱う。i64 最小値の独立 runtime
証跡、native helper ABI、全 backend/native selfhost parity、dynamic memory layout、全公開
command、Mac Apple Silicon / Linux x86_64 native stage0 の完了を意味しない。既存の
`vector-push-pair-rooted-v3` selfhost fixture failure、root lifetime checker の既知 failure、
package-wide test-only lint debt は今回の差分外として残る。
