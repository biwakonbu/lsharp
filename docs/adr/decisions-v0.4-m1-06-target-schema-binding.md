# ADR: v0.4 M1-06 target-bound evidence index schema

## Status

Accepted for the schema-only target namespace boundary (2026-08-01,
implementation commit `6dc3e754`). This ADR does not complete V4-M1-06,
V4-M1-01, or the v0.4 milestone.

## Context

The executable evidence audit already derives
`ci-artifacts/v4-m1-01/<source_commit>/<target>/` from the index fields. The
published JSON Schema previously checked only the common prefix, so a
schema-only consumer could accept a Mac index whose report path used the Linux
directory. The executable audit would reject that bundle later, but the first
consumer had already accepted the wrong target scope.

## Decision

- Add one JSON Schema `if`/`then` branch for each supported target.
- Require `oracle_report`, `native_report`, and `comparison` to contain the
  declared target directory and a 40-character lowercase source-commit
  directory.
- Keep exact source-commit freshness, regular-file, symlink, and project-root
  checks in the executable audit; JSON Schema still cannot compare a path
  segment with a sibling property's value or inspect the filesystem.

## Evidence

- The new schema contract test fails against the previous schema because
  `allOf` is absent, then passes after the target branches are added.
- `python3 scripts/ci/test-semantic-fixture-evidence-schema.py` — 4 tests.
- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — 12 tests.
- `python3 scripts/ci/test-semantic-fixture-evidence-aggregate-schema.py` — 4
  tests.
- `python3 scripts/ci/test-semantic-fixture-evidence-aggregate.py` — 7 tests.

## Consequences

Schema-only validation now rejects the opposite target's artifact namespace
before executable audit. Filesystem ownership and current-source freshness stay
explicit executable boundaries, and actual Mac/Linux artifact/runtime parity
remains pending.
