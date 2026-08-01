# ADR: v0.4 M1-06 evidence index ownership boundary

## Status

Accepted for the evidence-index/audit verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The report and comparison references are now bound to
`ci-artifacts/v4-m1-01/<source_commit>/<target>/`, but the audit input
`index.json` itself could still be placed in `docs/`, a temporary directory, or
another task's worktree. That weakens the runbook's task-owned bundle boundary
and makes cleanup/provenance ambiguous even when the referenced reports are
valid.

## Decision

- `index.json` must be a regular, non-symlink file named exactly `index.json`
  under the same exact target-scoped namespace as its oracle/native reports and
  comparison.
- The executable audit validates the index path before accepting its declared
  report paths. Absolute paths outside the project root, symlink traversal, and
  a source/target namespace mismatch fail closed.
- The JSON Schema continues to describe the index contents; filesystem
  ownership remains executable semantics because JSON Schema validates values,
  not the path of the document being validated.

## Consequences

One evidence bundle now has one task-owned root containing `index.json`, both
producer reports, and the comparison. Operators can archive or remove the
bundle as one unit without leaving an index detached from its evidence. Actual
Mac/Linux artifact and runtime parity remains pending, so V4-M1-06 stays `[~]`.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — passing
  in-bundle and outside-index rejection tests.
- `docs/development/operations/v4-m1-semantic-fixture-evidence.md` —
  target-scoped bundle creation and cleanup procedure.
