# ADR: v0.3 native MCP tools/call missing-name error envelope

## Status

Verified partial slice (2026-08-02). This decision fixes the
`tools/call` missing-name result envelope only.

## Context

The Rust canonical MCP transport defaults a missing `params.name` to an
empty tool name and sends it through `call_tool`. The canonical result is a
JSON-RPC result whose payload has `isError: true` and text `tool not found`.
The native shim instead returned a different `tool name が必要です` message,
so the same malformed route produced different observable error text.

## Decision

Normalize a missing or non-string `tools/call` name to the empty tool name and
reuse the native unknown-tool result path. Both transports therefore return
the same `jsonrpc: "2.0"`, request id, result-level `isError: true` envelope
with `content[0].text: "tool not found"`. This does not change the already
matching explicit unknown-tool-name route.

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k
  tools_call_missing_name_matches_rust_unknown_tool_result_envelope` failed
  because native returned `tool name が必要です`.
- GREEN: the same native focused test passed after the shim change.
- Rust oracle: the matching `mcp_server::tests::test_tools_call_missing_name_uses_unknown_tool_result_envelope`
  passed, and the MCP module suite passed with 89 tests.
- Native MCP suite passed with 82 tests.

## Remaining boundary

This slice does not prove full MCP error/semantic parity, all malformed
`params` shapes, package-install semantics, live provider API/auth
acquisition or verification, current-source Linux runtime, or Mac/Linux
packaged and rollback bytes parity. Those remain `[~]` work in the current
planning/TODO record.
