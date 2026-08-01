# ADR: v0.3 native MCP strict JSON duplicate-key boundary

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP JSON input and output parsing

## Context

Python's default JSON decoder keeps the last value for duplicate object keys.
That could make a duplicate field in an MCP request, direct/file manifest input,
native report, or emitted manifest silently change the observable contract. The
Rust manifest boundary already rejects duplicate keys before graph construction.

## Decision

- Parse the core shim JSON objects with an object-pairs hook that rejects
  duplicate keys, including nested objects.
- Apply the same strict parser to JSON-RPC requests, native stdout, direct and
  file manifest input, and emitted manifest files.
- Preserve fail-closed tool errors and process-level invalid-JSON diagnostics;
  never return structured content after duplicate-key input or output.

## Evidence

- RED: duplicate `id`, `schema_version`, and report `status` keys were silently
  accepted or reduced to the last value.
- GREEN: duplicate-key requests, manifest inputs, native reports, and emitted
  manifests now fail without native execution where applicable and without a
  traceback.
- Focused duplicate-key tests and the complete native MCP suite pass: 78 tests.
- Python compilation, docs audit, and `git diff --check` pass.

This closes only the native MCP JSON duplicate-key boundary. Provider semantics,
target runtime, and full Rust/native parity remain active `[~]` boundaries in
`TODO.md`.
