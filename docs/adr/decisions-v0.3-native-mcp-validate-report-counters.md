# ADR: v0.3 native MCP validate report counter boundary

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP `lsharp_validate` report counters

## Context

The published native MCP report schema already bounded `open_questions`,
`independent_reviews`, `contradicting_observations`, `stale_reviews`, and
`stale_evidence` at the unsigned 64-bit maximum. The postflight shim only checked
that these values were non-negative integers, so `u64::MAX + 1` could still be
returned as native success.

## Decision

- Reuse the shared `U64_MAX` boundary for all five report counters.
- Reject booleans, non-integers, negatives, and values above `U64_MAX` with the
  existing stable counter error before structured content is returned.
- Keep the public output schema assertions and valid nested report behavior in
  the native MCP contract tests.

## Evidence

- RED: a fake native report with `stale_evidence = 18446744073709551616` was
  accepted by the previous shim.
- GREEN: the overflow fixture is rejected without a traceback; all five schema
  counters continue to expose the same maximum.
- Full native MCP suite passes: 77 tests.
- Python compilation, docs audit, and `git diff --check` pass.

This closes only the native report counter postflight boundary. Provider
semantics, target runtime, and full Rust/native parity remain active `[~]`
boundaries in `TODO.md`.
