# ADR: v0.3 native MCP review clock lexical schema

## Status

Verified partial slice (2026-08-01). The native MCP `lsharp_validate` input
schema and shim preflight now share the canonical UTC timestamp lexical boundary
for `review_now`.

## Context

The Rust MCP contract publishes `review_now` as a fixed UTC timestamp with the
shape `YYYY-MM-DDTHH:MM:SSZ`. Native MCP previously advertised only
`minLength` and forwarded whitespace-separated, offset, or fractional values to
the native program. That allowed the wire schema and the preflight boundary to
disagree about whether an explicit review clock was well formed.

## Decision

- Publish the exact canonical UTC lexical pattern in the native
  `tools/list` schema.
- Reject a non-canonical `review_now` before reading input files or invoking
  `program.native`.
- Keep calendar validity (for example, rejecting a nonexistent February 30)
  in the native/Rust canonical timestamp validator. The JSON Schema pattern is
  intentionally lexical and does not duplicate calendar arithmetic.

## Evidence

- `scripts/ci/test-native-selfhost-mcp.py` asserts the schema pattern and
  verifies that a non-canonical review clock is a no-execution MCP error.
- The native MCP focused suite (10 tests), Python compilation, runner contract,
  shell syntax, docs audit, and `git diff --check` pass.

## Boundary

This closes only the native MCP lexical input contract. Calendar validation,
full Rust MCP tool parity, provider authentication/signature/lifecycle
semantics, and current-source Linux runtime evidence remain `[~]` under
`EC-M3-03` / `EC-M3-05` / `M3-05-N9`.
