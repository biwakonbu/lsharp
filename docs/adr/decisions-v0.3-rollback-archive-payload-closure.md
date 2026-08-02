# ADR: v0.3 rollback archive payload checksum closure

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/release-smoke.sh`, `scripts/ci/test-release-smoke-provider-snapshots.sh`
- Related: `M3-05-N2`, `M3-05-N7`, `M3-05-N9`

## Context

Rollback compatibility smoke required checksums for its known executable, component, manifest, and license
files, but did not require every regular file extracted from the rollback archive to be listed in
`checksums.txt`. An unregistered payload could therefore survive archive extraction and reach the rollback
smoke despite the archive's provenance record being incomplete.

## Decision

After the branch-specific rollback/native manifest checks and required checksum-entry checks, release smoke
validates the complete extracted payload closure. Every checksum entry must be a safe, unique path to a regular
file, and every extracted regular file except `checksums.txt` must have exactly one checksum entry. Symlinks,
missing checksum targets, duplicate entries, and unlisted payload files fail before executable smoke or rollback
compatibility execution.

This extends rollback archive provenance only. It does not change the stage0 archive producer round-trip,
manifest schema, rollback anchor bytes, provider/auth acquisition, or cryptographic verification contract.

## Evidence

- RED: a rollback archive with an additional `unlisted-payload` regular file and an otherwise valid anchor was
  accepted by the previous release smoke because required checksum entries still matched.
- GREEN: the same archive is rejected with `checksums.txt missing payload coverage: unlisted-payload` before
  rollback execution. Existing missing-required-entry diagnostics remain unchanged.
- Focused fetch/archive provenance, atomic install, stage0 package, official fake two-target, release-smoke,
  shell syntax, Python compile, and diff-check gates passed.

## Boundary and follow-up

This is offline rollback payload provenance evidence only. It does not prove rollback runtime compatibility on
Mac/Linux, current-source Linux stage0 execution, live provider/auth acquisition, or native cryptographic
verification. The related M3 items remain `[~]`.
