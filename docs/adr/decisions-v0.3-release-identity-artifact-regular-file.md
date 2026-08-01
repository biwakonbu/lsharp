# ADR: v0.3 release identity artifact regular-file boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `verify-native-release-identity.py` の明示 `--artifact` 入力
- Related: [`decisions-v0.3-native-stage0-release-artifact-binding.md`](decisions-v0.3-native-stage0-release-artifact-binding.md)

## Context

The offline release identity verifier compared `artifact_digest` with
`Path.read_bytes()`. A symlink to bytes outside the release identity input
namespace could therefore be accepted when its target had the expected digest.
That made the digest match the bytes while leaving the artifact path provenance
ambiguous.

## Decision

The verifier now lstat-checks the explicit `--artifact` path before reading it.
Only a regular, non-symlink file is accepted; a symlink or another file type
fails closed with `artifact must be a regular non-symlink file`. The existing
digest comparison remains unchanged, and provider snapshots continue to use
their separate regular-file and non-empty checks.

## Evidence

- RED: `python3 scripts/ci/test-native-release-identity.py -k test_rejects_symlinked_release_artifact_path`
  accepted a symlink whose target bytes matched the identity digest.
- GREEN: the same focused test rejected the symlink, and
  `python3 scripts/ci/test-native-release-identity.py` passed all 31 tests.
- No network, provider acquisition, stage regeneration, or Linux VM replay was
  used for this offline path-provenance contract.

## Boundary and follow-up

This is a verifier-input provenance slice, not proof of provider API/auth
acquisition, semantic signature verification, current-source Linux runtime,
or Mac/Linux packaged and rollback bytes parity. Those boundaries remain
partial in `TODO.md` and the v0.3 planning document. The current checkout has
no matching manifest/expected replay lock, and another session owns the Lima,
QEMU, and replayd processes, so heavy replay remains intentionally unexecuted.
