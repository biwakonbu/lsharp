# ADR: WASI argv helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-wasi-test-split.md`

## Context

`wasi.rs` は WASI preview1 / preview2、GC runtime、I/O helper を一つの production
module に持つ。`__command_line_args` と `__command_line_arg` の code emission も同じ
parent にあり、WASI runtime の別責務と同時に変更・レビューされる状態だった。origin/main
基準の parent は 4892 行で、argv helper は scratch layout、WASI import、String object
representation の三つの契約を抱えていた。

## Decision

- `emit_command_line_args_func` と `emit_command_line_arg_func` を
  `crates/lsharp-wasm/src/wasi/argv.rs`（258 行）へ移動する。
- `wasi.rs` は `argv` module を通じて両 helper を登録し、tagged pointer の共有 helper
  だけを `pub(super)` seam として提供する。
- `args_sizes_get` / `args_get` の呼び出し、scratch address `280` / `284`、負値・argc
  範囲外の空文字、allocator、length-header、tagged pointer、既存 function ordering は
  変更しない。
- `argv_tests.rs` の module seam test で、count と individual argument の両 function body
  が code section に登録される契約を固定する。

## Evidence

- RED: `argv` module が空の状態で seam test を実行し、両 helper の unresolved import で失敗。
- GREEN: `cargo test -p lsharp-wasm argv_module_emits_command_line_function_bodies -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm test_run_wasm_wasi_capture_preserves_lsp_soak_wire_after_reading_args -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm test_emit_wasm_wasi_p2_supports_stdin_and_args -- --nocapture`
  （1 passed）。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`

## Boundary

これは WASI argv code-emission の責務分離だけを扱う。argv の Rust/native selfhost parity、
dynamic memory layout、全公開 command、Mac Apple Silicon / Linux x86_64 native stage0 の
完了を意味しない。既存の `vector-push-pair-rooted-v3` selfhost fixture failure と
 package-wide test-only lint debt は今回の差分外として残る。
