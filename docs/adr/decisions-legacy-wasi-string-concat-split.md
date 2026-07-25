# ADR: WASI string-concat helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-wasi-read-file-split.md`

## Context

`wasi.rs` は WASI preview1 / preview2、GC runtime、I/O helper を一つの production
module に持つ。`__string_concat` は二つの String object の length header を読み、allocator
で `8 + total_len` bytes を確保し、tag・length と二段の `memory.copy` で新しい String
object を作る独立した code-emission 責務だが、他の helper と同じ parent にあり、WASI
変更の衝突単位を増やしていた。

## Decision

- `emit_string_concat_func` を `crates/lsharp-wasm/src/wasi/string_concat.rs`（102 行）へ
  移動する。これにより WASI production parent は 3639 行から 3543 行へ縮小する。
- Preview1 / Preview2 の helper registration は `string_concat` module 経由にし、function
  ordering、allocator function index、String length-header、tagged-pointer i64 return
  contract は維持する。
- 既存の二つの `memory.copy`、tagged pointer helper、空文字・nested concat の semantics
  は変更しない。`string_concat_tests.rs` の module seam test で helper body の登録を固定する。

## Evidence

- RED: 空の `string_concat` module に対する seam test が `emit_string_concat_func` の
  unresolved import／registration で失敗。
- GREEN: `cargo test -p lsharp-wasm --lib string_concat_module_emits_string_concat_function_body -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --test e2e test_e2e_string_concat -- --nocapture`
  （4 passed、通常・nested code location・empty・nested summary）。
- `cargo test -p lsharp-wasm --test e2e test_e2e_string_concat_empty -- --nocapture`、
  `test_e2e_string_concat_nested_summary_chain`、`test_e2e_string_print_string_concat`、
  `test_e2e_int_to_string_concat`（各 1 passed）。
- `cargo test -p lsharp-wasm --lib`（96 tests のうち 95 passed、既存の
  `RootLifetime::RootSetWithoutActiveSlot` failure 1 件）。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`

## Boundary

これは WASI `string-concat` code-emission の責務分離だけを扱う。String の全操作、長大な
入力、standalone/streamed runtime、Rust/native selfhost parity、dynamic memory layout、全
公開 command、Mac Apple Silicon / Linux x86_64 native stage0 の完了を意味しない。既存の
`vector-push-pair-rooted-v3` selfhost fixture failure、root lifetime checker の既知 failure、
package-wide test-only lint debt は今回の差分外として残る。
