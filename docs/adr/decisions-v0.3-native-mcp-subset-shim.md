# ADR: v0.3 native selfhost MCP subset shim

## Status

Verified partial slice (2026-08-01). Native MCP is no longer an unconditional
`mcp-server` rejection, but full tool parity and provider verification remain
active work. The provider-path rejection described by this initial slice is
superseded by
[`decisions-v0.3-native-mcp-provider-snapshot-adapter.md`](decisions-v0.3-native-mcp-provider-snapshot-adapter.md).

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
- Advertise only the deterministic subset currently implemented by the native
  `App.Cli` boundary: `lsharp_check`, `lsharp_validate`, `lsharp_format`,
  `lsharp_errors`, the offline local-package `lsharp_search` projection, the
  offline `lsharp_project_context` projection, and the offline
  `lsharp_package_api` projection. The docs lookup, package context/search,
  and package API lookup never execute a Rust or host compiler and never access
  a registry or network.
- `lsharp_package_api` resolves a deterministic installed-package directory and
  reads its existing `docs/api.json`. The native subset does not generate or
  mutate that file; package installation and API generation remain outside this
  offline boundary.
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

- `scripts/ci/test-native-selfhost-mcp.py`: 23 focused tests cover protocol
  discovery, native-only check/validate/format calls, canonical error lookup
  (LS codes, E0001-E0005 aliases, unknown codes, and no native execution),
  offline installed-package search, project context (TOML project/dependency
  projection), and package API (existing `docs/api.json` projection), including
  deterministic ordering, schema, argument rejection, and no native execution,
  identity forwarding, malformed input, missing executable, and provider-path
  fail-closed behavior. Error, package, and context assertions live in separate
  helper modules to keep the main test module within the repository file-size
  limit.
- `crates/lsharp-driver` schema and unit tests require the same closed-world
  `lsharp_errors`, `lsharp_search`, `lsharp_project_context`, and
  `lsharp_package_api` boundaries; the Rust MCP focused suite passes 62 tests.
- `scripts/ci/test-native-selfhost-dev.sh`: runner wiring test confirms
  `mcp-server` delegates to the shim and does not execute `program.native`
  directly or a host command.
- `python3 -m py_compile scripts/native-selfhost-mcp.py` and `bash -n`
  checks pass.

## Remaining boundary

The subset does not yet implement the Rust MCP tools for LSP intelligence,
package API generation/validation semantics, compile/run, or external provider
snapshot acquisition and signature/lifecycle verification. `lsharp_errors` is only a verified
documentation-table projection, `lsharp_search` is only a verified offline
installed-package projection, and `lsharp_project_context` is only a verified
offline TOML/package projection; `lsharp_package_api` is only a verified
existing-`docs/api.json` projection, not native compiler/package-install
semantics. N9 / `EC-M3-05`
therefore remains `[~]`; the next RED should select one additional tool or the
explicit provider adapter contract and compare Rust/native output with the same
fixture.
