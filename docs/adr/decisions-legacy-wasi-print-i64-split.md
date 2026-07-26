# ADR: WASI print-i64 helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`, `crates/lsharp-wasm/src/wasi/print_i64.rs`
- Related: `I-01`, `I-08`, `decisions-legacy-wasi-int-to-string-split.md`

## Context

`wasi.rs` contains the complete linear-memory implementation of `__print_i64` beside
allocator, GC, file, and input helpers. The decimal conversion and two `fd_write` calls are
an independent code-emission responsibility, but changes to it currently share the same
production file and review boundary as the rest of the WASI runtime.

## Decision

- Move `emit_print_i64_func` to `wasi/print_i64.rs`.
- Keep the helper `pub(super)` and reuse the parent memory-layout constants through `super`.
- Update Preview1 code generation to call the module helper without changing function ordering,
  local layout, decimal conversion, newline output, or the `fd_write` ABI.
- Add a module seam test that asserts one code body is emitted.

## Evidence

- RED: the new seam test failed while `print_i64.rs` was absent (`E0583`).
- GREEN: `wasi::print_i64_tests::print_i64_module_emits_print_function_body` (1 passed).
- `cargo test -p lsharp-wasm --test e2e test_e2e_print_int_backward_compat -- --nocapture`:
  1 passed.
- `cargo test -p lsharp-wasm wasi:: --lib`: 41 passed / 1 existing
  `RootLifetime::RootSetWithoutActiveSlot` failure.

## Consequences

The WASI production parent is reduced from 3133 lines to 3015 lines and print-i64 code
emission can be reviewed and tested independently. This slice does not complete the full
WasmGC/linear-memory parity, native ABI, dynamic memory layout, Mac/Linux stage0, or I-01/I-08
aggregate requirements. The known root-lifetime test failure remains outside this refactor.
