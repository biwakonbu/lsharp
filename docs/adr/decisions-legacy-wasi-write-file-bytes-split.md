# ADR: WASI write-file-bytes helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-wasi-file-exists-split.md`

## Context

`wasi.rs` は WASI preview1 / preview2、GC runtime、I/O helper を一つの production
module に持つ。`__write_file_bytes` は Vector の i64 要素を下位 8 bit の raw bytes へ
詰め、allocator で一時 buffer を確保し、preopened directory の `path_open`、`fd_write`
および `fd_close` を呼ぶ独立した code-emission 責務だが、String 用 write helper と同じ
parent にあり、WASI I/O 変更の衝突単位を増やしていた。

## Decision

- `emit_write_file_bytes_func` を `crates/lsharp-wasm/src/wasi/write_file_bytes.rs`
  （157 行）へ移動する。これにより WASI production parent は 4166 行から 4013 行へ縮小する。
- Preview1 の helper registration は `write_file_bytes` module 経由にし、function ordering、
  import/index、Vector layout、preopened dirfd `3`、fd scratch `280`、iovec scratch
  `352`/`360`、i64 return contract は維持する。
- Vector の低位 byte packing、`path_open` errno の fail-closed、`fd_close` errno の
  return 保持、および既存の allocation/rooting 境界は変更しない。
- `write_file_bytes_tests.rs` の module seam test で helper body の登録を固定する。

## Evidence

- RED: 空の `write_file_bytes` module に対する seam test が
  `emit_write_file_bytes_func` の unresolved import で失敗。
- GREEN: `cargo test -p lsharp-wasm --lib write_file_bytes_module_emits_write_file_bytes_function_body -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib wasi::tests::test_wasi_write_file_bytes_writes_vector_low_bytes -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib wasi::tests::test_wasi_write_helpers_preserve_fd_close_errno -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib wasi::tests::test_wasi_file_helpers_preserve_path_open_errno -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --test e2e test_e2e_selfhost_wasmemit_write_file_bytes_preserves_fd_close_errno -- --nocapture`
  （1 passed、51.25 秒）。
- `cargo test -p lsharp-wasm --lib`（93 tests のうち 92 passed、既存の
  `RootLifetime::RootSetWithoutActiveSlot` failure 1 件）。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`

## Boundary

これは WASI `write-file-bytes` code-emission の責務分離だけを扱う。I/O の全 errno/path
semantics、partial `fd_write` の standalone runtime、Rust/native selfhost parity、dynamic
memory layout、全公開 command、Mac Apple Silicon / Linux x86_64 native stage0 の完了を
意味しない。既存の `vector-push-pair-rooted-v3` selfhost fixture failure、root lifetime
checker の既知 failure、package-wide test-only lint debt は今回の差分外として残る。
