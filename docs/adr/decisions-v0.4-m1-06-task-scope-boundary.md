# ADR: v0.4 M1-06 evidence task scope boundary

## Status

Accepted for the evidence-index/schema verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The evidence index has a `task` identity while the referenced reports are
produced from a fixture matrix with its own suite identity. The audit accepted
any syntactically valid `V4-M1-##` task, so an index could label v0.4 M1-01
reports as M1-03 evidence and still pass all report comparisons.

## Decision

The audit derives the expected task from the fixture matrix suite and requires
an exact match before loading evidence reports. For the current matrix,
`suite=v4-m1-01` therefore requires `task=V4-M1-01`; a different V4-M1 task is
a scope error even when target, source commit, fixtures, and report bytes all
match.

## Consequences

Evidence identity cannot be relabeled independently of the fixture inventory.
When a future V4 task gets its own matrix suite, the same cross-field contract
will bind that suite to its task identity. The current V4-M1-01 execution and
two-target evidence remain pending.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — 5 focused
  tests, including a mismatched `V4-M1-03` task label.
- `python3 scripts/ci/semantic_fixture_evidence_audit.py` — task identity is
  checked before report loading and comparison.
