# ADR: v0.4 M1-01 artifact evidence requires an artifact command

## Status

Accepted for the verified inventory slice (2026-08-01). This ADR does not
complete V4-M1-01 or the v0.4 milestone.

## Context

The semantic fixture matrix lets each fixture declare both its expected
artifact state and the commands that produce evidence. Before this decision,
a valid fixture could require an artifact (`artifact.required: true`) while
declaring only `check`. That inventory could therefore claim a compile/runtime
boundary without naming a command capable of producing the artifact.

## Decision

- A fixture whose expected artifact is required must declare `compile` or
  `build` in its command list.
- The executable matrix validator rejects the mismatch before projecting the
  manifest; the error remains a schema/inventory error rather than a pending
  runtime observation.
- Invalid fixtures are not subject to this requirement because their expected
  artifact is explicitly `not-applicable`.

## Consequences

The fixture command scope and artifact evidence scope cannot silently diverge.
This is only an inventory integrity boundary: actual Rust/native artifact and
runtime evidence remains pending until the target gates run.

## Evidence

- RED: a valid runtime fixture changed to `commands: ["check"]` was accepted.
- GREEN: the same fixture is rejected with the `artifact command` diagnostic.
- `python3 scripts/ci/test-semantic-fixture-matrix.py` — 16 tests passed.
- `python3 scripts/ci/test-semantic-fixture-diff.py` — 4 tests passed.
- `python3 scripts/ci/test-semantic-fixture-rust-report.py` — 11 tests passed.
- `python3 scripts/ci/test-semantic-fixture-native-report.py` — 11 tests passed.
- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — 4 tests passed.
- `python3 scripts/ci/test-semantic-fixture-evidence-schema.py` — 3 tests passed.
