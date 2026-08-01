# ADR: Release smoke checksum target path boundary

- Status: Accepted (verified local contract)
- Date: 2026-08-02
- Scope: `scripts/ci/release-smoke.sh`, `scripts/ci/test-release-smoke-provider-snapshots.sh`
- Related: `M3-05-N2`, `M3-05-N7`, `EC-M3-05`

## Context

Release smoke verified each checksum by concatenating the archive root and the path supplied by
`checksums.txt`. A checksum entry containing `..` could therefore make the smoke hash a file outside the
extracted package. The archive itself could remain syntactically safe and all normal package checks could pass.

## Decision

Before checksum verification, parse each checksum target as a POSIX relative path and reject absolute paths or any
path containing a `..` segment. This keeps checksum evidence scoped to the extracted archive root without changing
archive entry, manifest, payload, provider identity, or atomic-install contracts.

## Evidence

- RED: the provider snapshot harness appended a checksum-valid `../../../outside-checksum-target.txt` entry to an
  otherwise valid primary archive; the previous release smoke accepted the external file.
- GREEN: the same fixture now fails before execution with `unsafe checksum target`, while valid primary/rollback,
  provider snapshot, official release snapshot, and identity focused harnesses pass.
- Current-source Linux replay was not started. Reproduce the blocker with `git rev-parse --verify HEAD` and
  `find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'`; no current-source
  matching manifest or expected hostgen lock was available at audit time.

## Boundary

This closes only release-smoke checksum target containment. It does not prove live provider/auth acquisition,
current-source Linux runtime, packaged bytes parity across targets, rollback runtime parity, or persistent-I/O
recovery. The related v0.3 items remain `[~]` in `TODO.md`.
