# ADR: v0.3 native MCP validation report root boundary

## Status

Verified partial slice (2026-08-01). Native MCP `lsharp_validate` now validates
the report root and required scalar/collection fields before returning
structured content.

## Context

The Rust MCP contract defines a closed validation report with seven required
fields. A native program that emits an array, `null`, malformed JSON, missing
fields, unknown fields, an invalid status, or a boolean where a count is
expected must not make the shim return schema-invalid content or raise an
uncaught Python exception during identity postflight.

## Decision

- Parse the native JSON using the existing fail-closed parser.
- Require an object containing `status`, the five non-negative count fields,
  and `trace_gaps`.
- Allow only the declared optional `review_evidence_identity`,
  `review_verifications`, and emitted `manifest` fields.
- Restrict status to `pass`, `fail`, or `unknown`; reject booleans and negative
  values for count fields; require list values for collection fields.
- Validate an optional emitted manifest through the manifest root contract
  before identity postflight.
- Return a stable MCP tool error with no traceback for every rejected report.

## Evidence

`scripts/ci/native_selfhost_mcp_validate_tests.py` covers array, `null`,
malformed, missing-field, unknown-field, invalid-status, and boolean-count
reports. The complete native MCP suite passes with 56 tests, including the
manifest input/output, identity, provider, package, LSP, and compile/run
contracts. Python compilation, runner tests, docs audit, and diff checks pass.

## Remaining boundary

Nested report item semantics, nested manifest runtime validation, provider
authentication/signature/lifecycle semantics, native stage0 report parity, and
current-source Linux runtime evidence remain `[~]` under `EC-M3-05` /
M3-05-N9.
