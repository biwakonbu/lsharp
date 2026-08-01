# ADR: v0.4 M1-06 target-scoped evidence artifact layout

## Status

Accepted for the evidence-index/audit verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The semantic-fixture runbook executes the same source commit for both
`aarch64-apple-darwin` and `x86_64-unknown-linux-gnu`. A source-commit-only
artifact directory lets the second target overwrite the first target's reports,
or lets an index point at a report produced for the other target while its
`target` field still looks valid.

## Decision

- The canonical report/comparison layout is
  `ci-artifacts/v4-m1-01/<source_commit>/<target>/...`.
- The executable audit derives that namespace from the index's `source_commit`
  and `target`, then requires `oracle_report`, `native_report`, and
  `comparison` to be regular files below that exact directory.
- The JSON Schema publishes the static `ci-artifacts/v4-m1-01/` prefix. The
  dynamic source-commit and target binding remains executable semantics because
  JSON Schema cannot compare a path to sibling property values in this version.
- The source-commit freshness check runs before path resolution, so a stale
  index cannot be hidden behind a namespace mismatch.

## Consequences

Mac and Linux evidence for one source commit have independent task-owned
directories and cannot overwrite or cross-reference one another accidentally.
Operators must set `TARGET` before creating `EVIDENCE_ROOT`; old
source-commit-only bundles are rejected and must be regenerated. Actual
two-target artifact/runtime parity remains pending, so V4-M1-06 stays `[~]`.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — passing
  target-mismatch, stale-source, in-namespace, and outside-namespace tests.
- `python3 scripts/ci/test-semantic-fixture-evidence-schema.py` — static
  versioned artifact prefix parity test.
- `docs/development/operations/v4-m1-semantic-fixture-evidence.md` —
  target-scoped producer/diff/audit layout and cleanup procedure.
