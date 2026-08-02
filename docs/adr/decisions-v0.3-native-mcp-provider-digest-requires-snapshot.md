# ADR: v0.3 native MCP provider digest requires an explicit snapshot

## Status

Accepted as a verified offline/fake-harness partial slice (2026-08-02).
This does not complete live provider/auth acquisition or current-source
Mac/Linux runtime and packaged/rollback parity.

## Context

`lsharp_validate` accepts provider snapshot paths and computes their raw-byte
SHA-256 before forwarding the digests to native validation. The input surface
also exposed `review_trust_store_digest` and `review_lifecycle_digest`; without
the corresponding snapshot paths, digest-only input could reach native
execution and present an unproven provider context as if it were an explicit
offline snapshot.

## Decision

- A provider digest is accepted with both explicit `trust_store` and
  `review_lifecycle` snapshot paths, or when it is bound to an explicit
  verification receipt. A digest without either source of provenance is
  rejected.
- Digest-only provider context fails before the native executable is invoked,
  with `provider digest requires explicit provider snapshot files`.
- Existing regular-file, non-empty, symlink, digest-match, receipt-context,
  and live provider/auth external-boundary checks remain unchanged.
- No cryptographic verifier or network/auth client is added.

## Evidence

The fake native harness first demonstrated RED: digest-only provider input
without a receipt was accepted and the native executable ran. The GREEN
contract now rejects it without `native.log`, while explicit snapshot paths,
receipt-bound digests, and receipt lifecycle fail-closed behavior remain
covered:

```bash
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts/ci \
python3 -m unittest -v scripts/ci/test-native-selfhost-mcp.py \
  -k provider_digest_requires_explicit_snapshot_paths
```

## Boundary and follow-up

This verifies only the native MCP adapter's offline provider-input ownership.
It does not verify signature semantics, live provider/auth acquisition,
current-source Mac/Linux runtime, full Rust/native producer parity, or
packaged/rollback bytes. Those remain `[~]` in TODO/planning. The current
manifest and expected replay lock do not match HEAD, and another session owns
Lima/QEMU/replayd, so Linux replay, stage regeneration, and full build were
not run.
