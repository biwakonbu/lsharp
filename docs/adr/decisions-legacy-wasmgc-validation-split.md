# ADR: WasmGC type conversion and validation split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasmgc.rs`,
  `crates/lsharp-wasm/src/wasmgc/validation.rs`,
  `crates/lsharp-wasm/src/wasmgc/validation_tests.rs`
- Related: `I-01`, `I-08`, `ISSUES-HANDOFF-WASMGC`,
  `imp-06-large-file-decomposition.md`

## Context

The WasmGC backend kept type conversion, typed-funcref discovery, String array
lookup, and the full IR validation pass in one 385-line block inside
`wasmgc.rs`. These checks are a stable boundary before instruction emission and
are independent from the core/component code builder.

## Decision

- Move the type conversion and validation helpers to
  `wasmgc/validation.rs`, exposing only the helpers needed by the parent as
  `pub(super)`.
- Keep public WasmGC entrypoints, section construction, instruction emission,
  and Component output wiring in `wasmgc.rs`.
- Preserve all `IrType`→Wasm type mappings, typed-funcref offsets, GC/function
  index diagnostics, print-string import detection, and fail-closed unsupported
  instruction boundaries. This is an ownership split only.
- Add a seam test for scalar conversion and in/out-of-range GC type indices.

## Evidence

- RED: adding the validation module declaration and seam test failed with
  `E0583` while `wasmgc/validation.rs` was absent.
- `wasmgc::validation_tests::validation_module_converts_scalar_and_checks_gc_indices`
  passed.
- The complete `wasmgc_probe` integration suite passed: 101 tests.
- `cargo test -p lsharp-wasm --lib` ran 109 tests with 108 passed and the
  existing `RootLifetime::RootSetWithoutActiveSlot` failure.
- `cargo clippy -p lsharp-wasm --lib --quiet -- -D warnings`, workspace check,
  targeted Rust 2024 rustfmt, and `git diff --check` passed.

## Consequences

`wasmgc.rs` is reduced from 992 to 628 lines. The validation module is 389
lines plus an 11-line seam test. Public WasmGC APIs and output contracts remain
stable; full native/selfhost parity and the broader WasmGC feature handoff are
still incomplete.
