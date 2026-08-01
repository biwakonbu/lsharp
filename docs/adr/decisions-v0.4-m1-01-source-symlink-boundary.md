# ADR: v0.4 M1-01 source fixture ownership boundary

## Status

Accepted for the verified inventory slice (2026-08-01). This ADR does not
complete V4-M1-01 or any v0.3/legacy item.

## Context

The semantic fixture manifest uses project-relative source paths. A lexical
`..`/absolute-path check is not sufficient: `Path.is_file()` follows a
symlink, so a fixture path inside the project could otherwise read source from
outside the project root. That would make the source inventory non-reproducible
and could bypass the task-owned worktree boundary.

## Decision

- Before accepting a fixture source, walk every component of its normalized
  project-relative path from the manifest root.
- Reject the source when any component is a symlink, including the final
  `.ls` file. Only an owned regular source fixture is accepted; `is_file()` is
  checked after this symlink traversal guard.
- Keep the existing lexical path rules and `.ls` suffix rule. This boundary
  applies to the manifest validator and does not claim native, artifact, or
  runtime parity.

## Consequences

The matrix cannot stage an external source through a symlink, even when the
symlink target happens to be a regular `.ls` file. Fixture source ownership is
now deterministic across the Rust and native producer lanes. Projects that
need shared fixtures must copy them into the task-owned worktree instead.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-matrix.py` — 19 focused contract
  tests, including rejection of a source fixture symlink traversal.
- `python3 scripts/ci/semantic_fixture_matrix.py --manifest scripts/ci/semantic-fixture-matrix.json --root .`
  — deterministic manifest projection.
