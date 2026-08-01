# ADR: v0.3 native MCP check output items

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP `lsharp_check` output postflight

## Context

The native MCP `lsharp_check` shim validated the report root, required fields, the boolean
`ok`, and the two array types. Its advertised schema, however, also specified the shape of each
`migrationDiagnostics` item: a bounded code and semantic/disposition enum, owner, range, and
non-negative line/character positions. Malformed nested items could therefore pass through as a
successful `structuredContent` response.

## Decision

- Validate every `migrationDiagnostics` item after native JSON parsing.
- Require `code`, `owner`, `selectedSemantics`, `disposition`, and `range`.
- Enforce the schema enums, object/range/position shapes, non-negative integer coordinates, and
  the optional string `message`.
- Preserve the advertised schema's open nested-object behavior for unknown item, range, and
  position fields; only declared type and required-field contracts are enforced.
- Keep the generic `diagnostics` array opaque because its advertised schema does not declare item
  fields.

## Evidence

- RED: malformed nested items were accepted by the previous array-only validator.
- GREEN: missing fields, invalid enum values, non-object ranges, and negative positions are
  rejected without a traceback; a complete migration item is accepted unchanged.
- Full native MCP suite passes: 69 tests.
- `python3 -m py_compile scripts/native-selfhost-mcp.py scripts/ci/native_selfhost_mcp_check_tests.py scripts/ci/test-native-selfhost-mcp.py` passes.

This closes only the native `lsharp_check` nested output boundary. Native stage0 runtime,
provider semantics, and full Rust/native parity remain active `[~]` boundaries in `TODO.md`.
