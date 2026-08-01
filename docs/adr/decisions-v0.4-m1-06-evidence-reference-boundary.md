# ADR: v0.4 M1-06 evidence reference boundary

## Status

Accepted for the evidence-index/schema verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The evidence index points to reports, comparison output, and an ADR using
project-relative paths. Checking only the final path component is insufficient:
an intermediate symlink can redirect a seemingly safe report path outside the
task-owned project root. That would make the audit consume evidence from an
unrelated location.

## Decision

`semantic_fixture_evidence_audit.py` rejects every symlink component in a
referenced path, resolves the candidate strictly, and requires the resolved
regular file to remain under the supplied project root. Missing paths,
directories, escape paths, and symlink traversal all fail before report JSON is
loaded.

## Consequences

Evidence reports and ADR references must be ordinary files staged within the
project root. The audit no longer accepts an external or redirected report,
even when the symlink target itself is readable and contains valid JSON. This
keeps source/target/report provenance tied to the task-owned evidence bundle.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — 5 focused
  tests, including an intermediate symlink escape.
- `python3 scripts/ci/semantic_fixture_evidence_audit.py` — path validation is
  exercised before referenced report loading.
