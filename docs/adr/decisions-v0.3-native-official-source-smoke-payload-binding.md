# ADR: v0.3 official two-target source-smoke payload binding

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/native-official-release-local.sh`, `scripts/ci/test-native-official-release-snapshots.sh`
- Related: `M3-04-N1`, `M3-05-N8`, `M3-05-N9`

## Context

The source-smoke evidence writer binds a target smoke run to the fetched stage0 directory with
`stage0_payload_sha256`. The official two-target orchestrator passed the fetched directories to Mac and Linux
smoke, but its postflight only compared the optional review-attestation report. A target smoke producer could
therefore emit a digest for another package and still pass the official evidence postflight.

## Decision

`native-official-release-local.sh` now recomputes the deterministic payload digest from
`${SMOKE_ROOT}/stage0-${target}` after each target smoke. It rejects an unavailable or symlinked fetched directory,
nested symlinks, an empty regular-file set, or a missing/mismatched `stage0_payload_sha256` before the target
smoke projection is accepted. The record format remains the existing source-smoke contract: sorted POSIX-relative
path, byte size, and file SHA-256 for every regular file.

This is an orchestrator cross-check of the existing digest; it does not change the stage0 manifest schema or
duplicate fetch/archive checksum validation.

## Evidence

- RED: the fake two-target harness emitted an intentionally wrong Linux payload digest while the previous
  official postflight reported success; the added contract assertion failed.
- GREEN: `bash scripts/ci/test-native-official-release-snapshots.sh` accepts matching Mac/Linux digests and rejects
  the wrong Linux digest with `source smoke evidence stage0_payload_sha256 mismatch`.
- The validation is fake/offline. The current-source manifest/expected replay lock is not available for the current
  checkout and Lima/QEMU/replayd are owned by another session, so no Linux replay, stage regeneration, or full build
  was started.

## Boundary and follow-up

This closes only the official fetched-stage0-to-target-evidence digest cross-check. It does not prove actual Mac or
Linux runtime execution, live provider/auth acquisition, native cryptographic verification, packaged bytes parity, or
rollback runtime parity. The related M3 items remain `[~]`.
