# ADR: v0.3 native MCP provider snapshot offline adapter

## Status

Verified partial slice (2026-08-01). Explicit provider snapshot files can now
reach native `lsharp_validate` without network or host fallback. Provider
authentication and lifecycle semantic verification remain incomplete.

## Context

The native MCP subset accepts review identity digest flags, while the MCP
contract also exposes `trust_store` and `review_lifecycle` snapshot paths.
Passing those paths to a native program without a deterministic adapter would
either silently ignore the identity or require an implicit provider/network
dependency. The shared Linux replay is not part of this contract and must not
be started for it.

## Decision

- Require `trust_store` and `review_lifecycle` together when either path is
  supplied.
- Read the explicit regular, non-symlink, non-empty files locally and compute
  `sha256:<lowercase-hex>` over their raw bytes. No JSON normalization or
  provider fetch is performed.
- If `review_trust_store_digest` or `review_lifecycle_digest` is also supplied,
  compare it with the computed digest and reject mismatches before native
  execution. Matching values are forwarded once.
- Reject missing, non-regular, symlink, empty, unreadable, or partial inputs as
  MCP tool errors. Explicit digest-only calls remain supported for callers that
  already own provider acquisition.
- This adapter does not verify signatures, trust roots, lifecycle ordering,
  authentication, or freshness. Those semantics stay at the external provider
  boundary until a native verifier is available.

## Evidence

- `scripts/ci/test-native-selfhost-mcp.py` covers digest forwarding, mismatch
  rejection, partial/missing/empty fail-closed behavior, and native no-fallback
  execution.
- `scripts/ci/test-native-selfhost-dev.sh` continues to verify that
  `mcp-server` delegates only to the native MCP shim.
- `python3 scripts/ci/test-native-selfhost-mcp.py`, Python compilation, shell
  syntax checks, runner contract tests, and `git diff --check` pass.

## Remaining boundary

Rust MCP full-tool parity, provider acquisition/authentication, signature and
lifecycle semantic verification, and current-source Linux runtime evidence
remain `[~]` under `EC-M3-05` / M3-05-N9.
