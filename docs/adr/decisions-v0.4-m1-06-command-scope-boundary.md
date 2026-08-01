# ADR: v0.4 M1-06 evidence command scope boundary

## Status

Accepted for the evidence-index/schema verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The fixture matrix permits several commands for a valid fixture, including
`check` and an artifact-producing `compile`/`build` route. The evidence index
records which command produced the referenced reports. Without a cross-field
check, an index could select `check` while claiming observed Wasm and runtime
evidence, even though that command cannot produce the required artifact.

## Decision

When a selected fixture requires an artifact, the evidence index command must
be `compile` or `build`. The command must still be declared by the fixture
matrix. Invalid fixtures may use their declared diagnostic command without an
artifact command because they do not claim an artifact or runtime result.

## Consequences

The command recorded beside a report now identifies an execution route capable
of producing the evidence it claims. A `check`-only selection for a valid
artifact/runtime fixture fails before reports are loaded. Actual command
execution and two-target evidence remain pending boundaries.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — 5 focused
  tests, including a valid fixture incorrectly selecting `check`.
- `python3 scripts/ci/semantic_fixture_evidence_audit.py` — command scope is
  checked against the projected fixture matrix before report comparison.
