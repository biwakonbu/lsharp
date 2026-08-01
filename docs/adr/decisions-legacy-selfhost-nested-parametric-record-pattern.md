# ADR: Selfhost nested parametric record pattern materialization

- Status: Accepted (verified partial slice)
- Date: 2026-08-01
- Scope: `crates/lsharp-types/src/infer.rs`,
  `crates/lsharp-types/src/infer/expr.rs`,
  `selfhost/src/Types/TypeInferPattern.ls`,
  `crates/lsharp-ir/src/lower/mod.rs`,
  `crates/lsharp-types/src/infer_tests.rs`,
  `crates/lsharp-ir/src/lower/tests/records_and_adt.rs`,
  `crates/lsharp-wasm/tests/wasmgc_probe/part_015.rs`,
  `crates/lsharp-wasm/tests/e2e/selfhost_typeinfer_quote_patterns.rs`,
  `crates/lsharp-wasm/tests/e2e/strings_patterns_compiler_integration.rs`
- Related: `LEGACY-LANG-01`,
  `docs/adr/decisions-legacy-selfhost-record-pattern-visibility.md`

## Context

Parametric record applications can appear as fields of another record. For example, an outer
record field may have the expected type `Box a` while a nested record pattern resolves its schema
as a structural `Record` type. Without a boundary conversion, the Rust oracle and selfhost
inference paths disagree at `Type::App` versus `Type::Record` unification.

## Decision

- Rust materializes only `Type::App` names registered in `record_registry`, applies the record
  parameter substitution, and recursively normalizes the registered record fields. Unknown
  applications and non-record ADT applications remain unchanged.
- Selfhost materializes a registered record application lazily at each record-pattern field
  boundary, after applying the current substitution and before unifying the child pattern.
- The existing constructor-scheme visibility guard remains authoritative. A schema registry entry
  alone cannot make a private record pattern visible.
- Recursive broad expansion, spill-floor changes, and context/state refactors are outside this
  decision; only the observed registered-record application mismatch is addressed.
- WasmGC lowering resolves a registered named head inside `TypeExpr::App` to its concrete GC
  reference type. Unknown applications retain the existing fallback and are not claimed as
  supported by this slice.

## Evidence

- Rust focused regression `test_nested_parametric_record_pattern_propagates_field_type` passed.
- `test_e2e_selfhost_typeinfer_nested_parametric_record_pattern_binds_field_type` passed.
- The complete `selfhost_typeinfer_quote_patterns` group passed: 15 tests, 0 failures.
- `test_e2e_selfhost_compiler_mode_nested_parametric_record_pattern_runs` passed through the
  selfhost compiler-mode Wasm runtime and produced `41\n1\n7\n` for binder, literal, and fallback
  arms. This test-only addition does not change the native source producer.
- `test_e2e_selfhost_compiler_mode_imported_nested_parametric_record_pattern_runs` and
  `test_e2e_selfhost_ftable_compiler_nested_parametric_record_pattern_runs` passed with the same
  `41\n1\n7\n` result, covering imported qualified schemas and the ftable compiler path.
- `test_e2e_selfhost_compiler_mode_deep_nested_parametric_record_pattern_runs` passed through a
  four-record chain (`Root a -> Outer a -> Middle a -> Box a`) with the same result. This is a
  bounded depth regression, not an arbitrary-depth completion claim.
- `test_e2e_selfhost_ftable_compiler_deep_nested_parametric_record_pattern_runs` passed for the
  same four-record chain; the source and ftable deep fixtures passed together in one focused batch.
- `test_wasmgc_nested_parametric_record_pattern_preserves_reference_field_types` passed after
  confirming that `Outer.inner` is `Ref(Box)` in the WasmGC IR.
- `wasm_gc_emitter_executes_lowered_nested_parametric_record_pattern` passed through Wasmtime
  WasmGC validation, instantiation, and execution with result `41`.
- Mac Apple Silicon native gate passed for source commit
  `fa97fa948489f635dc8888b5a269755a75776670`; the ignored native test passed and the artifact was
  4660 KiB.
- Linux x86_64 hostgen/VM gate passed for the same source commit. Stage2/stage3 code lengths were
  both `11332908`, and their transport stdout SHA-256 was identical:
  `aa5cee91b5f47dd54a7da64492859bb1b9eede381059051713e85310115ba7ad`.

## Boundary

This closes the nested parametric record field-binding slice, source/import/ftable Wasm runtime
fixtures, and the four-record recursion regression. Arbitrarily deep patterns, depths beyond the
four-record regression, complete record-pattern semantic parity, full ftable/linear-memory ABI, and the
`LEGACY-LANG-01` aggregate remain incomplete. The WasmGC evidence is a Rust IR/emitter backend
slice and does not establish native stage0 or Rust-free selfhost producer parity. `TODO.md`
therefore keeps the aggregate as `[~]`.
