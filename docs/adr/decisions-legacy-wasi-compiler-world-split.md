# ADR: WASI compiler-world emission split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`,
  `crates/lsharp-wasm/src/wasi/compiler_world.rs`,
  `crates/lsharp-wasm/src/wasi/compiler_world_tests.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`,
  `decisions-legacy-wasi-gc-collect-split.md`

## Context

The compiler-world Wasm emitter (`emit_wasm_wasi_with_options`) was a 760-line
production block in `wasi.rs`. It owns Preview1 imports, runtime helper
registration, user-function/table wiring, `_start`, `proc_exit` collection,
component-run export, and data-section emission. The public WASI entrypoints
and Preview2 component adapter are orchestration concerns and should not own
that core builder.

## Decision

- Move `emit_wasm_wasi_with_options` to `wasi/compiler_world.rs` and expose it
  only as `pub(super)`.
- Keep `emit_wasm_wasi`, `emit_wasm_wasi_p2`, HTTP detection, component adapter
  selection, shared layout/types, and public API in `wasi.rs`.
- Preserve every function index, import/export order, `_start` and
  `wasi:cli/run` wiring, allocator/GC/root helper registration, data offsets,
  and `export_component_run` behavior. This is an ownership split only.
- Add a seam test that emits an empty core Wasm module through the new builder.

## Evidence

- RED: adding the compiler-world module declaration and seam test failed with
  `E0583` while `wasi/compiler_world.rs` was absent.
- `wasi::compiler_world_tests::compiler_world_module_emits_empty_wasi_core_module`
  passed.
- Preview2 compile and component-run smoke tests passed.
- HTTP host bridge smoke, basic allocation E2E, and runtime heap telemetry E2E
  passed.
- `cargo test -p lsharp-wasm --lib` ran 108 tests with 107 passed and the
  existing `RootLifetime::RootSetWithoutActiveSlot` failure.
- `cargo clippy -p lsharp-wasm --lib --quiet -- -D warnings`, workspace check,
  targeted Rust 2024 rustfmt, and `git diff --check` passed.

## Consequences

`wasi.rs` is reduced from 1062 to 305 lines. The compiler-world builder is now
761 lines plus an 18-line seam test. Public APIs and runtime contracts remain
stable; remaining WASI production decomposition, native/selfhost parity, and
the aggregate I-01/I-08 work remain incomplete.
