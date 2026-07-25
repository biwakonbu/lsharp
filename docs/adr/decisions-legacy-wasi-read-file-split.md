# ADR: WASI read-file helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-wasi-write-file-split.md`

## Context

`wasi.rs` は WASI preview1 / preview2、GC runtime、I/O helper を一つの production
module に持つ。`__read_file` は String path を読み、preopened directory の `path_open`、
`fd_filestat_get`、allocator、`fd_read`、`fd_close` を順に呼んで String object を返す
独立した code-emission 責務だが、他の I/O helper と同じ parent にあり、WASI I/O 変更の
衝突単位を増やしていた。

## Decision

- `emit_read_file_func` を `crates/lsharp-wasm/src/wasi/read_file.rs`（241 行）へ移動する。
  これにより WASI production parent は 3874 行から 3639 行へ縮小する。
- Preview1 の helper registration は `read_file` module 経由にし、function ordering、
  import/index、String length-header、preopened dirfd `3`、fd scratch `280`、stat scratch
  `288`、iovec scratch `352`/`360`、tagged-pointer i64 return contract は維持する。
- `path_open`、`fd_filestat_get`、`fd_read`、`fd_close` の errno を fail-closed に扱う既存の
  empty/payload semantics、allocator 境界、親 module の tagged-pointer helper 利用は変更しない。
- `read_file_tests.rs` の module seam test で helper body の登録を固定する。

## Evidence

- RED: 空の `read_file` module に対する seam test が `emit_read_file_func` の unresolved
  import／registration で失敗。
- GREEN: `cargo test -p lsharp-wasm --lib read_file_module_emits_read_file_function_body -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib wasi::tests::test_wasi_read_file_preserves_fd_read_errno -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib wasi::tests::test_wasi_read_file_preserves_fd_close_errno -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib wasi::tests::test_wasi_file_helpers_preserve_path_open_errno -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib wasi::tests::test_wasi_file_helpers_fail_closed_on_path_open_errno -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib wasi::tests::test_emit_wasm_wasi_p2_supports_file_roundtrip -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --test e2e test_e2e_selfhost_wasmemit_read_file_preserves_fd_close_errno -- --nocapture`
  （1 passed、46.43 秒）。
- `cargo test -p lsharp-wasm --test e2e test_e2e_fd_read_file -- --nocapture` と
  `cargo test -p lsharp-wasm --test e2e test_e2e_file_roundtrip -- --nocapture`
  （各 1 passed）。
- `cargo test -p lsharp-wasm --lib`（95 tests のうち 94 passed、既存の
  `RootLifetime::RootSetWithoutActiveSlot` failure 1 件）。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`

## Boundary

これは WASI `read-file` code-emission の責務分離だけを扱う。4096 bytes 超の read、全 I/O
errno/path semantics、partial/streamed standalone runtime、Rust/native selfhost parity、
dynamic memory layout、全公開 command、Mac Apple Silicon / Linux x86_64 native stage0 の
完了を意味しない。既存の `vector-push-pair-rooted-v3` selfhost fixture failure、root
lifetime checker の既知 failure、package-wide test-only lint debt は今回の差分外として残る。
