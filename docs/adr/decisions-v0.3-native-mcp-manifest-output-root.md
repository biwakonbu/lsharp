# ADR: v0.3 native MCP manifest output root boundary

## Status

Verified partial slice (2026-08-01). Native MCP `lsharp_validate` now
validates the root contract of an emitted manifest before returning it to the
caller.

## Context

The native MCP output schema requires a versioned manifest object with
`schema_version`, `nodes`, `evidence`, and `edges`. A broken native stage0 or
helper could instead emit an array, `null`, malformed JSON, a missing required
field, an unknown top-level field, or a non-array collection. Returning that
value as structured content would violate the MCP schema and could turn later
identity checks into an unstable traceback.

## Decision

- Parse the emitted manifest and reject malformed JSON as a tool error.
- Require a JSON object, `schema_version: 1`, the four required root fields,
  and array values for `nodes`, `evidence`, and `edges`.
- Reject unknown top-level fields before identity postflight; optional
  `reviews` and `review_evidence_identity` remain allowed for the existing
  projection contract.
- Keep nested node/evidence/edge semantic validation in the native schema
  boundary; this slice only hardens the output root contract.
- Never return a traceback or schema-invalid structured content to the MCP
  caller.

## Evidence

`scripts/ci/native_selfhost_mcp_manifest_tests.py` drives fake native output
through array, `null`, malformed, missing-field, unknown-field, and wrong-type
root cases. The complete native MCP suite passes with 55 tests, including the
existing input-root, identity, provider, package, LSP, and compile/run
contracts. Python compilation, runner tests, docs audit, and diff checks also
pass.

## Remaining boundary

Nested manifest runtime validation, provider authentication/signature/lifecycle
semantics, native stage0 report parity, and current-source Linux runtime
evidence remain `[~]` under `EC-M3-05` / M3-05-N9.
