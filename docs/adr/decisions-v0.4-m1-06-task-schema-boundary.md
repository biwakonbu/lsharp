# ADR: v0.4 M1-06 task schema boundary

## Status

Accepted for the evidence-index/schema verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The executable audit binds the evidence index task to the `v4-m1-01` fixture
matrix, but the published schema still accepted any syntactically valid
`V4-M1-##` task. Schema-only consumers could therefore label the same fixture
reports as a different milestone task.

## Decision

The version-1 evidence-index schema declares `task` as the constant
`V4-M1-01`. The executable audit remains the authority for cross-checking this
identity against the loaded fixture matrix suite.

## Consequences

Schema validation and executable audit now reject task relabeling consistently.
When another V4 task receives its own evidence-index schema, it must be a new
versioned contract rather than silently broadening this one. Artifact/runtime,
target, and native parity evidence remain pending separately.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-schema.py` — schema task
  constant and required-field contract tests.
- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — executable
  task/suite identity mismatch test.
