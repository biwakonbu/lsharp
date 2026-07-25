# ADR: WASI stdin helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-wasi-argv-split.md`

## Context

`wasi.rs` は WASI preview1 / preview2、GC runtime、I/O helper を一つの production
module に持つ。`__read_stdin` の 4 KiB chunk loop、String object の初期化・連結、WASI
`fd_read` scratch layout が他の runtime emission と同じ parent にあり、stdin の変更が
別の WASI 責務と同じ review/merge 単位になっていた。

## Decision

- `emit_read_stdin_func` を `crates/lsharp-wasm/src/wasi/stdin.rs`（97 行）へ移動する。
- `wasi.rs` は `stdin` module を通じて helper を登録し、tagged pointer の共有 helper
  だけを `pub(super)` seam として提供する。
- 空 String の length-header、4104-byte reusable chunk、`fd_read` iovec scratch
  (`352` / `360`)、EOF loop、`string_concat`、既存 function ordering は変更しない。
- `stdin_tests.rs` の module seam test で read-stdin function body の登録を固定する。

## Evidence

- RED: `stdin` module が空の状態で seam test を実行し、helper の unresolved import で失敗。
- GREEN: `cargo test -p lsharp-wasm --lib stdin_module_emits_read_stdin_function_body -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib test_run_wasm_wasi_capture_preserves_lsp_soak_wire_after_reading_args -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib test_emit_wasm_wasi_p2_supports_stdin_and_args -- --nocapture`
  （1 passed）。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`

## Boundary

これは WASI stdin code-emission の責務分離だけを扱う。stdin の Rust/native selfhost
parity、dynamic memory layout、全公開 command、Mac Apple Silicon / Linux x86_64 native
stage0 の完了を意味しない。既存の `vector-push-pair-rooted-v3` selfhost fixture failure
と package-wide test-only lint debt は今回の差分外として残る。
