# ADR: v0.3 native MCP manifest edges

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP emitted manifest `edges`

## Context

The native MCP shim validated only that manifest `edges` was an array. The advertised schema
contains six relation variants with closed fields, typed IDs, and constrained subject kinds, so a
malformed relationship could be returned as a successful manifest.

## Decision

- Validate all six relation variants: `motivates`, `constrained-by`, `tested-by`, `supports`,
  `contradicts`, `evaluates`, and `invalidates`.
- Require the relation-specific fields and reject unknown or missing fields.
- Validate ID objects (`namespace`/`key`) and subject objects with relation-specific kind enums.
- Keep evidence nested validation as a separate slice; edge validation does not infer graph
  referential integrity between IDs and node/evidence collections.

## Evidence

- RED: malformed relation, ID, subject, and extra-field edges were accepted by array-only
  validation.
- GREEN: all invalid edge fixtures fail closed and a manifest containing valid `motivates`,
  `supports`, and `evaluates` edges is preserved.
- Full native MCP suite passes: 74 tests.
- `python3 -m py_compile scripts/native-selfhost-mcp.py scripts/ci/native_selfhost_mcp_manifest_tests.py scripts/ci/test-native-selfhost-mcp.py` passes.

This closes only emitted manifest edge shape. Evidence nested validation, referential integrity,
native stage0, provider semantics, and full Rust/native parity remain active `[~]` boundaries in
`TODO.md`.
