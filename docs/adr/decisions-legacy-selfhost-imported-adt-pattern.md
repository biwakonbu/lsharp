# ADR: Selfhost imported ADT constructor pattern runtime slice

- Status: Accepted (verified partial slice)
- Date: 2026-08-01
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_adt_import_runtime.rs`
- Related: `LEGACY-LANG-02`, `EC-M1-01`

## Context

Selfhost ADT constructor and pattern runtime tests already covered declarations flattened into a
single source fixture. Type-inference tests also covered imported qualified constructor lookup, but
ADT constructor patterns did not apply the alias visibility contract in the Rust oracle or the
selfhost parser/type-inference path. The compiler-mode file resolver and generated Wasm runtime had no focused regression for an ADT
declared in one source file and consumed by another through `:open :only` or `:as ... :only`.

## Decision

Keep the existing file-based module resolver and constructor export filtering contract. Verify the
boundary with two files: `App.Shapes` declares `(Maybe a)` with `Just` and `Nothing`, while
`App.Main` imports only those constructors through both open and alias-qualified forms, matches both
variants, and is compiled by the selfhost compiler-mode entrypoint. Keep the full constructor hash
for type-environment lookup and store the raw suffix after constructor children so the runtime tag
check compares the same raw constructor identity as the value representation. Execute the emitted
Wasm with the existing compiler-mode runtime imports. For type inference, resolve a qualified
constructor pattern through the import alias and `:only` list before looking up its module-qualified
scheme, while preserving the existing unqualified constructor hash behavior.

## Evidence

`test_e2e_selfhost_compiler_mode_imported_adt_constructor_pattern_runs` and
`test_e2e_selfhost_compiler_mode_imported_alias_qualified_adt_constructor_pattern_runs` passed in
one focused invocation. Both generated Wasm programs executed the imported `Just 41` branch and the
imported `Nothing` fallback, producing `41\n0\n`.

`test_e2e_selfhost_typeinfer_analysis_filters_import_alias_only_adt_constructor_pattern` also
passed: the selected `L.Some` pattern produced `0` diagnostics, while the excluded `L.Other`
pattern produced `1`. The Rust oracle accepted the selected pattern and rejected the excluded
`:only` pattern.

## Boundary

This is a source-file import and compiler-mode prelude-ftable runtime slice. It does not establish
flat ftable compiler import parity, multi-segment module-qualified constructor patterns, broad
parametric or recursive ADT representation, nominal/exhaustiveness semantics, WasmGC/linear-memory
parity, or current-source native stage0 evidence on both supported targets. This does not establish
the full ADT type-inference aggregate. `TODO.md` keeps
`LEGACY-LANG-02` as `[~]`.
