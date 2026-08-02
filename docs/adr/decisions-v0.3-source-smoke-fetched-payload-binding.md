# ADR: v0.3 source-smoke fetched stage0 payload binding

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `write-native-source-smoke-evidence.py`, native source-file smoke caller and evidence harness
- Related: `M3-04-N1`, `M3-05-N2`, `M3-05-N7`, `M3-05-N9`

## Context

The fetched stage0 package already had archive and checksum validation, but source-file smoke evidence only
stored the stage0 manifest digest. That left the remaining fetched payload bytes and their relative names outside
the evidence identity, so the evidence could not prove that the smoke used the complete fetched package.

## Decision

The evidence writer now requires an explicit `--stage0-dir` alongside the manifest. It requires the directory and
the manifest to be the regular, non-symlink `manifest.json` inside that directory, rejects root or nested symlinks,
and rejects a package with no regular files. It computes `stage0_payload_sha256` from sorted records containing each
regular file's POSIX-relative path, byte size, and SHA-256, then stores that digest beside
`stage0_manifest_sha256`. The native source-file smoke caller passes the fetched stage0 directory explicitly.

This binds all regular fetched payload bytes and names without changing the stage0 manifest schema or reimplementing
fetch/archive checksum validation. The digest is deterministic and can be independently recomputed from the captured
stage0 directory.

## Evidence

- RED: `bash scripts/ci/test-native-selfhost-source-file-smoke-evidence.sh` failed because the existing writer did
  not accept the required `--stage0-dir` input.
- GREEN: the same harness verifies the independently recomputed payload digest, preserves the manifest digest and
  evidence payload, and rejects work-directory and stage0 root/nested symlinks, missing reports, manifest path
  mismatch, and evidence-directory overwrite.
- Focused source-file smoke evidence remains offline. The current-source manifest/expected replay lock is not
  available for the current checkout, and the Lima/QEMU/replayd processes are owned by another session, so no Linux
  replay, stage regeneration, or full build was started.

## Boundary and follow-up

This closes only the fetched-package-to-source-smoke-evidence payload binding. It does not prove live provider/auth
acquisition, native cryptographic verification, current-source Linux runtime, Mac/Linux packaged bytes parity, or
rollback runtime parity. The related M3 items remain `[~]`.
