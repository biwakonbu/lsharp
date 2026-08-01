# ADR: v0.4 M1-06 report producer boundary

## Status

Accepted for the evidence-index/schema verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The diff and evidence-audit commands receive separate oracle and native report
paths. The report schema allowed either supported producer in either file, so
swapping the reports could still produce a pending or passing comparison with
the wrong provenance.

## Decision

The `oracle` input must declare `producer=rust-oracle`, and the `native` input
must declare `producer=native-stage0`. Both the direct differential command and
the evidence-index audit enforce this role before comparing fixture data.

## Consequences

Report path labels and producer identity now describe the same execution lane.
An allowed producer in the wrong lane fails closed instead of being treated as
equivalent evidence. Source commit, target, fixture, artifact, and runtime
parity remain separate checks and are not promoted by this identity check.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-diff.py` — 5 focused tests,
  including swapped oracle/native producer roles.
- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — 6 focused
  tests, including swapped report producer roles.
