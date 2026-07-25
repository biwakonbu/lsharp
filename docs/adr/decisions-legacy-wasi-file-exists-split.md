# ADR: WASI file-exists helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-wasi-root-split.md`

## Context

`wasi.rs` は WASI preview1 / preview2、GC runtime、I/O helper を一つの production
module に持つ。`__file_exists` は String の length-header から path を読み、preopened
directory の `path_open` と `fd_close` を呼び、open/close errno を 0/1 の i64 へ変換する
独立した code-emission 責務だが、他の read/write helper と同じ parent にあり、WASI I/O
変更の衝突単位を増やしていた。

## Decision

- `emit_file_exists_func` を `crates/lsharp-wasm/src/wasi/file_exists.rs`（71 行）へ移動する。
- Preview1 の helper registration は `file_exists` module 経由にし、function ordering、
  import/index、scratch fd slot `280`、preopened dirfd `3`、i64 return contract は維持する。
- `path_open` errno と `fd_close` errno を fail-closed に扱い、既存の存在判定 semantics は変更しない。
- `file_exists_tests.rs` の module seam test で helper body の登録を固定する。

## Evidence

- RED: 空の `file_exists` module に対する seam test が helper の unresolved import で失敗。
- GREEN: `cargo test -p lsharp-wasm --lib file_exists_module_emits_file_exists_function_body -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib`（92 tests のうち 91 passed、既存の
  `RootLifetime::RootSetWithoutActiveSlot` failure 1 件）。
- `cargo test -p lsharp-wasm --lib wasi::tests::test_wasi_file_exists -- --nocapture`
- `cargo test -p lsharp-wasm --test e2e test_e2e_file_exists_check -- --nocapture`
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`

## Boundary

これは WASI `file-exists` code-emission の責務分離だけを扱う。I/O の全 errno/path
semantics、Rust/native selfhost parity、dynamic memory layout、全公開 command、Mac Apple
Silicon / Linux x86_64 native stage0 の完了を意味しない。既存の
`vector-push-pair-rooted-v3` selfhost fixture failure、root lifetime checker の既知 failure、
package-wide test-only lint debt は今回の差分外として残る。
