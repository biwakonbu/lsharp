# ADR: v0.4 M1-06 evidence artifact namespace boundary

## Status

Accepted for the evidence-index/audit verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The V4 M1 semantic-fixture runbook assigns report and comparison output to a
task-owned `ci-artifacts/` directory. Before this decision, the evidence index
could reference any regular JSON file under the project root. A copied or
hand-authored file in a source or documentation directory could therefore be
treated as a producer report even though it was outside the task-owned
artifact boundary.

## Decision

- `oracle_report`, `native_report`, and `comparison` references in the version-1
  evidence index must use a normalized project-relative path under
  `ci-artifacts/`.
- The executable audit applies the same namespace check before resolving the
  referenced regular file. Existing symlink, root-escape, and regular-file
  checks remain in force.
- The published JSON Schema uses the same `^ci-artifacts/` path shape so
  schema-only consumers cannot accept a report location that the executable
  audit rejects.
- The ADR reference remains separate: it must continue to be a regular
  Markdown file under `docs/adr/`, not an artifact output.

## Consequences

Evidence bundles have an explicit ownership boundary and cannot silently
reuse arbitrary project files as report inputs. Test fixtures create their
temporary bundle below `ci-artifacts/`; bundles outside that namespace fail
closed before report parsing. Actual native/artifact/runtime evidence and
two-target completion remain pending, so V4-M1-06 stays `[~]`.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — passing
  in-namespace bundle and outside-namespace rejection tests.
- `python3 scripts/ci/test-semantic-fixture-evidence-schema.py` — report and
  comparison path pattern parity tests.
- `docs/development/operations/v4-m1-semantic-fixture-evidence.md` —
  task-owned `ci-artifacts/` generation and cleanup procedure.
