# ADR: v0.3 native MCP manifest referential closure

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP emitted manifest referential integrity

## Context

The native MCP shim validated the shape of manifest nodes, reviews, evidence, and edges, but it
did not check whether typed IDs actually resolved inside the emitted manifest. A malformed
manifest could therefore pass postflight validation while pointing at missing or wrong-kind
graph records.

## Decision

- Reject duplicate node, evidence, and explicitly emitted review IDs.
- Require evidence subjects of kind `intent` or `claim` to resolve to a node of the same kind.
- Require graph-owned edge endpoints to resolve with the Rust validation semantics:
  `motivates`, `constrained-by`, `tested-by`, `supports`, `contradicts`, `evaluates`, and
  `invalidates` validate their node/evidence endpoints; contract, change, and an omitted review
  registry remain opaque external boundaries.
- When `reviews` is present, review endpoints must resolve to that explicit registry. When it is
  absent, review IDs remain opaque for backward compatibility.

## Evidence

- RED: duplicate IDs and missing/wrong-kind node, evidence, and review references were accepted
  by shape-only postflight validation.
- GREEN: malformed closure fixtures fail closed, all seven relation variants remain valid when
  their typed endpoints resolve, and opaque contract/omitted-review boundaries remain accepted.
- Full native MCP suite passes: 76 tests.
- `python3 -m py_compile scripts/native-selfhost-mcp.py scripts/ci/native_selfhost_mcp_manifest_tests.py scripts/ci/test-native-selfhost-mcp.py` passes.

This closes only emitted manifest referential integrity. Native stage0, provider semantics,
target runtime, and full Rust/native parity remain active `[~]` boundaries in `TODO.md`.
