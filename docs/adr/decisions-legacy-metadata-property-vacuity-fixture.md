# ADR: deterministic Bool property fixtures preserve the vacuity boundary

- Status: Accepted (verified maintenance/test-contract slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-tooling/src/metadata_test_tests.rs`
- Related backlog: `ISSUES-HANDOFF-LOW-RISK` / `I-01` / `I-08`

## Context

The tooling metadata suite had two known failures:

- `test_run_metadata_tests_executes_bool_property_binder`
- `test_run_metadata_tests_rejects_bool_property_above_two_cases`

Both fixtures used `(or value (not value))` as the postcondition. That
expression is intentionally vacuous, so `check_metadata` correctly returned
`LS2005` before the deterministic Bool smoke profile or its `cases` boundary
could be exercised. The failures were in the fixture contract, not in Bool
sampling or profile enforcement.

## Decision

Use a non-vacuous Bool property for execution/profile tests:
`(= result (if value 1 0))` with a function that returns the corresponding
integer. Keep the original tautological property as a dedicated regression
test that must return `LS2005`. Keep the `cases: 3` expectation as `LS3002`, so
the deterministic profile boundary is tested after vacuity validation passes.
No production implementation, diagnostic ordering, or public API changes are
made.

## Evidence

- Baseline focused metadata suite: 33 passed / 2 failed, both named above and
  both returning `LS2005`.
- Updated focused suite: 36 passed / 0 failed, including the new
  `test_run_metadata_tests_rejects_vacuous_bool_property`.
- `cargo test -p lsharp-tooling --lib -- --nocapture`: 133 passed / 0 failed.
- `cargo clippy -p lsharp-tooling --lib --tests -- -D warnings`: passed.
- Targeted Rust 2024 `rustfmt`, `git diff --check`, and `bash scripts/audit_docs.sh`:
  passed.

## Consequences

The Bool binder execution contract and the `LS2005` vacuity boundary are now
independently observable, while the `LS3002` deterministic-profile boundary is
no longer masked by an invalid fixture. Broader property semantics and native
stage0 parity remain outside this slice.
