# ADR: v0.4 M1-06 two-target evidence aggregate

## Status

Accepted for the evidence-index/audit verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The target-scoped audit can prove one Mac or Linux bundle, but a single target
`pass` is not enough for the v0.4 completion target. The runbook requires both
`aarch64-apple-darwin` and `x86_64-unknown-linux-gnu`, and a target may remain
pending while its Linux replay or packaged artifact is unavailable.

## Decision

- Add a version-1 aggregate index at
  `ci-artifacts/v4-m1-01/<source_commit>/aggregate/index.json`.
- The aggregate must list exactly the two supported targets, in canonical
  order, and reference each target's exact `index.json` namespace.
- Both target indexes must select the same lexicographically ordered fixture
  IDs; otherwise a target-specific subset cannot be mistaken for parity.
- `semantic_fixture_evidence_aggregate.py` re-runs the existing per-target
  audit for both raw indexes. It does not trust target or aggregate `status`
  declarations.
- Aggregate status is `mismatch` if either target mismatches, `pending` if no
  target mismatches but either target is pending, and `pass` only when both
  target audits pass. A declared status that differs from this result fails
  closed.
- The JSON Schema fixes the aggregate suite/task and two-entry shape; dynamic
  source-commit/target path binding remains executable semantics.

## Consequences

Mac-only evidence cannot be promoted to a release-complete result. Pending
Linux evidence remains visible and returns exit code `2`; mismatches return
`1`; only two-target parity returns `0`. Actual native artifact/runtime and
rollback evidence remain pending, so V4-M1-06 stays `[~]`.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-aggregate.py` — both-pass,
  pending, missing-target, status, cross-target, and stale-source contracts.
- `python3 scripts/ci/test-semantic-fixture-evidence-aggregate-schema.py` —
  aggregate schema field and safe-path shape contracts.
- `docs/development/operations/v4-m1-semantic-fixture-evidence.md` —
  per-target producer flow followed by aggregate audit.
