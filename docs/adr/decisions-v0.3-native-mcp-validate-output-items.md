# ADR: v0.3 native MCP validate output items

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP `lsharp_validate` report item postflight

## Context

The native MCP `lsharp_validate` shim checked the report root, top-level fields, collection
types, status, and numeric counters. Its advertised schema also constrained `trace_gaps` and
`review_verifications` item objects, but malformed nested values were forwarded as a successful
report.

## Decision

- Validate every `trace_gaps` item: closed fields, required `code`/`subject_id`, the two trace-gap
  codes, and non-empty string subject IDs.
- Validate every optional `review_verifications` item: closed fields, required `review_id`/`state`,
  the canonical review ID pattern, and the four lifecycle states.
- Preserve the existing top-level and count validation behavior.
- Leave `review_evidence_identity` and manifest nested validation as separate active boundaries;
  their root/type checks and identity projection checks remain unchanged.

## Evidence

- RED: malformed trace-gap and review-verification items were accepted by the previous
  collection-only validator.
- GREEN: missing fields, unknown fields, invalid codes/IDs/states are rejected without a
  traceback; a valid nested report is accepted unchanged.
- Full native MCP suite passes: 70 tests.
- `python3 -m py_compile scripts/native-selfhost-mcp.py scripts/ci/native_selfhost_mcp_validate_tests.py scripts/ci/test-native-selfhost-mcp.py` passes.

This closes only the native `lsharp_validate` report item boundary. Identity, manifest runtime,
native stage0, provider semantics, and full Rust/native parity remain active `[~]` boundaries in
`TODO.md`.
