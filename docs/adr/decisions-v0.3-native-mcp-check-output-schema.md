# ADR: v0.3 native MCP check output schema

## Status

Verified partial slice (2026-08-01). Native MCP now publishes the structured
`lsharp_check` migration diagnostic schema used by the Rust MCP contract.

## Decision

- Keep `ok`, `diagnostics`, and `migrationDiagnostics` as the stable top-level
  output fields.
- Describe each migration diagnostic's code, owner, selected semantics,
  disposition, source range, and optional message.
- Reuse a local JSON Schema `$defs/position` for zero-based line/character
  positions, matching the Rust MCP schema.

## Evidence

`scripts/ci/test-native-selfhost-mcp.py` asserts the published enums, required
fields, range references, and focused native-only behavior. Python compilation,
runner tests, shell syntax checks, docs audit, and `git diff --check` pass.

## Remaining boundary

Diagnostic item parity beyond migration diagnostics, full Rust MCP tool parity,
provider authentication/signature/lifecycle semantics, and current-source
Linux runtime evidence remain `[~]` under `EC-M3-05` / M3-05-N9.
