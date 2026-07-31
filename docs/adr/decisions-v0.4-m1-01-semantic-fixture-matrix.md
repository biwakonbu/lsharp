# ADR: v0.4 M1-01 semantic fixture matrix contract

## Status

Accepted for the verified inventory slice (2026-08-01). This ADR does not
complete V4-M1-01 or any v0.3/legacy item.

## Context

The next-version plan needs one fixture set that can later be sent through the
Rust oracle, Rust-host selfhost, Mac native stage0, and Linux native stage0.
Existing tests cover many individual layers, but there was no small,
machine-readable inventory that made target scope, failure boundaries, or
pending artifact/runtime evidence explicit. A manifest that silently omitted a
target or accepted an implicit fallback would make a later differential report
look stronger than its evidence.

## Decision

- Use `scripts/ci/semantic-fixture-matrix.json` as the version-1 inventory for
  V4-M1-01. Every fixture names a project-relative `.ls` source, valid/invalid
  kind, covered layers, observable outputs, commands, expected diagnostics and
  spans, exit code, artifact state, and runtime state.
- Require both supported targets (`aarch64-apple-darwin` and
  `x86_64-unknown-linux-gnu`) on the manifest and every fixture in a stable
  order. Unknown or omitted targets fail closed.
- Require the execution policy `current-source`, `fallback=forbidden`, and
  `network=forbidden` at both manifest and fixture scope. The inventory cannot
  claim evidence for stale stage0 or implicit host/provider execution.
- Keep artifact and target/runtime results explicitly `pending` or `not-run`
  until the corresponding Rust differential, native, Wasm, and target gates
  produce evidence. The validator must not promote pending data to success.
- Compare one `rust-oracle` report with one `native-stage0` report per target
  using `scripts/ci/semantic_fixture_diff.py`. Equal diagnostics/exit/runtime/
  artifact observations are `pass`; any mismatch is `mismatch`; an unobserved
  artifact or runtime boundary is `pending` with exit code 2.
- Validate the contract with the standalone Python helper and focused unittest;
  the matrix projector emits the deterministic input and the diff helper emits
  a deterministic comparison result.

## Consequences

- Future runners can consume one stable fixture inventory without discovering
  target scope or expected failure semantics ad hoc.
- The current slice is useful immediately for RED/GREEN schema work, while its
  incomplete execution evidence remains visible as `[~]` in the milestone.
- Adding a fixture requires an explicit source path and observable contract;
  this intentionally adds a small amount of inventory maintenance before
  runtime parity work.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-matrix.py` — focused contract tests.
- `python3 scripts/ci/test-semantic-fixture-diff.py` — pending/pass/mismatch and
  stale source/target contract tests.
- `python3 scripts/ci/semantic_fixture_matrix.py --manifest scripts/ci/semantic-fixture-matrix.json --root .`
  — deterministic manifest projection.
