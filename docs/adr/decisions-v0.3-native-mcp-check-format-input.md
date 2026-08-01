# ADR: v0.3 native MCP check/format input boundary

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP `lsharp_check` and `lsharp_format`

## Context

The native MCP shim already exposed `source`/`file` alternatives for check and format, but their
runtime helpers ignored unknown fields. The corresponding `tools/list` schemas also omitted
`additionalProperties: false`, so callers could send an input that was silently discarded and still
reach the native program.

## Decision

- `lsharp_check` and `lsharp_format` accept exactly one of `source` or `file`.
- Unknown arguments are rejected before the native program starts.
- Both tools advertise the same closed-world input schema with `additionalProperties: false`.
- Existing source/file loading and native format output behavior remain unchanged.

## Evidence

- RED: unknown and source+file inputs reached the fake native program; schema tests found the missing
  closed-world flag.
- GREEN: `python3 scripts/ci/test-native-selfhost-mcp.py -k check_rejects_invalid_arguments` and
  `-k format_rejects_invalid_arguments` pass.
- Full native MCP suite passes: 65 tests.
- `python3 -m py_compile scripts/native-selfhost-mcp.py scripts/ci/native_selfhost_mcp_check_tests.py scripts/ci/native_selfhost_mcp_format_tests.py scripts/ci/test-native-selfhost-mcp.py` passes.

This closes only the check/format input boundary. Native stage0 runtime, provider semantics, and
full Rust/native parity remain active `[~]` boundaries in `TODO.md`.
