# ADR: v0.3 native MCP initialize server identity parity

## Status

Verified partial slice (2026-08-02). This decision covers only the
`initialize` response metadata; it does not complete the native MCP tool set,
provider semantics, or target runtime evidence.

## Context

The Rust canonical MCP transport returns `serverInfo.name` as `lsharp` and
uses the package version (`0.1.0` at this checkout). The native JSON-RPC shim
returned `lsharp-native` instead, so clients could observe different server
identity for the same `initialize` route even though the protocol version and
tool registry matched.

## Decision

Keep the native shim's existing protocol version and deterministic subset, but
return the Rust-canonical server identity `{ "name": "lsharp", "version":
"0.1.0" }` from `initialize`. This is an envelope metadata parity contract,
separate from ReviewId, lifecycle, provider, and artifact provenance checks.

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k initialize_tools_and_supported_calls_stay_native_only`
  failed because native returned `lsharp-native`.
- GREEN: the same focused test passed after the native response was aligned;
  the test now fixes both server identity fields and the existing tool route
  inventory.
- Rust canonical source: `crates/lsharp-driver/src/mcp_protocol.rs` uses
  `name: "lsharp"` and `env!("CARGO_PKG_VERSION")`; `Cargo.toml` is `0.1.0`.

## Remaining boundary

This does not prove full Rust/native MCP parity, package-install semantics,
live provider API/auth acquisition or semantic verification, current-source
Linux runtime, or Mac/Linux packaged and rollback bytes parity. Those remain
`[~]` work in the current planning/TODO record.
