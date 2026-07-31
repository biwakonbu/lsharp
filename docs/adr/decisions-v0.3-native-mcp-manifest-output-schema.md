# ADR: v0.3 native MCP manifest output boundary

## Status

Verified partial slice (2026-08-01). Native MCP now declares the required
top-level shape of an emitted validation manifest.

## Decision

- Require `schema_version`, `nodes`, `evidence`, and `edges` in the projected
  manifest and pin `schema_version` to integer `1`.
- Keep the nested node/evidence/edge schemas as the next parity boundary rather
  than silently treating arbitrary manifest objects as valid output.

## Evidence

`scripts/ci/test-native-selfhost-mcp.py` asserts the manifest required fields,
version const, and array types in `tools/list`. Native MCP/runner tests, Python
compilation, shell syntax checks, docs audit, and `git diff --check` pass.

## Remaining boundary

Nested intent-graph manifest validation, all Rust MCP tools, provider
authentication/signature/lifecycle semantics, and current-source Linux runtime
evidence remain `[~]` under `EC-M3-05` / M3-05-N9.
