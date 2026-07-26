# ADR: WASI instruction emission dispatcher split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`, `crates/lsharp-wasm/src/wasi/instructions.rs`
- Related: `I-01`, `I-08`, `decisions-legacy-wasi-gc-mark-split.md`

## Context

The WASI backend's instruction dispatcher remaps IR function indices and indirect-call type
indices, dispatches runtime helper calls, and delegates struct/write-file-bytes instructions.
It is a separate translation boundary from module construction and runtime helper emission,
but was embedded in the large `wasi.rs` production file.

## Decision

- Move `emit_instructions_wasi` to `wasi/instructions.rs`.
- Keep the dispatcher `pub(super)` and retain the parent-owned `IR_IMPORT_COUNT` and struct
  scratch contracts.
- Preserve IR-to-Wasm function-index remapping, CallIndirect type remapping, custom struct
  emission, `write-file-bytes` target rejection, and all caller argument ordering.
- Add direct seam tests for empty emission and fail-closed `write-file-bytes` handling.

## Evidence

- RED: the new seam test failed while `instructions.rs` was absent (`E0583`).
- GREEN: instruction module seam tests (2 passed).
- `cargo test -p lsharp-wasm --test e2e record_field_access_compile -- --nocapture`:
  1 passed.
- `cargo test -p lsharp-wasm --test e2e print_int_backward_compat -- --nocapture`: 1 passed.
- `cargo test -p lsharp-wasm wasi:: --lib`: 44 passed / 1 existing
  `RootLifetime::RootSetWithoutActiveSlot` failure.

## Consequences

The WASI production parent is reduced from 2792 lines to 2649 lines and instruction lowering
can be reviewed independently from runtime construction. This slice does not complete all
backend/native/selfhost parity, native ABI, dynamic memory layout, Mac/Linux stage0, or the
aggregate I-01/I-08 requirements. The known root-lifetime failure remains outside this
refactor.
