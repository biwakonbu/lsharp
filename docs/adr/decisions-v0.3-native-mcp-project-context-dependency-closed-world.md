# ADR: v0.3 native MCP project-context dependency closed world

## Status

Verified partial slice (2026-08-02). Rust MCP and the native MCP shim now
apply the same fail-closed source-shape boundary to offline
`lsharp_project_context` dependency projections.

## Context

`lsharp_project_context` is a read-only projection, but its dependency summary
is still an observable provider/package boundary. The native shim previously
selected `path` before inspecting other keys and silently omitted unsupported
dependency tables. Its fallback TOML reader also discarded unknown attributes.
Rust deserialization could similarly accept an ambiguous table by selecting the
Git variant. That made the two projections disagree and could hide an
unintended provider source.

## Decision

- A dependency is either a non-empty registry version string or a source table
  containing exactly one of `path` and `git`.
- Source tables are closed-world: unknown attributes are rejected. `branch`
  and `tag` are valid only with `git`; source values and Git selectors must be
  non-empty strings.
- Rust MCP validates the raw `lsharp.toml` dependency table before loading the
  projected config. The native shim applies the same checks, including its
  Python fallback TOML path, and never silently drops an invalid declaration.
- This route remains offline and read-only. It does not add registry/network
  acquisition, installer mutation, provider authentication, or cryptographic
  verification.

## Evidence

- Rust `mcp_server::tests` covers the existing valid projection and four
  invalid dependency-source cases (5 focused project-context tests total).
- `scripts/ci/test-native-selfhost-mcp.py -k project_context` covers the valid
  projection, invalid arguments, and the same four invalid source cases (3
  focused tests total); invalid requests do not invoke the fake native program.
- Python AST syntax and `git diff --check` pass for the task files.

## Boundary

This closes only the offline `lsharp_project_context` dependency source-shape
projection. It does not implement MCP package installation, registry/cache or
live provider/auth acquisition, full Rust/native MCP parity, current-source
Mac/Linux runtime, or packaged/rollback target evidence. Those remain `[~]` in
`EC-M3-05` / `M3-05-N9` and the current-source runtime milestones.
