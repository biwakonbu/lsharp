# ADR: v0.4 M1-06 current source boundary

## Status

Accepted for the evidence-index/schema verified slice (2026-08-01). This ADR
does not complete V4-M1-06, V4-M1-01, or the v0.4 milestone.

## Context

The report and evidence index schemas require a 40-character source commit and
the oracle/native reports must agree with each other. That consistency check
alone still permits the same stale commit to be copied into every report and
index, allowing evidence from another checkout to be accepted as current.

## Decision

The differential helper resolves `git rev-parse --verify HEAD` in its supplied
project root and requires both report source commits to equal that value. The
evidence-index audit applies the same check to the index before loading report
observations. A missing Git repository, malformed HEAD, or shared stale commit
fails closed.

## Consequences

Reports and evidence indexes are bound to the checkout that is being audited,
not merely to a self-consistent caller-provided string. Historical evidence can
still be audited by checking out its exact source commit in a dedicated
worktree first. Native/Rust target execution and two-target parity remain
separate pending gates.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-diff.py` — 6 focused tests,
  including a shared stale source commit.
- `python3 scripts/ci/test-semantic-fixture-evidence-audit.py` — 7 focused
  tests, including a stale source commit shared by index and reports.
