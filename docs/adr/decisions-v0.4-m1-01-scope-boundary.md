# ADR: v0.4 M1-01 semantic fixture scope boundary

## Status

Accepted for the verified inventory slice (2026-08-01). This ADR does not
complete V4-M1-01 or any v0.3/legacy item.

## Context

The semantic fixture matrix has separate `layers`, `observables`, and
`expected` sections. Without a cross-field contract, a fixture can claim a
Wasm artifact or runtime result while omitting the codegen/runtime boundary
that would produce it. Such a manifest would make a later evidence report
appear complete without naming the required observation.

## Decision

`semantic_fixture_matrix.py` validates the following scope relationships:

- A fixture that requires an artifact must include the `codegen` layer and the
  `wasm` observable.
- A fixture that declares an expected runtime result must include the
  `runtime` layer and the `runtime` observable.
- `wasm` and `runtime` observables cannot be declared when their corresponding
  artifact/runtime result is not expected.
- A fixture cannot declare the `runtime` layer when its runtime is `not-run`.

The existing invalid codegen fixture remains valid: it records the attempted
codegen failure boundary but does not claim an artifact. This keeps failure
boundaries distinct from successful artifact/runtime evidence.

## Consequences

The manifest now fails closed when its execution claims and scope inventory
diverge. Adding a new valid artifact/runtime fixture requires naming both the
processing boundary and its observable output. Invalid fixtures can still
stop at an intermediate layer without fabricating a Wasm or runtime result.
Actual Rust/native target evidence remains a separate pending boundary.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-matrix.py` — 17 focused contract
  tests, including missing `wasm` observable and `runtime` layer mutations.
- `python3 scripts/ci/semantic_fixture_matrix.py --manifest scripts/ci/semantic-fixture-matrix.json --root .`
  — deterministic manifest projection.
