# ADR: WasmGC instruction lowering and Component output split

- Status: Accepted (verified maintenance slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/src/wasmgc.rs`,
  `crates/lsharp-wasm/src/wasmgc/instructions.rs`,
  `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Related: `I-01`, `I-08`, `ISSUES-HANDOFF-WASMGC`,
  `imp-06-large-file-decomposition.md`

## Context

After validation had been separated from the WasmGC backend, the remaining
parent still combined public entrypoints and section assembly with IR
instruction lowering and the Component output linear-memory adapter. Those
instruction-level responsibilities have a separate runtime-index and GC
opcode contract and can be isolated without changing the public emitter API.

## Decision

- Move `WasmGcEmitOptions`, `ComponentOutputLocals`,
  `emit_wasm_gc_instructions`, and the canonical Component output copy/write
  helper to `wasmgc/instructions.rs`.
- Keep public WasmGC entrypoints, validation, type/import/export/code-section
  assembly, and the minimal struct probe in `wasmgc.rs`.
- Expose only parent-scoped `pub(super)` context. Preserve runtime import
  index mapping, typed-funcref offsets, struct/array/ref opcode lowering,
  packed-byte `ArrayGetU`, and the canonical output import boundary.
- Add an integration contract test that verifies the Component output core
  module exports memory and retains `main` while importing only the canonical
  `lsharp:wasmgc-output/stdout@0.1.0` / `write` boundary.

## Evidence

- RED: adding the `instructions` module declaration before the child existed
  failed with `E0583`.
- The focused Component output contract test passed after the split.
- `cargo test -p lsharp-wasm --test wasmgc_instruction_seam`: 1 passed.
- `cargo test -p lsharp-wasm --test wasmgc_probe`: 101 passed.
- `cargo test -p lsharp-wasm --lib`: 110 passed; the one failure is the
  existing `RootLifetime::RootSetWithoutActiveSlot` test failure in the WASI
  root-lifetime lane.
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`, targeted Rust 2024
  rustfmt, and `git diff --check` passed.

## Consequences

`wasmgc.rs` is reduced from 628 to 429 lines and the extracted instruction
module is 206 lines. Public WasmGC paths and generated import/export/runtime
contracts remain stable. Full WasmGC language/native/selfhost parity and the
broader `I-01` / `I-08` decomposition remain incomplete.
