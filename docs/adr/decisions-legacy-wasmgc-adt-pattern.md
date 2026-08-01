# ADR: WasmGC ordinary ADT pattern runtime slice

- Status: Accepted (verified partial slice)
- Date: 2026-08-01
- Scope: `crates/lsharp-ir/src/lower/tests/records_and_adt.rs`,
  `crates/lsharp-wasm/tests/wasmgc_probe/part_015.rs`
- Related: `LEGACY-LANG-02`, `LEGACY-EXEC-01`

## Context

The WasmGC backend already represented ordinary ADTs as a shared struct with a tag field and
typed payload slots, but source-level construction and pattern execution did not have a direct
runtime regression. Linear-memory ADT tests and selfhost compiler tests do not establish the
WasmGC emitter/runtime boundary.

## Decision

- Keep the WasmGC ADT representation as `tag` in field zero followed by shared typed payload
  slots.
- Verify source lowering through a non-parametric `Option` with `Some Int` and `None`, including
  the tag check, payload binder, and fallback arm.
- Treat this as a backend/runtime slice only; it does not expand parametric ADT representation or
  move the native stage0 producer to WasmGC.

## Evidence

- `test_wasmgc_adt_pattern_lowers_typed_payload_and_tag_checks` passed and confirmed the tag and
  payload `StructGet` instructions in the lowered IR.
- `wasm_gc_emitter_executes_lowered_adt_pattern_with_typed_payload` passed through WasmGC
  validation, instantiation, and execution. The `Some 41` arm plus `None` fallback produced `42`.

## Boundary

Parametric ADT payloads, recursive/self-referential ADTs, nested ADT/import/ftable parity,
nominal and exhaustiveness semantics, linear-memory ABI parity, native stage0 evidence, and the
`LEGACY-LANG-02` aggregate remain incomplete. `TODO.md` keeps the aggregate as `[~]`.
