# ADR: v0.3 native selfhost MCP subset shim

## Status

Verified partial slice (2026-08-01). Native MCP is no longer an unconditional
`mcp-server` rejection, but full tool parity and provider verification remain
active work.

## Context

The Rust host already exposes MCP over newline-delimited JSON-RPC, while the
native selfhost runner previously stopped before executing any MCP request.
That made the public `mcp-server` entry point a Rust-only surface and gave no
native evidence for the current source/runtime milestone. The Linux replay is
also a shared resource, so this boundary must be testable without starting a
VM or regenerating stage0.

## Decision

- Add `scripts/native-selfhost-mcp.py` as a thin stdio adapter around the
  native `program.native`; it never invokes `cargo`, `rustc`, host `lsharp`,
  network access, or a provider helper.
- Advertise only the deterministic native subset currently implemented by
  `App.Cli`: `lsharp_check`, `lsharp_validate`, and `lsharp_format`.
- Preserve MCP JSON-RPC `initialize`, `ping`, `tools/list`, and `tools/call`
  envelopes. Child exit codes `1`/`2` with valid JSON reports remain structured
  tool results, while empty/malformed output is an `isError` result.
- `validate` forwards explicit identity digest fields and can attach an
  emitted manifest. `trust_store` and `review_lifecycle` file paths are
  rejected before native execution; obtaining and authenticating provider
  snapshots remains the caller/provider-adapter boundary.
- Unsupported MCP tools return an MCP tool error instead of silently falling
  back to the Rust host.

## Evidence

- `scripts/ci/test-native-selfhost-mcp.py`: four focused tests cover protocol
  discovery, native-only check/validate/format calls, identity forwarding,
  malformed input, missing executable, and provider-path fail-closed behavior.
- `scripts/ci/test-native-selfhost-dev.sh`: runner wiring test confirms
  `mcp-server` delegates to the shim and does not execute `program.native`
  directly or a host command.
- `python3 -m py_compile scripts/native-selfhost-mcp.py` and `bash -n`
  checks pass.

## Remaining boundary

The subset does not yet implement the Rust MCP tools for LSP intelligence,
package APIs, compile/run, search, or external provider snapshot acquisition
and signature/lifecycle verification. N9 / `EC-M3-05` therefore remains
`[~]`; the next RED should select one additional tool or the explicit provider
adapter contract and compare Rust/native output with the same fixture.
