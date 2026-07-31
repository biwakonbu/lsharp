# ADR: v0.3 native MCP validate closed-world input

## Status

Verified partial slice (2026-08-01). Native `lsharp_validate` now rejects
unknown top-level arguments before invoking `program.native`.

## Context

The Rust MCP validator publishes `additionalProperties: false` for
`lsharp_validate`. The native shim previously exposed a looser schema and
silently ignored an unknown argument, which could make a caller believe a
review or validation option had been applied when it had not.

## Decision

- Publish `additionalProperties: false` in the native `tools/list` schema.
- Keep one explicit allowlist for source/manifest selection, provider snapshot
  inputs, review identity fields, digest fields, and `include_manifest`.
- Reject any unknown top-level argument as an MCP tool error before reading
  input files or starting the native program.

## Evidence

- `scripts/ci/test-native-selfhost-mcp.py` verifies the closed-world schema and
  unknown-argument no-execution behavior alongside the existing MCP suite.
- Native MCP and runner focused tests, Python compilation, shell syntax checks,
  docs audit, and `git diff --check` pass.

## Remaining boundary

Full Rust MCP tool parity, runtime validation of nested manifest input/output,
provider authentication/signature/lifecycle semantics, and current-source Linux
runtime evidence remain `[~]` under `EC-M3-05` / M3-05-N9.
