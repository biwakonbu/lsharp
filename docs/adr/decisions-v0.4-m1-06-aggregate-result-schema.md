# ADR: v0.4 M1-06 aggregate audit result schema

## Status

Accepted for the evidence-index/audit verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The two-target aggregate audit already re-computes both target indexes and
prints a result, but the output shape was only implicit in the Python
implementation. A schema-only consumer could therefore miss the selected
fixture scope or the per-target pending/mismatch details.

## Decision

- Define `docs/schemas/v4-m1-06-evidence-aggregate-result.schema.json` for the
  recomputed JSON emitted by `semantic_fixture_evidence_aggregate.py`.
- Require the top-level source commit, status, and lexicographically selected
  `fixture_ids` together with exactly two target result entries.
- Require each target result to carry its target-scoped `index` path, matching
  fixture IDs, status, count, pending boundaries, and structured mismatches.
- Keep cross-target equality, canonical target order, current-source binding,
  and exit-code semantics as executable audit rules; JSON Schema alone cannot
  express those relationships.

## Consequences

Consumers can validate the aggregate output shape without importing the audit
implementation, while the executable audit remains authoritative for dynamic
scope and provenance checks. Native artifact/runtime and rollback evidence are
still pending, so V4-M1-06 remains `[~]`.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-aggregate-schema.py` —
  input and recomputed-result schema contracts.
- `python3 scripts/ci/test-semantic-fixture-evidence-aggregate.py` —
  both-target result fields and cross-target scope/exit behavior.
