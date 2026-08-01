# ADR: Selfhost nested parametric record pattern materialization

- Status: Accepted (verified partial slice)
- Date: 2026-08-01
- Scope: `crates/lsharp-types/src/infer.rs`,
  `crates/lsharp-types/src/infer/expr.rs`,
  `selfhost/src/Types/TypeInferPattern.ls`,
  `crates/lsharp-types/src/infer_tests.rs`,
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

## Evidence

- Rust focused regression `test_nested_parametric_record_pattern_propagates_field_type` passed.
- `test_e2e_selfhost_typeinfer_nested_parametric_record_pattern_binds_field_type` passed.
- The complete `selfhost_typeinfer_quote_patterns` group passed: 15 tests, 0 failures.
- `test_e2e_selfhost_compiler_mode_nested_parametric_record_pattern_runs` passed through the
  selfhost compiler-mode Wasm runtime and produced `41\n1\n7\n` for binder, literal, and fallback
  arms. This test-only addition does not change the native source producer.
- Mac Apple Silicon native gate passed for source commit
  `fa97fa948489f635dc8888b5a269755a75776670`; the ignored native test passed and the artifact was
  4660 KiB.
- Linux x86_64 hostgen/VM gate passed for the same source commit. Stage2/stage3 code lengths were
  both `11332908`, and their transport stdout SHA-256 was identical:
  `aa5cee91b5f47dd54a7da64492859bb1b9eede381059051713e85310115ba7ad`.

## Boundary

This closes the nested parametric record field-binding slice and one source compiler-mode Wasm
runtime fixture. Import visibility, arbitrarily deep patterns, complete record-pattern semantic
parity, ftable/linear-memory ABI parity, and the `LEGACY-LANG-01` aggregate remain incomplete.
`TODO.md` therefore keeps the aggregate as `[~]`.
