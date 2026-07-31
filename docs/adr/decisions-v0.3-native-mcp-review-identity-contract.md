# ADR: v0.3 native MCP review identity contract

## Status

Verified partial slice (2026-08-01). Native MCP now publishes and enforces the
same review identity input boundary as the native `App.Cli` validator.

## Context

`lsharp_validate` accepts four core review identity values: subject digest,
source commit, artifact digest, and evaluation time. The native validator only
constructs an identity when all four are present; a partial set is invalid.
The MCP shim must not rely on a client honoring the advertised schema because
the child process boundary itself must remain fail-closed.

## Decision

- Expose `dependentRequired` for all four identity fields in `tools/list`, so
  any one of them requires the other three during client-side preflight.
- Apply the same all-or-none check in the shim before invoking `program.native`.
  Partial identity input returns an MCP tool error and leaves no native log.
- Keep `review_trust_store_digest` and `review_lifecycle_digest` optional when
  no snapshot paths are supplied; when paths are supplied, their separate
  provider pair rule remains enforced by the provider snapshot adapter.
- When the four core identity fields are supplied, compare the native report's
  `review_evidence_identity` with the caller identity plus the computed provider
  digests. Reject missing, reordered, or unequal fields before returning the MCP
  result. If a manifest was requested, apply the same comparison to its
  `review_evidence_identity`.

## Evidence

- `scripts/ci/test-native-selfhost-mcp.py` verifies the published dependency
  schema, complete identity forwarding, report/manifest projection, and
  missing/mismatched identity fail-closed behavior.
- Native MCP and runner focused tests, Python compilation, shell syntax checks,
  docs audit, and `git diff --check` pass.

## Remaining boundary

Full Rust MCP tool parity, provider authentication/signature/lifecycle
semantics, native stage0 report/runtime evidence, and current-source Linux
runtime evidence remain `[~]` under `EC-M3-05` / M3-05-N9.
