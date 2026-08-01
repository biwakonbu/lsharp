# ADR: v0.3 native MCP manifest sampling invariant

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP emitted manifest `execution.sampling`

## Context

The native MCP shim validated sampling counters and coverage bucket names independently, but it
could accept a non-empty coverage map whose counts did not partition the declared `cases`, or
whose checked sum exceeded the u64 wire range. Rust's canonical `SamplingPlan` rejects both
conditions while preserving compatibility for omitted or empty coverage.

## Decision

- When `coverage` is non-empty, require the checked sum of bucket counts to equal `cases`.
- Reject a checked sum above the u64 wire range before comparing it with `cases`.
- Keep omitted and empty coverage accepted; generator, cases, seed, bucket names, and individual
  counters retain their existing shape validation.

## Evidence

- RED: non-partitioned and overflowing coverage maps were accepted by shape-only validation.
- GREEN: mismatch and overflow fixtures fail closed, while canonical and empty-coverage records
  remain valid.
- Full native MCP suite passes: 77 tests.
- `python3 -m py_compile scripts/native-selfhost-mcp.py scripts/ci/native_selfhost_mcp_manifest_tests.py scripts/ci/test-native-selfhost-mcp.py` passes.

This closes only emitted manifest sampling consistency. Referential/provider semantics, native
stage0, target runtime, and full Rust/native parity remain active `[~]` boundaries in `TODO.md`.
