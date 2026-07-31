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
  `App.Cli` boundary: `lsharp_hover`, `lsharp_definition`, `lsharp_references`, `lsharp_completion`, `lsharp_check`, `lsharp_validate`, `lsharp_format`,
  `lsharp_errors`, the offline local-package `lsharp_search` projection, the
  offline `lsharp_project_context` projection, and the offline
  `lsharp_package_api` and `lsharp_stdlib_api` projections. The docs
  lookup, package context/search, package API lookup, and stdlib metadata lookup
  never execute a Rust or host compiler and never access a registry or network.
- `lsharp_hover` sends an initialize, did-open, and hover sequence to the native
  `lsp --stdio` boundary and projects the scalar signature/doc result to the MCP
  shape. Source and file inputs use the same position contract, including the
  `col` compatibility alias; malformed frames, child diagnostics, and missing or
  invalid hover responses fail closed without a host LSP fallback.
- `lsharp_definition` sends the same source/file and position contract to the
  native `lsp --stdio` boundary and projects a single LSP location range to the
  closed MCP `{start, end}` shape. Source and file routes accept the `col`
  compatibility alias; malformed frames, child diagnostics, missing responses,
  ambiguous locations, and invalid range positions fail closed without a host
  LSP fallback.
- `lsharp_references` sends the same source/file and position contract with
  `includeDeclaration: true` to the native `lsp --stdio` boundary and projects
  the returned locations to the closed MCP `{count, ranges}` shape. A native
  `null` result is the empty reference set; malformed frames, child diagnostics,
  missing responses, and invalid location ranges fail closed without a host LSP
  fallback.
- `lsharp_completion` sends the same source/file and position contract to the
  native `lsp --stdio` boundary and projects LSP arrays or completion lists to
  the closed MCP `{items}` shape. Numeric LSP completion kinds are mapped to
  the Rust MCP names; a native `null` result is the empty candidate set, while
  malformed results, invalid items, child diagnostics, and missing responses
  fail closed without a host LSP fallback.
- `lsharp_package_api` resolves a deterministic installed-package directory.
  When `docs/api.json` exists it is read without invoking the native program and
  validated against the full closed-world API shape. When the artifact is absent,
  the shim enumerates sorted `src/**/*.ls` files, invokes the native program's
  read-only `doc <source> --json` contract for each file, maps the validated
  documents to the package API shape in memory, and never writes `docs/api.json`.
  Package installation and the native stage0/runtime boundary remain outside
  this offline projection.
- `lsharp_stdlib_api` reads the checked-in `stdlib/api.json`, generated from the
  Rust canonical `doc --json` output for every standard-library module. When
  that artifact is absent, it enumerates sorted direct `stdlib/*.ls` sources and
  invokes the native program's read-only `doc --json` contract, mapping the
  validated documents in memory without writing `api.json`. Native filtering is
  limited to an optional non-empty module name; artifact shape and Rust/native
  semantic equality are covered by tests.
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

- `scripts/ci/test-native-selfhost-mcp.py`: 47 focused tests cover protocol
  discovery, native LSP hover/definition/references/completion, native-only check/validate/format calls, canonical error lookup
  (LS codes, E0001-E0005 aliases, unknown codes, and no native execution),
  offline installed-package search, project context (TOML project/dependency
  projection), package API (existing `docs/api.json` projection and missing-file
  native `doc --json` generation with full closed-world validation), and stdlib API
  (generated `stdlib/api.json` projection and missing-artifact native `doc --json`
  generation), including deterministic ordering, schema, argument rejection,
  malformed native-doc fail-closed behavior, and no artifact mutation,
  identity forwarding, malformed input, missing executable, and provider-path
  fail-closed behavior. Error, package, and context assertions live in separate
  helper modules to keep the main test module within the repository file-size
  limit.
- `scripts/ci/test-native-selfhost-lsp-stdio.py`: 5 frame-relay tests remain
  green, including child stderr/nonzero, malformed/truncated frame, and replay
  prefix rejection cases used by the MCP hover, definition, references, and completion adapters.
- `crates/lsharp-driver` schema and unit tests require the same closed-world
  `lsharp_errors`, `lsharp_search`, `lsharp_project_context`, `lsharp_package_api`,
  and `lsharp_stdlib_api` boundaries; `cargo test -p lsharp-driver
  mcp_server::tests` passes 87 tests, including the six package-API cases and
  equality with `stdlib/api.json`. The driver unit target passes 214 tests;
  the separate `default_path_delegation` integration target remains at 34/46;
  its 12 failures are embedded-component/default-path assertions outside this
  MCP slice.
- `scripts/ci/test-native-selfhost-dev.sh`: runner wiring test confirms
  `mcp-server` delegates to the shim and does not execute `program.native`
  directly or a host command.
- `python3 -m py_compile scripts/native-selfhost-mcp.py` and `bash -n`
  checks pass.

## Remaining boundary

The subset does not yet implement the Rust MCP tools for full LSP intelligence,
package installation semantics, compile/run, or external provider
snapshot acquisition and signature/lifecycle verification. `lsharp_errors` is only a verified
documentation-table projection, `lsharp_search` is only a verified offline
installed-package projection, and `lsharp_project_context` is only a verified
offline TOML/package projection; `lsharp_package_api` is a verified existing
`docs/api.json` projection plus a no-mutation native `doc --json` source
projection with closed-world shape validation, and `lsharp_stdlib_api` is only a verified
generated-artifact or missing-artifact native `doc --json` source projection; `lsharp_hover`,
`lsharp_definition`, `lsharp_references`, and `lsharp_completion` are verified native LSP stdio
projections, while full native compiler/package-install semantics remain outside this slice. N9 / `EC-M3-05`
therefore remains `[~]`; the next RED should select one additional LSP tool,
compile/run boundary, or the explicit provider adapter contract and compare Rust/native
output with the same fixture.
