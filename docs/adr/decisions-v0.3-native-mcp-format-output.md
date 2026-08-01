# ADR: v0.3 native MCP format output boundary

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP `lsharp_format`

## Context

The native MCP shim delegates `lsharp_format` to the native `fmt` command and wraps its stdout as
`{"formatted": ...}`. Before this decision, the advertised output schema was open-world and a native
formatter that returned formatted stdout with a non-zero exit status was reported as a successful MCP
call. That could turn a failed formatting operation into a misleading success.

## Decision

- Keep formatted source as an opaque UTF-8 string; the shim must not parse or reinterpret L# source text.
- Advertise a closed output object with exactly the required `formatted` string field.
- Treat any non-zero native `fmt` exit as an MCP tool error, preserving the exit status and stderr detail.
- Preserve the existing empty-stdout failure in the shared native execution boundary.

## Evidence

- RED: format schema and native non-zero tests failed before the implementation.
- GREEN: `python3 scripts/ci/test-native-selfhost-mcp.py -k format_` passes.
- Full native MCP suite passes: 60 tests.
- `python3 -m py_compile scripts/native-selfhost-mcp.py scripts/ci/native_selfhost_mcp_format_tests.py scripts/ci/test-native-selfhost-mcp.py` passes.

This closes only the native format output boundary. Native stage0/current-source runtime, full Rust MCP
parity, and packaged target evidence remain active `[~]` boundaries in `TODO.md`.
