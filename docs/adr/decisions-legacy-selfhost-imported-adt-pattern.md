# ADR: Selfhost imported ADT constructor pattern runtime slice

- Status: Accepted (verified partial slice)
- Date: 2026-08-01
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_adt_import_runtime.rs`
- Related: `LEGACY-LANG-02`, `EC-M1-01`

## Context

Selfhost ADT constructor and pattern runtime tests already covered declarations flattened into a
single source fixture. Type-inference tests also covered imported qualified constructor lookup, but
the compiler-mode file resolver and generated Wasm runtime had no focused regression for an ADT
declared in one source file and consumed by another through `:open :only`.

## Decision

Keep the existing file-based module resolver and constructor export filtering contract. Verify the
boundary with two files: `App.Shapes` declares `(Maybe a)` with `Just` and `Nothing`, while
`App.Main` imports only those constructors, matches both variants, and is compiled by the selfhost
compiler-mode entrypoint. Execute the emitted Wasm with the existing compiler-mode runtime imports.

## Evidence

`test_e2e_selfhost_compiler_mode_imported_adt_constructor_pattern_runs` passed. The generated Wasm
executed the imported `Just 41` branch and the imported `Nothing` fallback, producing `41\n0\n`.

## Boundary

This is a source-file import and compiler-mode runtime slice. It does not establish ftable import
parity, qualified or alias-qualified constructor patterns, broad parametric or recursive ADT
representation, nominal/exhaustiveness semantics, WasmGC/linear-memory parity, or current-source
native stage0 evidence on both supported targets. `TODO.md` keeps `LEGACY-LANG-02` as `[~]`.
