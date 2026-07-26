# ADR: WASI print-string helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-wasi-string-eq-split.md`

## Context

`wasi.rs` は WASI preview1、preview2、GC runtime、I/O helper を一つの production module
に持つ。`__print_string` は String object の length header と payload pointer を読み、
linear-memory の iovec scratch を設定して fd 1 の `fd_write` を呼ぶ独立した code-emission
責務だが、他の helper と同じ parent にあり、WASI 変更の衝突単位を増やしていた。

## Decision

- `emit_print_string_func` を `crates/lsharp-wasm/src/wasi/print_string.rs`（62 行）へ移動
  する。これにより WASI production parent は 3448 行から 3392 行へ縮小する。
- Preview1 の helper registration は `print_string` module 経由にし、function ordering、
  String length-header、payload offset `8`、iovec address `IOV_ADDR`、nwritten scratch
  `NWRITTEN_ADDR`、fd 1 の `fd_write`、空文字 no-op contract は維持する。
- `print_string_tests.rs` の module seam test で helper body の登録を固定する。parent の
  memory-layout constants は明示 import し、別の定数複製は行わない。

## Evidence

- RED: 空の `print_string` module に対する seam test が `emit_print_string_func` の
  unresolved import で失敗。
- GREEN: `cargo test -p lsharp-wasm --lib print_string_module_emits_print_string_function_body -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --test e2e test_e2e_string_heap_print -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --test e2e test_e2e_write_string_stdout -- --nocapture`、
  `test_e2e_fd_write_stdout`、`test_e2e_string_heap_multiple_literals`、
  `test_e2e_string_print_string_empty`、`test_e2e_string_print_string_concat`（各 1 passed）。
- `cargo test -p lsharp-wasm --lib`（98 tests のうち 97 passed、既存の
  `RootLifetime::RootSetWithoutActiveSlot` failure 1 件）。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`

## Boundary

これは WASI `print-string` code-emission の責務分離だけを扱う。fd_write の全 errno/partial
write semantics、Preview2/Component の別出力境界、Rust/native selfhost parity、dynamic
memory layout、全公開 command、Mac Apple Silicon / Linux x86_64 native stage0 の完了を意味
しない。既存の `vector-push-pair-rooted-v3` selfhost fixture failure、root lifetime checker
の既知 failure、package-wide test-only lint debt は今回の差分外として残る。
