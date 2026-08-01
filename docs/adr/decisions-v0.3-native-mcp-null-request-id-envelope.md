# ADR: v0.3 native MCP null request-id envelope parity

## Status

Verified partial slice (2026-08-02). This decision fixes the JSON-RPC
request-id envelope distinction only.

## Context

The Rust canonical transport checks whether the `id` field exists. A missing
`id` is a notification and produces no response, while an explicit JSON
`id: null` is preserved in the response envelope. The native shim used
`request.get("id")` followed by a `None` check, so it incorrectly dropped
explicit-null ids as if they were notifications.

## Decision

Treat only an absent `id` field as a notification. Preserve an explicit null
value through `jsonrpc_result`, so a request such as `{"jsonrpc":"2.0",
"id":null,"method":"ping"}` returns a JSON-RPC 2.0 result with `id: null`.
This is an envelope contract, separate from `initialize` identity,
`tools/list` order, ReviewId/lifecycle, provider/auth, and artifact provenance.

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k jsonrpc_null_request_id_is_preserved_in_response_envelope`
  returned no response from the native shim.
- GREEN: the same focused test passed after distinguishing a missing id field
  from an explicit null id; the fake native program was not invoked.
- Rust canonical source: `crates/lsharp-driver/src/mcp_protocol.rs` uses
  `request.get("id").cloned()` and responds whenever the field exists.

## Remaining boundary

This does not prove all MCP error envelopes or full Rust/native semantic
parity, package-install semantics, live provider API/auth acquisition or
verification, current-source Linux runtime, or Mac/Linux packaged and
rollback bytes parity. Those remain `[~]` work in the current planning/TODO
record.
