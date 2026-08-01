# ADR: v0.4 M1-01 non-empty observed artifact boundary

## Status

Accepted for the semantic fixture diff producer contract (2026-08-01,
implementation commit `4b70bb7d`). This ADR does not complete V4-M1-01 or the
Mac/Linux artifact and runtime gates.

## Context

An observed fixture artifact was validated only as a regular `sha256` value and
a non-negative integer size. That allowed `size=0` to enter the comparison as
an observed artifact, even though an actual Wasm artifact must contain bytes.
Both producer reports could therefore carry the same empty artifact metadata
and avoid the intended pending/mismatch boundary.

## Decision

`semantic_fixture_diff.py` rejects an observed artifact unless `size` is a
positive integer. Pending and not-applicable states keep their existing shape;
the check applies only after a producer claims that an artifact was observed.

## Evidence

- The RED test sets both producer reports to an observed artifact with
  `size=0`; the previous implementation returned pending (`2`) instead of
  rejecting the report.
- After the change, the focused test rejects the report with an explicit
  positive-size error.
- The full diff suite (7 tests), evidence audit suite (12 tests), and aggregate
  suite (7 tests) pass.

## Consequences

Empty files cannot be promoted into observed artifact evidence. The producer
still owns actual Wasm validation and runtime execution; this boundary only
guards the report comparison input and does not claim target parity.
