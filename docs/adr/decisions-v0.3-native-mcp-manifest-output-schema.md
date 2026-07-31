# ADR: v0.3 native MCP manifest output boundary

## Status

Verified partial slice (2026-08-01). Native MCP now declares the required
top-level and nested shape of validation manifests on both input and output
schema surfaces.

## Decision

- Require `schema_version`, `nodes`, `evidence`, and `edges` in the projected
  manifest and pin `schema_version` to integer `1`.
- Close the manifest object, node, evidence, review, identity, and edge
  variants with the same identifier, enum, provenance, execution, sampling,
  and relation constraints as the Rust MCP schema.
- Reuse the same closed manifest schema for the object variant of the native
  MCP input, while retaining the non-empty JSON string variant for compatibility.

## Evidence

`scripts/ci/test-native-selfhost-mcp.py` asserts the manifest required fields,
version const, nested node/evidence/edge shapes, and closed input/output object
boundaries in `tools/list`. Native MCP/runner tests, Python compilation, shell
syntax checks, docs audit, and `git diff --check` pass.

## Remaining boundary

Runtime validation of every manifest field before invoking a native stage0,
all Rust MCP tools, provider authentication/signature/lifecycle semantics, and
current-source Linux runtime evidence remain `[~]` under `EC-M3-05` / M3-05-N9.
