# ADR: v0.3 native MCP tools/call non-object params envelope

## Status

Verified partial slice (2026-08-02). This decision fixes the
`tools/call` non-object `params` result envelope only.

## Context

The Rust canonical transport keeps a non-object `params` value, then reads
the missing tool name as an empty string. `call_tool` consequently returns a
JSON-RPC result with `isError: true` and text `tool not found`. The native shim
instead rejected the params shape with a different `params は object が必要です`
message, so an array, string, or null params value drifted from the canonical
route.

## Decision

Normalize a non-object `tools/call` params value to an empty object before
reading `name` and `arguments`. This preserves the Rust/native result-level
error envelope for the non-object boundary without changing the separate
missing-name or explicit unknown-tool-name contracts.

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k
  tools_call_non_object_params_matches_rust_unknown_tool_result_envelope`
  failed because native returned `params は object が必要です`.
- GREEN: the same native focused test passed after normalization.
- Rust oracle: `mcp_server::tests::test_tools_call_non_object_params_uses_unknown_tool_result_envelope`
  passed.
- Native MCP suite passed with 83 tests; Rust MCP module suite passed with
  90 tests.

## Remaining boundary

This slice does not prove every malformed JSON-RPC or MCP error envelope,
package-install semantics, live provider API/auth acquisition or verification,
current-source Linux runtime, or Mac/Linux packaged and rollback bytes parity.
Those remain `[~]` work in the current planning/TODO record.
