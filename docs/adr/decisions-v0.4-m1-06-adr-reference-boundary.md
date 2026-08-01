# ADR: v0.4 M1-06 ADR reference boundary

## Status

Accepted for the evidence-index/schema verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The evidence index has an `adr` path, but the generic regular-file check
accepted any readable project file. An index could therefore point at a README
or source file while presenting it as the decision record for the evidence
bundle.

## Decision

The audit requires `evidence index.adr` to resolve to a regular Markdown file
under `docs/adr/`. Safe-path, symlink, and project-root checks remain in force;
the location and file type are an additional semantic scope check.

## Consequences

Every evidence bundle names a decision record from the ADR namespace rather
than an arbitrary project file. Legacy JSONL records are not accepted for this
V4 evidence index; a new evidence decision must have a Markdown ADR. Report
and target/runtime evidence remain separate pending boundaries.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — 7 focused
  tests, including an ADR path outside `docs/adr/`.
- `python3 scripts/ci/semantic_fixture_evidence_audit.py` — ADR scope is
  checked after safe file resolution and before report loading.
