# ADR: v0.4 M1-06 aggregate index target namespace schema

## Status

Accepted for the evidence-index/audit verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The aggregate audit already required two supported targets at runtime, but the
input JSON Schema only described two entries with a broad `ci-artifacts/`
prefix. A schema-only consumer could therefore accept swapped target order or
an index path outside the target-scoped `index.json` namespace.

## Decision

- Use JSON Schema Draft 2020-12 `prefixItems` to require
  `aarch64-apple-darwin` followed by `x86_64-unknown-linux-gnu`.
- Require each positional index reference to resolve to
  `ci-artifacts/v4-m1-01/<source_commit>/<target>/index.json` with a lowercase
  40-character source commit.
- Keep the executable audit responsible for current-HEAD binding, regular
  file/symlink safety, target index contents, and cross-target fixture scope.

## Consequences

Schema-only consumers now reject the most common target/path mix-ups before an
audit run. Dynamic provenance and filesystem checks remain fail-closed in the
Python audit, and native artifact/runtime evidence is still pending, so
V4-M1-06 remains `[~]`.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-aggregate-schema.py` —
  positional target and target-scoped path contracts.
- `python3 scripts/ci/test-semantic-fixture-evidence-aggregate.py` —
  executable two-target audit and cross-target path checks.
