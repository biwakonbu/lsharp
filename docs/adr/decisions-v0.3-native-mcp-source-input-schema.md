# ADR: v0.3 native MCP source input schema

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP source/file input boundary

## Context

The native MCP shim already rejected empty and whitespace-only `source` values in its shared
runtime input helper, but the `tools/list` schemas advertised only `type: string`. Callers could
therefore not apply the same non-empty preflight before invoking a tool. The mismatch affected
the four native LSP tools, `lsharp_check`, `lsharp_validate`, `lsharp_format`, and
`lsharp_compile_run`, which all reuse the common source/file schema.

## Decision

- The shared `source` property advertises `type: string` and `minLength: 1` for every source-based
  native MCP tool.
- The existing runtime helper remains the authoritative whitespace policy and rejects empty or
  whitespace-only source values before the native program starts.
- The `file` property keeps its existing `minLength: 1` schema and regular-file checks.

## Evidence

- RED: the schema contract test failed with a missing `source.minLength` field.
- GREEN: schema coverage confirms all eight source-based tools expose `minLength: 1`; `check` and
  `format` reject empty and whitespace-only source values without creating a native log.
- Full native MCP suite passes: 68 tests.
- `python3 -m py_compile scripts/native-selfhost-mcp.py scripts/ci/native_selfhost_mcp_check_tests.py scripts/ci/native_selfhost_mcp_format_tests.py scripts/ci/test-native-selfhost-mcp.py` passes.

This closes only the source input schema/runtime preflight mismatch. Native stage0 runtime,
provider semantics, and full Rust/native parity remain active `[~]` boundaries in `TODO.md`.
