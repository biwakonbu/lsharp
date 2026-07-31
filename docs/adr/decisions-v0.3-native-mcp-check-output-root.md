# ADR: v0.3 native MCP check output root boundary

## Status

Verified partial slice (2026-08-01). Native MCP `lsharp_check` now validates
its closed output root before returning structured content.

## Context

The Rust MCP check contract requires `ok`, `diagnostics`, and
`migrationDiagnostics`. A native program that emits an array, `null`,
malformed JSON, a missing or unknown field, a non-boolean `ok`, or a non-array
diagnostic collection must not leak schema-invalid content to an MCP caller.

## Decision

- Parse the native JSON using the existing fail-closed parser.
- Require exactly the three declared top-level fields.
- Require `ok` to be a boolean and both diagnostic fields to be arrays.
- Reject every invalid shape as a stable MCP tool error before structured
  content is returned; nested diagnostic item semantics remain the native
  schema boundary.

## Evidence

`scripts/ci/native_selfhost_mcp_check_tests.py` covers array, `null`,
malformed, missing-field, unknown-field, non-boolean, and non-array outputs.
The complete native MCP suite passes with 57 tests, alongside runner tests,
Python compilation, docs audit, and diff checks.

## Remaining boundary

Nested diagnostic/migration item validation, full Rust MCP parity, provider
authentication/signature/lifecycle semantics, native stage0 report parity, and
current-source Linux runtime evidence remain `[~]` under `EC-M3-05` /
M3-05-N9.
