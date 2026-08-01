# ADR: v0.4 M1-06 ADR schema boundary

## Status

Accepted for the evidence-index/schema verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The executable audit restricted `evidence index.adr` to `docs/adr/*.md`, but
the published JSON Schema still described it as any safe project-relative
string. Consumers that validate only the schema could therefore accept a path
that the authoritative audit rejects.

## Decision

The schema's `adr` pattern requires a Markdown path under `docs/adr/` and
rejects parent traversal and backslashes. The executable audit remains
responsible for existence, regular-file, symlink, and resolved-root checks.

## Consequences

Schema-only tooling and the audit now share the same ADR namespace and file-type
boundary. This is input-shape parity, not evidence completion; source commit,
report, artifact, runtime, and target gates remain independently required.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-schema.py` — schema
  structure and ADR pattern contract tests.
- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — executable
  ADR path and regular-file scope tests.
