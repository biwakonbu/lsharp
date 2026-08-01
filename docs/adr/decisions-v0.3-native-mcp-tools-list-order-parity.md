# ADR: v0.3 native MCP tools/list order parity

## Status

Verified partial slice (2026-08-02). This decision fixes only the deterministic
order of the `tools/list` descriptors.

## Context

The Rust canonical MCP registry lists the supported tools in a stable order:
`lsharp_check`, `lsharp_validate`, `lsharp_hover`, `lsharp_completion`,
`lsharp_format`, `lsharp_definition`, `lsharp_references`,
`lsharp_project_context`, `lsharp_package_api`, `lsharp_stdlib_api`,
`lsharp_compile_run`, `lsharp_errors`, and `lsharp_search`. The native shim
exposed the same set but started with the LSP tools and therefore produced a
different observable `tools/list` array. The existing test compared only the
set of names and did not detect this route difference.

## Decision

Retain the existing native descriptors and sort the published list by an
explicit canonical order tuple matching Rust `list_tools()`. The focused test
asserts the complete ordered array, so adding a native route without updating
the canonical order fails closed at module initialization and requires an
explicit parity decision.

This is an MCP route-envelope contract, separate from `initialize` server
identity, ReviewId/lifecycle, provider/auth, and artifact provenance.

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k initialize_tools_and_supported_calls_stay_native_only`
  failed because native returned the LSP-first order.
- GREEN: the same focused test passed after applying the canonical order; all
  descriptor schemas and the existing route name set remain covered by that
  test.
- Canonical source: `crates/lsharp-driver/src/mcp_server.rs`, `list_tools()`.

## Remaining boundary

This does not prove full Rust/native MCP semantic parity, package-install
semantics, live provider API/auth acquisition or verification, current-source
Linux runtime, or Mac/Linux packaged and rollback bytes parity. Those remain
`[~]` work in the current planning/TODO record.
