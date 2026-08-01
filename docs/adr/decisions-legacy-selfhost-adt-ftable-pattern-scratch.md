# ADR: Selfhost flat ftable ADT alias targets and recursive pattern scratch

- Status: Accepted (verified partial slice)
- Date: 2026-08-01
- Scope: `selfhost/src/Backend/Wasm/Compiler.ls`, `selfhost/src/Backend/Wasm/CompilerSplit.ls`,
  `crates/lsharp-wasm/tests/e2e/strings_patterns_compiler_integration.rs`,
  `crates/lsharp-wasm/tests/e2e/selfhost_pattern_scratch_contract.rs`,
  `crates/lsharp-wasm/tests/e2e/selfhost_adt_import_runtime.rs`
- Related: `LEGACY-LANG-02`, `LEGACY-ROOT-01`

## Context

The flat ftable compiler had two independent gaps. Imported ADT variants with the same raw
constructor name could resolve to the last raw ftable entry instead of the selected module's
function target. Nested constructor patterns also reused locals consumed by map opcodes, so a
later sibling lookup could overwrite an earlier binder. The latter affected both the source-aware
and ftable compiler paths.

## Decision

Register an imported ADT alias by looking up each variant through the target module-qualified
constructor key and then registering the resulting function index under the alias-qualified key.
Keep raw constructor tags unchanged for runtime representation. For pattern recursion, reserve the
map opcode local footprint from `pattern-temp-base` and advance it by six locals for each nested
child. Do not change the existing result, outer scratch, or arm-binder layout.

## Evidence

- `test_e2e_selfhost_ftable_compiler_imported_alias_qualified_same_name_adt_constructors_run`
  generated and executed Wasm with `41\n5\n` for two same-name constructors.
- `test_e2e_selfhost_compiler_mode_adt_nested_constructor_pattern_runs` and
  `test_e2e_selfhost_ftable_compiler_adt_constructor_pattern_runs` both produced
  `41\n7\n42\n`.
- `test_e2e_selfhost_source_pattern_scratch_survives_deep_branching_constructor` and
  `test_e2e_selfhost_ftable_pattern_scratch_survives_deep_branching_constructor` both produced
  `42\n`.
- `test_e2e_selfhost_ftable_compiler_alias_qualified_record_pattern_runs` remained green.
- `test_e2e_selfhost_compiler_mode_imported_same_name_adt_aliases_run` generated and executed
  file-imported `App.Left` / `App.Right` Wasm with `41\n5\n` through `compile-file-mode`.

## Boundary

This is an actual Wasm source/ftable compiler slice, including the CompilerMode source-file
prelude ftable alias boundary. It does not establish flat `program-functions-base` file-import
parity, multi-segment module-qualified names, broad parametric or recursive ADT semantics,
nominal/exhaustiveness, WasmGC/linear-memory ABI parity, or current-source native stage0 evidence
on both supported targets. `LEGACY-LANG-02` and `LEGACY-ROOT-01` remain `[~]` in `TODO.md`.
