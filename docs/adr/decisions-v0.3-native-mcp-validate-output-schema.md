# ADR: v0.3 native MCP validate report schema

## Status

Verified partial slice (2026-08-01). Native MCP publishes the core Rust
`lsharp_validate` report shape for trace gaps and review provenance.

## Decision

- Mark the top-level report closed-world and preserve the seven required
  counters/status fields.
- Describe trace-gap codes and non-empty subject IDs, bounded unsigned counters,
  nullable provider digests in `review_evidence_identity`, and review state
  values in `review_verifications`.
- Keep the emitted manifest explicitly present as an object while retaining the
  native subset's existing manifest projection boundary.

## Evidence

`scripts/ci/test-native-selfhost-mcp.py` asserts the report's closed-world flag,
trace-gap enum, identity required fields, nullable digest types, verification
state enum, and manifest property. Python compilation, runner tests, shell
syntax checks, docs audit, and `git diff --check` pass.

## Remaining boundary

Full intent-graph manifest schema parity, all Rust MCP tools, provider
authentication/signature/lifecycle semantics, and current-source Linux runtime
evidence remain `[~]` under `EC-M3-05` / M3-05-N9.
