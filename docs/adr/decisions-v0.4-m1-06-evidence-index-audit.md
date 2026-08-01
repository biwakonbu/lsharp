# ADR: v0.4 M1-06 evidence index audit contract

## Status

Accepted for the evidence-index/schema verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

V4-M1-06 requires fixture, command, target, artifact/runtime, negative-gate,
and ADR evidence to remain aligned. A prose checklist or an index that merely
claims `pass` can hide stale reports, a narrower fixture set, or pending
artifact/runtime boundaries. The audit must therefore treat the referenced
reports and comparison result as authoritative and recompute the comparison.

## Decision

- Use a version-1 `v4-m1-06` evidence index with a V4 task, target,
  source commit, ADR path, Rust/native report paths, comparison path, overall
  status, and a lexicographically sorted fixture list.
- Require the index `task` to match the fixture matrix suite identity. The
  current `v4-m1-01` matrix therefore accepts only `V4-M1-01` as the covered
  task; a syntactically valid but different V4-M1 task fails closed.
- Publish the input shape as
  `docs/schemas/v4-m1-06-evidence-index.schema.json`. The JSON Schema fixes
  required fields, supported targets/statuses, safe relative reference shape,
  fixture command shape, and the four required negative-gate values; semantic
  path existence, ordering, and report parity remain the executable audit's
  responsibility.
- Each fixture entry names one command declared by the fixture matrix and
  explicitly records four required negative gates:
  `fallback-forbidden`, `network-forbidden`, `source-commit-bound`, and
  `target-declared`. Every gate must have the value `pass`; missing or unknown
  gates fail closed.
- If the selected fixture requires an artifact, its evidence command must be
  `compile` or `build`; a `check` selection cannot claim artifact/runtime
  evidence even when `check` is listed in the fixture's command set.
- The report referenced as `oracle` must declare `producer=rust-oracle`, and
  the report referenced as `native` must declare `producer=native-stage0`.
  Producer identity is checked before differential comparison.
- `scripts/ci/semantic_fixture_evidence_audit.py` validates project-relative
  regular-file references without symlink traversal, revalidates both reports through the V4-M1-01
  report contract, recomputes `semantic_fixture_diff.py`'s comparison, and
  rejects any index status or comparison JSON that differs from the recomputed
  result. It emits a deterministic enriched evidence projection containing
  diagnostics, exit codes, artifact digest/size, runtime result, and gate
  values per fixture.
- Exit status remains an evidence boundary: `pass=0`, `mismatch=1`, and
  `pending=2`. Pending or mismatch observations cannot be promoted to a
  successful index. The audit does not fabricate target or runtime evidence.

## Consequences

- A future target run can preserve one small index while keeping report bytes
  and comparison output as the source of truth.
- A stale source commit, unsafe path, omitted fixture, undeclared command,
  missing negative gate, or status mismatch fails before a success result is
  emitted.
- The current slice proves only the schema/audit boundary. Actual Mac/Linux
  native artifacts, runtime output, release evidence, and full V4 task coverage
  remain explicit follow-up work and keep the milestone at `[~]`.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — passing,
  pending, verified-claim rejection, scope, gate, and safe-path contract tests.
- `python3 scripts/ci/test-semantic-fixture-evidence-schema.py` — schema
  required-field, status/target, reference, command, and negative-gate contract
  tests.
- `docs/development/operations/v4-m1-semantic-fixture-evidence.md` — target別の
  producer → diff → evidence audit → cleanup 手順。実 target の結果そのものではない。
- `python3 -m py_compile scripts/ci/semantic_fixture_evidence_audit.py
  scripts/ci/test-semantic-fixture-evidence-audit.py` — syntax gate.
