# ADR: v0.3 native MCP manifest evidence

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP emitted manifest `evidence`

## Context

The native MCP shim validated the emitted manifest's `evidence` field only as an array. The
advertised schema defined a full evidence record with subject, execution/sampling, provenance,
and bounded outcome fields, so malformed records could be returned as successful manifests.

## Decision

- Validate evidence item closed fields and all required top-level fields.
- Validate identifier fields, method/outcome/independence enums, and the constrained subject kind.
- Validate execution runner/target/digests and nested sampling cases, seed, generator, shrinks,
  and coverage counters/property names.
- Validate provenance producer, tool version, and timestamp as non-empty strings.
- Do not infer referential integrity between evidence subjects and node/edge collections; that is a
  separate graph-consistency boundary.

## Evidence

- RED: malformed evidence records were accepted by array-only manifest validation.
- GREEN: missing/unknown fields, invalid enums, subject kinds, execution/sampling values, and
  empty provenance fields fail closed; a complete evidence record is preserved.
- Full native MCP suite passes: 75 tests.
- `python3 -m py_compile scripts/native-selfhost-mcp.py scripts/ci/native_selfhost_mcp_manifest_tests.py scripts/ci/test-native-selfhost-mcp.py` passes.

This closes only emitted manifest evidence shape. Referential integrity, native stage0, provider
semantics, and full Rust/native parity remain active `[~]` boundaries in `TODO.md`.
