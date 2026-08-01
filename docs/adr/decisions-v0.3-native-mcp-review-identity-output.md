# ADR: v0.3 native MCP review identity output

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP `review_evidence_identity` report postflight

## Context

The native MCP `lsharp_validate` shim only checked that an optional
`review_evidence_identity` report field was an object. When an existing manifest supplied review
context without explicit identity arguments, malformed identity fields could pass through even
though the advertised output schema required six named fields and nullable digest types.

## Decision

- Validate the identity object whenever it is present in a native validate report.
- Reject unknown fields and require `subject_digest`, `source_commit`, `artifact_digest`,
  `trust_store_digest`, `lifecycle_digest`, and `now` in schema order.
- Require non-empty strings for the four non-nullable fields and allow only non-empty strings or
  `null` for the two provider digest fields.
- Keep the existing explicit-identity equality and field-order verification unchanged.

## Evidence

- RED: manifest-input calls accepted missing, extra, and wrongly typed identity fields.
- GREEN: malformed identity objects fail closed without a traceback; a complete nullable identity
  is returned unchanged.
- Full native MCP suite passes: 72 tests.
- `python3 -m py_compile scripts/native-selfhost-mcp.py scripts/ci/native_selfhost_mcp_validate_tests.py scripts/ci/test-native-selfhost-mcp.py` passes.

This closes only the native validate identity report shape. Manifest nested runtime validation,
native stage0, provider semantics, and full Rust/native parity remain active `[~]` boundaries in
`TODO.md`.
